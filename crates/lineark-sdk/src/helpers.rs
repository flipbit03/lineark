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
        let response = self.http().get(url).send().await?;

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
        let payload = self
            .file_upload(
                None,
                if make_public { Some(true) } else { None },
                size,
                content_type.to_string(),
                filename.to_string(),
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
