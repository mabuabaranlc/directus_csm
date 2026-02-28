use crate::{FileStat, ReadOptions, StorageDriver, StorageError};
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use futures::Stream;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use nexus_env::env_string;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncRead;
use tokio::sync::RwLock;
use tracing::{debug, warn};

const GCS_BASE_URL: &str = "https://storage.googleapis.com/storage/v1";
const GCS_UPLOAD_URL: &str = "https://storage.googleapis.com/upload/storage/v1";
const GCS_SCOPE: &str = "https://www.googleapis.com/auth/devstorage.full_control";

/// Cached OAuth2 access token with expiry.
#[derive(Debug, Clone)]
struct GcsToken {
    token: String,
    expires_at: DateTime<Utc>,
}

/// Service account credentials parsed from the JSON key file.
#[derive(Debug, Clone, Deserialize)]
pub struct ServiceAccountCredentials {
    client_email: String,
    private_key: String,
    token_uri: String,
}

/// JWT claims for the Google OAuth2 service account flow.
#[derive(Debug, Serialize)]
struct GcsJwtClaims {
    iss: String,
    scope: String,
    aud: String,
    iat: i64,
    exp: i64,
}

/// Response from the Google OAuth2 token endpoint.
#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

/// GCS object metadata returned by the Objects.get API.
#[derive(Debug, Deserialize)]
struct GcsObjectMetadata {
    #[serde(default)]
    size: String,
    #[serde(default)]
    updated: String,
    #[serde(rename = "contentType", default)]
    content_type: Option<String>,
}

/// GCS Objects.list response.
#[derive(Debug, Deserialize)]
struct GcsListResponse {
    #[serde(default)]
    items: Option<Vec<GcsListItem>>,
    #[serde(rename = "nextPageToken")]
    next_page_token: Option<String>,
}

/// A single item in a GCS Objects.list response.
#[derive(Debug, Deserialize)]
struct GcsListItem {
    name: String,
}

/// Google Cloud Storage driver using the GCS JSON API v1.
///
/// Authenticates via service account JWT flow and caches the access token.
pub struct GcsDriver {
    client: reqwest::Client,
    bucket: String,
    credentials: ServiceAccountCredentials,
    access_token: Arc<RwLock<GcsToken>>,
}

