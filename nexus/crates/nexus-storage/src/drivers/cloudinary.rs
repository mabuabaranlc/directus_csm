use crate::{FileStat, ReadOptions, StorageDriver, StorageError};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use nexus_env::env_string;
use serde::Deserialize;
use std::pin::Pin;
use tokio::io::AsyncRead;
use tracing::warn;

const CLOUDINARY_API_BASE: &str = "https://api.cloudinary.com/v1_1";
const CLOUDINARY_RES_BASE: &str = "https://res.cloudinary.com";

/// Response from the Cloudinary Admin API resource detail endpoint.
#[derive(Debug, Deserialize)]
struct CloudinaryResourceDetail {
    #[serde(default)]
    bytes: u64,
    #[serde(default)]
    created_at: String,
    #[serde(default)]
    format: Option<String>,
}

/// Response from the Cloudinary Admin API resources list endpoint.
#[derive(Debug, Deserialize)]
struct CloudinaryListResponse {
    #[serde(default)]
    resources: Vec<CloudinaryListResource>,
    next_cursor: Option<String>,
}

/// A single resource in a Cloudinary list response.
#[derive(Debug, Deserialize)]
struct CloudinaryListResource {
    public_id: String,
}

/// Cloudinary storage driver using the Cloudinary REST API.
///
/// Uses HTTP Basic Auth (api_key:api_secret) for all Admin API operations and
/// public delivery URLs for reading files. All assets are stored with
/// `resource_type = raw` to handle arbitrary file types.
pub struct CloudinaryDriver {
    client: reqwest::Client,
    cloud_name: String,
    api_key: String,
    api_secret: String,
}

impl CloudinaryDriver {
    /// Create a new CloudinaryDriver from environment variables.
    ///
    /// Required env vars:
    ///   - `STORAGE_CLOUDINARY_CLOUD_NAME`
    ///   - `STORAGE_CLOUDINARY_API_KEY`
    ///   - `STORAGE_CLOUDINARY_API_SECRET`
    pub fn new_from_env() -> Result<Self, StorageError> {
        let cloud_name = env_string("STORAGE_CLOUDINARY_CLOUD_NAME").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_CLOUDINARY_CLOUD_NAME environment variable is required".to_string(),
            )
        })?;

        let api_key = env_string("STORAGE_CLOUDINARY_API_KEY").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_CLOUDINARY_API_KEY environment variable is required".to_string(),
            )
        })?;

        let api_secret = env_string("STORAGE_CLOUDINARY_API_SECRET").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_CLOUDINARY_API_SECRET environment variable is required".to_string(),
            )
        })?;

        let client = reqwest::Client::new();

        Ok(Self {
            client,
            cloud_name,
            api_key,
            api_secret,
        })
    }

    /// Create a CloudinaryDriver with explicit configuration.
    pub fn new(
        client: reqwest::Client,
        cloud_name: String,
        api_key: String,
        api_secret: String,
    ) -> Self {
        Self {
            client,
            cloud_name,
            api_key,
            api_secret,
        }
    }

    /// Normalize path: strip leading slash for public_id consistency.
    fn normalize_key(path: &str) -> &str {
        path.strip_prefix('/').unwrap_or(path)
    }

    /// Build the public delivery URL for a raw asset.
    fn delivery_url(&self, public_id: &str) -> String {
        format!(
            "{}/{}/raw/upload/{}",
            CLOUDINARY_RES_BASE, self.cloud_name, public_id
        )
    }

    /// Build an Admin API URL for the given path segments.
    fn admin_url(&self, path: &str) -> String {
        format!("{}/{}/{}", CLOUDINARY_API_BASE, self.cloud_name, path)
    }

    /// Map an HTTP response status to an appropriate StorageError.
    fn map_response_error(status: reqwest::StatusCode, path: &str, body: &str) -> StorageError {
        if status == reqwest::StatusCode::NOT_FOUND {
            StorageError::NotFound(path.to_string())
        } else if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::UNAUTHORIZED
        {
            StorageError::PermissionDenied(path.to_string())
        } else {
            StorageError::Backend(format!(
                "Cloudinary request failed ({}): {}",
                status, body
            ))
        }
    }
}

