use clap::Args;
use lineark_sdk::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::output::{self, Format};

/// Manage file embeds (download/upload).
#[derive(Debug, Args)]
pub struct EmbedsCmd {
    #[command(subcommand)]
    pub action: EmbedsAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum EmbedsAction {
    /// Download a file from a URL (handles Linear's signed/expiring URLs).
    ///
    /// Examples:
    ///   lineark embeds download "<https://uploads.linear.app/...>" --output ./file.png
    Download {
        /// URL of the file to download.
        url: String,
        /// Output file path (defaults to filename from URL).
        #[arg(long)]
        output: Option<PathBuf>,
        /// Overwrite existing files.
        #[arg(long, default_value = "false")]
        overwrite: bool,
    },
    /// Upload a file to Linear's cloud storage and return the asset URL.
    ///
    /// The upload is a two-step process handled by the SDK:
    /// 1. Request a signed upload URL from Linear's API
    /// 2. PUT the file to the signed URL
    ///
    /// Examples:
    ///   lineark embeds upload ./screenshot.png
    ///   lineark embeds upload ./report.pdf --public
    Upload {
        /// Path to the file to upload.
        file: PathBuf,
        /// Make the uploaded file publicly accessible.
        #[arg(long, default_value = "false")]
        public: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadOutput {
    asset_url: String,
    filename: String,
    size: u64,
}

pub async fn run(cmd: EmbedsCmd, client: &Client, format: Format) -> anyhow::Result<()> {
    match cmd.action {
        EmbedsAction::Download {
            url,
            output,
            overwrite,
        } => {
            let output_path = match output {
                Some(p) => p,
                None => {
                    // Extract filename from URL (before query params).
                    let url_path = url.split('?').next().unwrap_or(&url);
                    let filename = url_path.rsplit('/').next().unwrap_or("download");
                    let filename = if filename.is_empty() {
                        "download"
                    } else {
                        filename
                    };
                    let filename =
                        percent_encoding::percent_decode_str(filename).decode_utf8_lossy();
                    PathBuf::from(filename.as_ref())
                }
            };

            if output_path.exists() && !overwrite {
                return Err(anyhow::anyhow!(
                    "File '{}' already exists. Use --overwrite to replace it.",
                    output_path.display()
                ));
            }

            let result = client
                .download_url(&url)
                .await
                .map_err(|e| anyhow::anyhow!("Download failed: {}", e))?;

            tokio::fs::write(&output_path, &result.bytes)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to write file: {}", e))?;

            let info = serde_json::json!({
                "path": output_path.display().to_string(),
                "size": result.bytes.len(),
                "contentType": result.content_type,
            });
            output::print_one(&info, format);
        }
        EmbedsAction::Upload { file, public } => {
            if !file.exists() {
                return Err(anyhow::anyhow!("File '{}' not found", file.display()));
            }

            let file_bytes = tokio::fs::read(&file)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to read file: {}", e))?;

            let filename = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("upload")
                .to_string();

            let content_type = mime_from_extension(&file);
            let file_size = file_bytes.len() as u64;

            let result = client
                .upload_file(&filename, &content_type, file_bytes, public)
                .await
                .map_err(|e| anyhow::anyhow!("Upload failed: {}", e))?;

            let info = UploadOutput {
                asset_url: result.asset_url,
                filename,
                size: file_size,
            };
            output::print_one(&info, format);
        }
    }
    Ok(())
}

/// Guess MIME type from file extension.
fn mime_from_extension(path: &std::path::Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "json" => "application/json",
        "xml" => "application/xml",
        "csv" => "text/csv",
        "txt" | "md" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        _ => "application/octet-stream",
    }
    .to_string()
}