impl GcsDriver {
    /// Create a new GcsDriver from environment variables.
    ///
    /// Required env vars:
    ///   - `STORAGE_GCS_BUCKET`
    ///
    /// Optional env vars:
    ///   - `STORAGE_GCS_CREDENTIALS` — path to a service account JSON key file
    ///     (falls back to `GOOGLE_APPLICATION_CREDENTIALS`)
    pub async fn new_from_env() -> Result<Self, StorageError> {
        let bucket = env_string("STORAGE_GCS_BUCKET").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_GCS_BUCKET environment variable is required".to_string(),
            )
        })?;

        let credentials_path = env_string("STORAGE_GCS_CREDENTIALS")
            .or_else(|| env_string("GOOGLE_APPLICATION_CREDENTIALS"))
            .ok_or_else(|| {
                StorageError::Backend(
                    "STORAGE_GCS_CREDENTIALS or GOOGLE_APPLICATION_CREDENTIALS environment variable is required".to_string(),
                )
            })?;

        let credentials_json = tokio::fs::read_to_string(&credentials_path)
            .await
            .map_err(|e| {
                StorageError::Backend(format!(
                    "Failed to read service account credentials from {}: {}",
                    credentials_path, e
                ))
            })?;

        let credentials: ServiceAccountCredentials =
            serde_json::from_str(&credentials_json).map_err(|e| {
                StorageError::Backend(format!(
                    "Failed to parse service account credentials: {}",
                    e
                ))
            })?;

        let client = reqwest::Client::new();

        // Obtain an initial access token.
        let token = Self::fetch_access_token(&client, &credentials).await?;

        Ok(Self {
            client,
            bucket,
            credentials,
            access_token: Arc::new(RwLock::new(token)),
        })
    }

    /// Create a GcsDriver with explicit configuration.
    pub async fn new(
        client: reqwest::Client,
        bucket: String,
        credentials: ServiceAccountCredentials,
    ) -> Result<Self, StorageError> {
        let token = Self::fetch_access_token(&client, &credentials).await?;

        Ok(Self {
            client,
            bucket,
            credentials,
            access_token: Arc::new(RwLock::new(token)),
        })
    }

    /// Normalize path: strip leading slash for object key consistency.
    fn normalize_key(path: &str) -> &str {
        path.strip_prefix('/').unwrap_or(path)
    }

    /// URL-encode an object name for use in GCS API paths.
    fn encode_object(name: &str) -> String {
        urlencoding::encode(name).into_owned()
    }

    /// Get a valid access token, refreshing if near expiry.
    ///
    /// Refreshes when the token is within 60 seconds of expiration.
    async fn get_token(&self) -> Result<String, StorageError> {
        // Fast path: check under a read lock.
        {
            let token = self.access_token.read().await;
            if token.expires_at > Utc::now() + Duration::seconds(60) {
                return Ok(token.token.clone());
            }
        }

        // Slow path: acquire a write lock and refresh.
        let mut token = self.access_token.write().await;

        // Double-check after acquiring write lock (another task may have refreshed).
        if token.expires_at > Utc::now() + Duration::seconds(60) {
            return Ok(token.token.clone());
        }

        debug!("Refreshing GCS access token");
        let new_token = Self::fetch_access_token(&self.client, &self.credentials).await?;
        *token = new_token;
        Ok(token.token.clone())
    }

    /// Build and sign a JWT, then exchange it for an access token.
    async fn fetch_access_token(
        client: &reqwest::Client,
        credentials: &ServiceAccountCredentials,
    ) -> Result<GcsToken, StorageError> {
        let now = Utc::now();
        let claims = GcsJwtClaims {
            iss: credentials.client_email.clone(),
            scope: GCS_SCOPE.to_string(),
            aud: credentials.token_uri.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::hours(1)).timestamp(),
        };

        let header = Header::new(Algorithm::RS256);
        let encoding_key =
            EncodingKey::from_rsa_pem(credentials.private_key.as_bytes()).map_err(|e| {
                StorageError::Backend(format!("Failed to parse service account private key: {}", e))
            })?;

        let jwt = encode(&header, &claims, &encoding_key).map_err(|e| {
            StorageError::Backend(format!("Failed to sign JWT: {}", e))
        })?;

        let resp = client
            .post(&credentials.token_uri)
            .form(&[
                (
                    "grant_type",
                    "urn:ietf:params:oauth:grant-type:jwt-bearer",
                ),
                ("assertion", &jwt),
            ])
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("Token exchange request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::Backend(format!(
                "Token exchange failed ({}): {}",
                status, body
            )));
        }

        let token_resp: TokenResponse = resp.json().await.map_err(|e| {
            StorageError::Backend(format!("Failed to parse token response: {}", e))
        })?;

        Ok(GcsToken {
            token: token_resp.access_token,
            expires_at: now + Duration::seconds(token_resp.expires_in),
        })
    }

    /// Map an HTTP response status and body to an appropriate StorageError.
    fn map_response_error(status: reqwest::StatusCode, path: &str, body: &str) -> StorageError {
        if status == reqwest::StatusCode::NOT_FOUND {
            StorageError::NotFound(path.to_string())
        } else if status == reqwest::StatusCode::FORBIDDEN
            || status == reqwest::StatusCode::UNAUTHORIZED
        {
            StorageError::PermissionDenied(path.to_string())
        } else {
            StorageError::Backend(format!("GCS request failed ({}): {}", status, body))
        }
    }
}