#[async_trait]
impl StorageDriver for CloudinaryDriver {
    async fn read(
        &self,
        path: &str,
        options: Option<ReadOptions>,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>, StorageError> {
        let key = Self::normalize_key(path);
        let url = self.delivery_url(key);

        let mut request = self.client.get(&url);

        // Apply byte range if provided.
        if let Some(ref opts) = options {
            if let Some((start, end)) = opts.range {
                let range_str = match end {
                    Some(e) => format!("bytes={}-{}", start, e),
                    None => format!("bytes={}-", start),
                };
                request = request.header(reqwest::header::RANGE, range_str);
            }
        }

        let resp = request.send().await.map_err(|e| {
            StorageError::Backend(format!("Cloudinary read request failed: {}", e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, path, &body));
        }

        let bytes = resp.bytes().await.map_err(|e| {
            StorageError::Backend(format!("Failed to read Cloudinary response body: {}", e))
        })?;

        Ok(Box::pin(std::io::Cursor::new(bytes)))
    }

    async fn write(
        &self,
        path: &str,
        mut content: Pin<Box<dyn AsyncRead + Send>>,
        _content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let key = Self::normalize_key(path);

        // Read the entire content into memory for the upload.
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut content, &mut buf)
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to read content: {}", e)))?;

        let url = self.admin_url("raw/upload");

        // Use the Admin API upload endpoint with HTTP Basic Auth.
        // This avoids needing to compute SHA-1 signatures.
        let form = reqwest::multipart::Form::new()
            .text("public_id", key.to_string())
            .text("overwrite", "true")
            .text("resource_type", "raw")
            .part(
                "file",
                reqwest::multipart::Part::bytes(buf)
                    .file_name(key.to_string())
                    .mime_str("application/octet-stream")
                    .map_err(|e| {
                        StorageError::Backend(format!("Failed to set MIME type: {}", e))
                    })?,
            );

        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Cloudinary upload request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, path, &body));
        }

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let key = Self::normalize_key(path);

        // Use the Admin API to delete by public_ids.
        let url = self.admin_url("resources/raw/upload");

        let body = serde_json::json!({
            "public_ids": [key],
        });

        let resp = self
            .client
            .delete(&url)
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Cloudinary delete request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, path, &body));
        }

        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<FileStat, StorageError> {
        let key = Self::normalize_key(path);
        let encoded = urlencoding::encode(key);

        let url = self.admin_url(&format!("resources/raw/upload/{}", encoded));

        let resp = self
            .client
            .get(&url)
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Cloudinary stat request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, path, &body));
        }

        let detail: CloudinaryResourceDetail = resp.json().await.map_err(|e| {
            StorageError::Backend(format!(
                "Failed to parse Cloudinary resource metadata: {}",
                e
            ))
        })?;

        let modified = DateTime::parse_from_rfc3339(&detail.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|e| {
                warn!(
                    "Failed to parse Cloudinary resource created_at timestamp: {}",
                    e
                );
                Utc::now()
            });

        // Cloudinary raw resources typically do not carry a meaningful content
        // type. If the API returns a format field we can try to map common
        // extensions, but for raw uploads this is usually absent.
        let content_type = detail.format.and_then(|f| guess_content_type(&f));

        Ok(FileStat {
            size: detail.bytes,
            modified,
            content_type,
        })
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        match self.stat(path).await {
            Ok(_) => Ok(true),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn mv(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        let src_key = Self::normalize_key(src);
        let dest_key = Self::normalize_key(dest);

        let url = self.admin_url("raw/rename");

        let form = reqwest::multipart::Form::new()
            .text("from_public_id", src_key.to_string())
            .text("to_public_id", dest_key.to_string());

        let resp = self
            .client
            .post(&url)
            .basic_auth(&self.api_key, Some(&self.api_secret))
            .multipart(form)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Cloudinary rename request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, src, &body));
        }

        Ok(())
    }

    async fn copy(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        // Cloudinary does not provide a native copy operation for raw resources.
        // We read the source file and re-upload it with the destination public_id.
        let data = self.read(src, None).await?;
        self.write(dest, data, None).await?;
        Ok(())
    }

    fn list(
        &self,
        prefix: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, StorageError>> + Send>> {
        let client = self.client.clone();
        let cloud_name = self.cloud_name.clone();
        let api_key = self.api_key.clone();
        let api_secret = self.api_secret.clone();
        let prefix = prefix.map(|p| Self::normalize_key(p).to_string());

        Box::pin(async_stream::try_stream! {
            let mut next_cursor: Option<String> = None;

            loop {
                let mut url = format!(
                    "{}/{}/resources/raw?max_results=500",
                    CLOUDINARY_API_BASE, cloud_name
                );

                if let Some(ref p) = prefix {
                    url.push_str(&format!("&prefix={}", urlencoding::encode(p)));
                }

                if let Some(ref cursor) = next_cursor {
                    url.push_str(&format!("&next_cursor={}", urlencoding::encode(cursor)));
                }

                let resp = client
                    .get(&url)
                    .basic_auth(&api_key, Some(&api_secret))
                    .send()
                    .await
                    .map_err(|e| {
                        StorageError::Backend(format!("Cloudinary list request failed: {}", e))
                    })?;

                let status = resp.status();
                let list_resp: CloudinaryListResponse = if status.is_success() {
                    resp.json().await.map_err(|e| {
                        StorageError::Backend(format!(
                            "Failed to parse Cloudinary list response: {}",
                            e
                        ))
                    })?
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    Err(StorageError::Backend(format!(
                        "Cloudinary list failed ({}): {}",
                        status, body
                    )))?
                };

                for resource in list_resp.resources {
                    yield resource.public_id;
                }

                match list_resp.next_cursor {
                    Some(cursor) => next_cursor = Some(cursor),
                    None => break,
                }
            }
        })
    }
}

/// Map a file extension (without the leading dot) to a MIME content type string.
///
/// This mirrors the helper used by `LocalDriver` for guessing MIME types from
/// file extensions and is used when Cloudinary returns a `format` field in its
/// resource metadata.
fn guess_content_type(ext: &str) -> Option<String> {
    let mime = match ext.to_lowercase().as_str() {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "bmp" => "image/bmp",
        "tiff" | "tif" => "image/tiff",
        "avif" => "image/avif",
        "pdf" => "application/pdf",
        "json" => "application/json",
        "xml" => "application/xml",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "ts" => "application/typescript",
        "txt" => "text/plain",
        "csv" => "text/csv",
        "md" => "text/markdown",
        "yaml" | "yml" => "application/x-yaml",
        "toml" => "application/toml",
        "zip" => "application/zip",
        "gz" | "gzip" => "application/gzip",
        "tar" => "application/x-tar",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "avi" => "video/x-msvideo",
        "mov" => "video/quicktime",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "eot" => "application/vnd.ms-fontobject",
        "wasm" => "application/wasm",
        _ => return None,
    };
    Some(mime.to_string())
}
