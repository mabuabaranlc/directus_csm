use crate::{FileStat, ReadOptions, StorageDriver, StorageError};
use async_trait::async_trait;
use aws_sdk_s3::{
    config::{BehaviorVersion, Credentials, Region},
    primitives::ByteStream,
    Client,
};
use chrono::{DateTime, Utc};
use futures::Stream;
use nexus_env::{env_string, env_string_or};
use std::pin::Pin;
use tokio::io::AsyncRead;

pub struct S3Driver {
    client: Client,
    bucket: String,
}

impl S3Driver {
    /// Create a new S3Driver from environment variables.
    ///
    /// Required env vars:
    ///   - `STORAGE_S3_BUCKET`
    ///
    /// Optional env vars:
    ///   - `STORAGE_S3_REGION` (default: "us-east-1")
    ///   - `STORAGE_S3_KEY` + `STORAGE_S3_SECRET` (omit for role-based auth)
    pub fn new_from_env() -> Result<Self, StorageError> {
        let bucket = env_string("STORAGE_S3_BUCKET").ok_or_else(|| {
            StorageError::Backend("STORAGE_S3_BUCKET environment variable is required".to_string())
        })?;

        let region = env_string_or("STORAGE_S3_REGION", "us-east-1");
        let access_key = env_string("STORAGE_S3_KEY");
        let secret_key = env_string("STORAGE_S3_SECRET");

        let mut config_builder = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .region(Region::new(region));

        if let (Some(key), Some(secret)) = (access_key, secret_key) {
            let credentials = Credentials::new(key, secret, None, None, "env");
            config_builder = config_builder.credentials_provider(credentials);
        }

        let config = config_builder.build();
        let client = Client::from_conf(config);

        Ok(Self { client, bucket })
    }

    /// Create an S3Driver with explicit configuration.
    pub fn new(client: Client, bucket: String) -> Self {
        Self { client, bucket }
    }

    /// Normalize path: strip leading slash for S3 key consistency.
    fn normalize_key(path: &str) -> &str {
        path.strip_prefix('/').unwrap_or(path)
    }
}

#[async_trait]
impl StorageDriver for S3Driver {
    async fn read(
        &self,
        path: &str,
        options: Option<ReadOptions>,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>, StorageError> {
        let key = Self::normalize_key(path);

        let mut request = self.client.get_object().bucket(&self.bucket).key(key);

        // Apply byte range if provided
        if let Some(ref opts) = options {
            if let Some((start, end)) = opts.range {
                let range_str = match end {
                    Some(e) => format!("bytes={}-{}", start, e),
                    None => format!("bytes={}-", start),
                };
                request = request.range(range_str);
            }
        }

        let resp = request.send().await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("NoSuchKey") || msg.contains("404") {
                StorageError::NotFound(path.to_string())
            } else if msg.contains("AccessDenied") || msg.contains("403") {
                StorageError::PermissionDenied(path.to_string())
            } else {
                StorageError::Backend(format!("S3 GetObject failed: {}", e))
            }
        })?;

        let byte_stream = resp.body;
        let async_read = byte_stream.into_async_read();

        Ok(Box::pin(async_read))
    }

    async fn write(
        &self,
        path: &str,
        mut content: Pin<Box<dyn AsyncRead + Send>>,
        content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let key = Self::normalize_key(path);

        // Read the entire content into memory for the PutObject call.
        // For very large files, consider using multipart upload instead.
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut content, &mut buf)
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to read content: {}", e)))?;

        let body = ByteStream::from(buf);

        let mut request = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .body(body);

        if let Some(ct) = content_type {
            request = request.content_type(ct);
        }

        request.send().await.map_err(|e| {
            let msg = e.to_string();
            if msg.contains("AccessDenied") || msg.contains("403") {
                StorageError::PermissionDenied(path.to_string())
            } else {
                StorageError::Backend(format!("S3 PutObject failed: {}", e))
            }
        })?;

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let key = Self::normalize_key(path);

        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("AccessDenied") || msg.contains("403") {
                    StorageError::PermissionDenied(path.to_string())
                } else {
                    StorageError::Backend(format!("S3 DeleteObject failed: {}", e))
                }
            })?;

        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<FileStat, StorageError> {
        let key = Self::normalize_key(path);

        let resp = self
            .client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("NotFound") || msg.contains("404") || msg.contains("NoSuchKey") {
                    StorageError::NotFound(path.to_string())
                } else if msg.contains("AccessDenied") || msg.contains("403") {
                    StorageError::PermissionDenied(path.to_string())
                } else {
                    StorageError::Backend(format!("S3 HeadObject failed: {}", e))
                }
            })?;

        let size = resp.content_length.unwrap_or(0) as u64;

        let modified: DateTime<Utc> = resp
            .last_modified
            .and_then(|t| {
                DateTime::from_timestamp(t.secs(), t.subsec_nanos())
            })
            .unwrap_or_else(Utc::now);

        let content_type = resp.content_type;

        Ok(FileStat {
            size,
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
        self.copy(src, dest).await?;
        self.delete(src).await?;
        Ok(())
    }

    async fn copy(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        let src_key = Self::normalize_key(src);
        let dest_key = Self::normalize_key(dest);

        let copy_source = format!("{}/{}", self.bucket, src_key);

        self.client
            .copy_object()
            .bucket(&self.bucket)
            .copy_source(&copy_source)
            .key(dest_key)
            .send()
            .await
            .map_err(|e| {
                let msg = e.to_string();
                if msg.contains("NoSuchKey") || msg.contains("404") {
                    StorageError::NotFound(src.to_string())
                } else if msg.contains("AccessDenied") || msg.contains("403") {
                    StorageError::PermissionDenied(src.to_string())
                } else {
                    StorageError::Backend(format!("S3 CopyObject failed: {}", e))
                }
            })?;

        Ok(())
    }

    fn list(
        &self,
        prefix: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, StorageError>> + Send>> {
        let client = self.client.clone();
        let bucket = self.bucket.clone();
        let prefix = prefix.map(|p| Self::normalize_key(p).to_string());

        Box::pin(async_stream::try_stream! {
            let mut continuation_token: Option<String> = None;

            loop {
                let mut request = client
                    .list_objects_v2()
                    .bucket(&bucket);

                if let Some(ref p) = prefix {
                    request = request.prefix(p);
                }

                if let Some(ref token) = continuation_token {
                    request = request.continuation_token(token);
                }

                let resp = request.send().await.map_err(|e| {
                    StorageError::Backend(format!("S3 ListObjectsV2 failed: {}", e))
                })?;

                if let Some(contents) = resp.contents {
                    for object in contents {
                        if let Some(key) = object.key {
                            yield key;
                        }
                    }
                }

                match resp.next_continuation_token {
                    Some(token) => continuation_token = Some(token),
                    None => break,
                }
            }
        })
    }
}
