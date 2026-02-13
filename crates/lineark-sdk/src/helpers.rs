//! HTTP helpers for Linear file operations.
//!
//! Linear's file handling works outside the GraphQL API: uploads go to Google
//! Cloud Storage via signed URLs, and downloads fetch from Linear's CDN. These
//! helpers use the SDK's internal HTTP client so consumers don't need a separate
//! `reqwest` dependency.

use crate::client::Client;
use crate::error::LinearError;

/// Metadata about a successfully downloaded file.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    /// The raw file bytes.
    pub bytes: Vec<u8>,
    /// Content-Type header from the response, if present.
    pub content_type: Option<String>,
}

/// Metadata returned after a successful two-step file upload.
#[derive(Debug, Clone)]
pub struct UploadResult {
    /// The permanent asset URL for referencing this file in comments, descriptions, etc.
    pub asset_url: String,
}

impl Client {
    /// Download a file from a URL.
    ///
    /// Handles Linear's signed/expiring CDN URLs (e.g. `https://uploads.linear.app/...`)
    /// as well as any other publicly accessible URL. Returns the raw bytes and
    /// content type so the caller can write them to disk or process them further.
    ///
    /// # Errors
    ///
    /// Returns [`LinearError::HttpError`] if the server responds with a non-2xx status,
    /// or [`LinearError::Network`] if the request fails at the transport level.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), lineark_sdk::LinearError> {
    /// let client = lineark_sdk::Client::auto()?;
    /// let result = client.download_url("https://uploads.linear.app/...").await?;
    /// std::fs::write("output.png", &result.bytes).unwrap();
    /// # Ok(())
    /// # }
    /// ```
    pub async fn download_url(&self, url: &str) -> Result<DownloadResult, LinearError> {
        let response = self
            .http()
            .get(url)
            .header("Authorization", self.token())
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(LinearError::HttpError {
                status: status.as_u16(),
                body,
            });
        }

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let bytes = response.bytes().await?.to_vec();