#[async_trait]
impl StorageDriver for GcsDriver {
    async fn read(
        &self,
        path: &str,
        options: Option<ReadOptions>,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>, StorageError> {
        let key = Self::normalize_key(path);
        let encoded = Self::encode_object(key);
        let token = self.get_token().await?;

        let url = format!(
            "{}/b/{}/o/{}?alt=media",
            GCS_BASE_URL, self.bucket, encoded
        );

        let mut request = self
            .client
            .get(&url)
            .bearer_auth(&token);

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
            StorageError::Backend(format!("GCS read request failed: {}", e))
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, path, &body));
        }

        // Read the response body and wrap in a Cursor for AsyncRead.
        let bytes = resp.bytes().await.map_err(|e| {
            StorageError::Backend(format!("Failed to read GCS response body: {}", e))
        })?;

        Ok(Box::pin(std::io::Cursor::new(bytes)))
    }

    async fn write(
        &self,
        path: &str,
        mut content: Pin<Box<dyn AsyncRead + Send>>,
        content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let key = Self::normalize_key(path);
        let encoded = Self::encode_object(key);
        let token = self.get_token().await?;

        // Read the entire content into memory for the simple upload.
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut content, &mut buf)
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to read content: {}", e)))?;

        let ct = content_type.unwrap_or("application/octet-stream");

        let url = format!(
            "{}/b/{}/o?uploadType=media&name={}",
            GCS_UPLOAD_URL, self.bucket, encoded
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_TYPE, ct)
            .body(buf)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("GCS write request failed: {}", e))
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
        let encoded = Self::encode_object(key);
        let token = self.get_token().await?;

        let url = format!("{}/b/{}/o/{}", GCS_BASE_URL, self.bucket, encoded);

        let resp = self
            .client
            .delete(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("GCS delete request failed: {}", e))
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
        let encoded = Self::encode_object(key);
        let token = self.get_token().await?;

        let url = format!("{}/b/{}/o/{}", GCS_BASE_URL, self.bucket, encoded);

        let resp = self
            .client
            .get(&url)
            .bearer_auth(&token)
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("GCS stat request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, path, &body));
        }

        let metadata: GcsObjectMetadata = resp.json().await.map_err(|e| {
            StorageError::Backend(format!("Failed to parse GCS object metadata: {}", e))
        })?;

        let size: u64 = metadata.size.parse().unwrap_or(0);

        let modified = DateTime::parse_from_rfc3339(&metadata.updated)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_else(|e| {
                warn!("Failed to parse GCS object updated timestamp: {}", e);
                Utc::now()
            });

        Ok(FileStat {
            size,
            modified,
            content_type: metadata.content_type,
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
        self.copy(src, dest).await?;
        self.delete(src).await?;
        Ok(())
    }

    async fn copy(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        let src_key = Self::normalize_key(src);
        let dest_key = Self::normalize_key(dest);
        let src_encoded = Self::encode_object(src_key);
        let dest_encoded = Self::encode_object(dest_key);
        let token = self.get_token().await?;

        let url = format!(
            "{}/b/{}/o/{}/copyTo/b/{}/o/{}",
            GCS_BASE_URL, self.bucket, src_encoded, self.bucket, dest_encoded
        );

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&token)
            .header(reqwest::header::CONTENT_LENGTH, "0")
            .send()
            .await
            .map_err(|e| {
                StorageError::Backend(format!("GCS copy request failed: {}", e))
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(Self::map_response_error(status, src, &body));
        }

        Ok(())
    }

    fn list(
        &self,
        prefix: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, StorageError>> + Send>> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let prefix = prefix.map(|p| Self::normalize_key(p).to_string());
        let access_token = self.access_token.clone();
        let credentials = self.credentials.clone();

        Box::pin(async_stream::try_stream! {
            let mut page_token: Option<String> = None;

            loop {
                // Refresh token if needed (inline logic since we cannot call &self methods).
                let token = {
                    let needs_refresh = {
                        let t = access_token.read().await;
                        t.expires_at <= Utc::now() + Duration::seconds(60)
                    };

                    if needs_refresh {
                        let mut t = access_token.write().await;
                        // Double-check after acquiring write lock.
                        if t.expires_at <= Utc::now() + Duration::seconds(60) {
                            let new_token = GcsDriver::fetch_access_token(&client, &credentials).await?;
                            *t = new_token;
                        }
                        t.token.clone()
                    } else {
                        access_token.read().await.token.clone()
                    }
                };

                let mut url = format!("{}/b/{}/o?maxResults=1000", GCS_BASE_URL, bucket);

                if let Some(ref p) = prefix {
                    url.push_str(&format!("&prefix={}", urlencoding::encode(p)));
                }

                if let Some(ref tok) = page_token {
                    url.push_str(&format!("&pageToken={}", urlencoding::encode(tok)));
                }

                let resp = client
                    .get(&url)
                    .bearer_auth(&token)
                    .send()
                    .await
                    .map_err(|e| {
                        StorageError::Backend(format!("GCS list request failed: {}", e))
                    })?;

                let status = resp.status();
                let list_resp: GcsListResponse = if status.is_success() {
                    resp.json().await.map_err(|e| {
                        StorageError::Backend(format!("Failed to parse GCS list response: {}", e))
                    })?
                } else {
                    let body = resp.text().await.unwrap_or_default();
                    Err(StorageError::Backend(format!(
                        "GCS list failed ({}): {}",
                        status, body
                    )))?
                };

                if let Some(items) = list_resp.items {
                    for item in items {
                        yield item.name;
                    }
                }

                match list_resp.next_page_token {
                    Some(tok) => page_token = Some(tok),
                    None => break,
                }
            }
        })
    }
}