        Ok(DownloadResult {
            bytes,
            content_type,
        })
    }

    /// Upload a file to Linear's cloud storage.
    ///
    /// This is a two-step process:
    /// 1. Call the [`fileUpload`](Client::file_upload) GraphQL mutation to obtain
    ///    a signed upload URL and required headers from Linear.
    /// 2. `PUT` the raw file bytes to that signed URL (a Google Cloud Storage endpoint).
    ///
    /// On success, returns an [`UploadResult`] containing the permanent `asset_url`
    /// that can be referenced in issue descriptions, comments, or attachments.
    ///
    /// # Arguments
    ///
    /// * `filename` — The original filename (e.g. `"screenshot.png"`). Linear uses this
    ///   for display and content-type inference on its side.
    /// * `content_type` — MIME type of the file (e.g. `"image/png"`).
    /// * `bytes` — The raw file content.
    /// * `make_public` — If `true`, the uploaded file will be publicly accessible
    ///   without authentication.
    ///
    /// # Errors
    ///
    /// Returns an error if the `fileUpload` mutation fails, if the signed URL
    /// upload fails, or if the response is missing expected fields.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # async fn example() -> Result<(), lineark_sdk::LinearError> {
    /// let client = lineark_sdk::Client::auto()?;
    /// let bytes = std::fs::read("screenshot.png").unwrap();
    /// let result = client
    ///     .upload_file("screenshot.png", "image/png", bytes, false)
    ///     .await?;
    /// println!("Uploaded to: {}", result.asset_url);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn upload_file(
        &self,
        filename: &str,
        content_type: &str,
        bytes: Vec<u8>,
        make_public: bool,
    ) -> Result<UploadResult, LinearError> {
        let size = bytes.len() as i64;

        // Step 1: Request a signed upload URL from Linear's API.
        // We use a custom query instead of the generated `file_upload` method
        // because we need the nested `headers { key value }` field which the
        // codegen omits (it only includes scalar fields).
        let variables = serde_json::json!({
            "metaData": null,
            "makePublic": if make_public { Some(true) } else { None::<bool> },
            "size": size,
            "contentType": content_type,
            "filename": filename,
        });
        let payload = self
            .execute::<serde_json::Value>(
                "mutation FileUpload($metaData: JSON, $makePublic: Boolean, $size: Int!, \
                 $contentType: String!, $filename: String!) { \
                 fileUpload(metaData: $metaData, makePublic: $makePublic, size: $size, \
                 contentType: $contentType, filename: $filename) { \
                 success uploadFile { filename contentType size uploadUrl assetUrl \
                 headers { key value } } } }",
                variables,
                "fileUpload",
            )
            .await?;

        if payload.get("success").and_then(|v| v.as_bool()) != Some(true) {
            return Err(LinearError::MissingData(format!(
                "fileUpload mutation failed: {}",
                serde_json::to_string(&payload).unwrap_or_default()
            )));
        }

        let upload_file = payload.get("uploadFile").ok_or_else(|| {
            LinearError::MissingData("No 'uploadFile' in fileUpload response".to_string())
        })?;

        let upload_url = upload_file
            .get("uploadUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                LinearError::MissingData("No 'uploadUrl' in fileUpload response".to_string())
            })?;

        let asset_url = upload_file
            .get("assetUrl")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                LinearError::MissingData("No 'assetUrl' in fileUpload response".to_string())
            })?
            .to_string();

        // Collect upload headers prescribed by Linear.
        let headers: Vec<(String, String)> = upload_file
            .get("headers")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|h| {
                        let key = h.get("key")?.as_str()?.to_string();
                        let val = h.get("value")?.as_str()?.to_string();
                        Some((key, val))
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Step 2: PUT the file bytes to the signed upload URL.
        let mut request = self
            .http()
            .put(upload_url)
            .header("Content-Type", content_type)
            .body(bytes);

        for (key, value) in &headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let response = request.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LinearError::HttpError {
                status: status.as_u16(),
                body,
            });
        }

        Ok(UploadResult { asset_url })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_client_with_base(base_url: &str) -> Client {
        let mut client = Client::from_token("test-token").unwrap();
        client.set_base_url(base_url.to_string());
        client
    }

    // ── download_url ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn download_url_returns_bytes_and_content_type() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/files/test.png"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_bytes(vec![0x89, 0x50, 0x4E, 0x47]) // PNG magic bytes
                    .insert_header("content-type", "image/png"),
            )
            .mount(&server)
            .await;

        let client = test_client_with_base(&server.uri());
        let url = format!("{}/files/test.png", server.uri());
        let result = client.download_url(&url).await.unwrap();

        assert_eq!(result.bytes, vec![0x89, 0x50, 0x4E, 0x47]);
        assert_eq!(result.content_type, Some("image/png".to_string()));
    }

    #[tokio::test]
    async fn download_url_without_content_type_header() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/files/raw"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"raw data".to_vec()))
            .mount(&server)
            .await;

        let client = test_client_with_base(&server.uri());
        let url = format!("{}/files/raw", server.uri());
        let result = client.download_url(&url).await.unwrap();

        assert_eq!(result.bytes, b"raw data");
        assert_eq!(result.content_type, None);
    }

    #[tokio::test]
    async fn download_url_404_returns_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/files/missing"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let client = test_client_with_base(&server.uri());
        let url = format!("{}/files/missing", server.uri());
        let result = client.download_url(&url).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LinearError::HttpError { status, body } => {
                assert_eq!(status, 404);
                assert_eq!(body, "Not Found");
            }
            other => panic!("expected HttpError, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn download_url_500_returns_http_error() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/files/error"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&server)
            .await;

        let client = test_client_with_base(&server.uri());
        let url = format!("{}/files/error", server.uri());
        let result = client.download_url(&url).await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LinearError::HttpError { status, .. } => assert_eq!(status, 500),
            other => panic!("expected HttpError, got: {:?}", other),
        }
    }

    // ── upload_file ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn upload_file_two_step_flow() {
        let server = MockServer::start().await;
        let upload_url = format!("{}/upload-target", server.uri());
        let asset_url = "https://linear-uploads.example.com/asset/test.png";

        // Step 1: Mock the fileUpload GraphQL mutation.
        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "fileUpload": {
                        "success": true,
                        "uploadFile": {
                            "uploadUrl": upload_url,
                            "assetUrl": asset_url,
                            "filename": "test.png",
                            "contentType": "image/png",
                            "size": 4,
                            "headers": [
                                { "key": "x-goog-meta-test", "value": "123" }
                            ]
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        // Step 2: Mock the PUT to the signed upload URL.
        Mock::given(method("PUT"))
            .and(path("/upload-target"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&server)
            .await;

        let mut client = Client::from_token("test-token").unwrap();
        // Point GraphQL calls at the mock server.
        client.set_base_url(server.uri());

        let bytes = vec![0x89, 0x50, 0x4E, 0x47]; // PNG magic
        let result = client
            .upload_file("test.png", "image/png", bytes, false)
            .await
            .unwrap();

        assert_eq!(result.asset_url, asset_url);

        // Verify both requests were made.
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests.len(),
            2,
            "should have made 2 requests (mutation + PUT)"
        );
        assert_eq!(requests[0].method.as_str(), "POST"); // GraphQL mutation
        assert_eq!(requests[1].method.as_str(), "PUT"); // File upload
    }

    #[tokio::test]
    async fn upload_file_mutation_failure_returns_error() {
        let server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "fileUpload": {
                        "success": false
                    }
                }
            })))
            .mount(&server)
            .await;

        let mut client = Client::from_token("test-token").unwrap();
        client.set_base_url(server.uri());

        let result = client
            .upload_file("test.png", "image/png", vec![1, 2, 3], false)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LinearError::MissingData(msg) => {
                assert!(msg.contains("fileUpload mutation failed"), "got: {msg}");
            }
            other => panic!("expected MissingData, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn upload_file_put_failure_returns_http_error() {
        let server = MockServer::start().await;
        let upload_url = format!("{}/upload-target", server.uri());

        Mock::given(method("POST"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "fileUpload": {
                        "success": true,
                        "uploadFile": {
                            "uploadUrl": upload_url,
                            "assetUrl": "https://example.com/asset.png",
                            "headers": []
                        }
                    }
                }
            })))
            .mount(&server)
            .await;

        Mock::given(method("PUT"))
            .and(path("/upload-target"))
            .respond_with(ResponseTemplate::new(403).set_body_string("Forbidden"))
            .mount(&server)
            .await;

        let mut client = Client::from_token("test-token").unwrap();
        client.set_base_url(server.uri());

        let result = client
            .upload_file("test.png", "image/png", vec![1, 2, 3], false)
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            LinearError::HttpError { status, body } => {
                assert_eq!(status, 403);
                assert_eq!(body, "Forbidden");
            }
            other => panic!("expected HttpError, got: {:?}", other),
        }
    }
}
