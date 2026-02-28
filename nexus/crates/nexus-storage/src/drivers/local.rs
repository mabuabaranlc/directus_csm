use crate::{FileStat, ReadOptions, StorageDriver, StorageError, TusDriver};
use async_trait::async_trait;
use futures::Stream;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::io::AsyncRead;

pub struct LocalDriver {
    root: PathBuf,
}

impl LocalDriver {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    /// Guess MIME type from file extension
    fn guess_mime(path: &str) -> Option<String> {
        let ext = path.rsplit('.').next()?.to_lowercase();
        let mime = match ext.as_str() {
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
}

#[async_trait]
impl StorageDriver for LocalDriver {
    async fn read(
        &self,
        path: &str,
        _options: Option<ReadOptions>,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>, StorageError> {
        let full_path = self.resolve_path(path);
        let file = tokio::fs::File::open(&full_path).await?;
        Ok(Box::pin(file))
    }

    async fn write(
        &self,
        path: &str,
        mut content: Pin<Box<dyn AsyncRead + Send>>,
        _content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path);

        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut file = tokio::fs::File::create(&full_path).await?;
        tokio::io::copy(&mut content, &mut file).await?;
        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path);
        tokio::fs::remove_file(&full_path).await?;
        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<FileStat, StorageError> {
        let full_path = self.resolve_path(path);
        let metadata = tokio::fs::metadata(&full_path).await?;
        let modified = metadata
            .modified()
            .map(chrono::DateTime::<chrono::Utc>::from)
            .unwrap_or_else(|_| chrono::Utc::now());

        let content_type = Self::guess_mime(path);

        Ok(FileStat {
            size: metadata.len(),
            modified,
            content_type,
        })
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path);
        Ok(full_path.exists())
    }

    async fn mv(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        let src_path = self.resolve_path(src);
        let dest_path = self.resolve_path(dest);

        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::rename(&src_path, &dest_path).await?;
        Ok(())
    }

    async fn copy(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        let src_path = self.resolve_path(src);
        let dest_path = self.resolve_path(dest);

        if let Some(parent) = dest_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::copy(&src_path, &dest_path).await?;
        Ok(())
    }

    fn list(
        &self,
        prefix: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, StorageError>> + Send>> {
        let root = self.root.clone();
        let prefix = prefix.map(|p| p.to_string());

        Box::pin(async_stream::try_stream! {
            let search_path = if let Some(ref p) = prefix {
                root.join(p)
            } else {
                root.clone()
            };

            // Use a stack-based approach for recursive traversal
            let mut dirs = vec![search_path];
            while let Some(dir) = dirs.pop() {
                let mut entries = match tokio::fs::read_dir(&dir).await {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                while let Some(entry) = entries.next_entry().await? {
                    let path = entry.path();
                    if path.is_dir() {
                        dirs.push(path);
                    } else if let Ok(relative) = path.strip_prefix(&root) {
                        yield relative.to_string_lossy().to_string();
                    }
                }
            }
        })
    }
}

#[async_trait]
impl TusDriver for LocalDriver {
    async fn create_chunk(
        &self,
        key: &str,
        content: &[u8],
        offset: u64,
    ) -> Result<u64, StorageError> {
        let chunks_dir = self.root.join(".tus-chunks");
        tokio::fs::create_dir_all(&chunks_dir).await?;

        let chunk_path = chunks_dir.join(key);

        // If offset is 0 and file exists, truncate it; otherwise append
        if offset == 0 {
            tokio::fs::write(&chunk_path, content).await?;
        } else {
            use tokio::io::AsyncWriteExt;
            let mut file = tokio::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&chunk_path)
                .await?;
            file.write_all(content).await?;
        }

        let metadata = tokio::fs::metadata(&chunk_path).await?;
        Ok(metadata.len())
    }

    async fn finish_chunks(&self, key: &str) -> Result<(), StorageError> {
        let chunks_dir = self.root.join(".tus-chunks");
        let chunk_path = chunks_dir.join(key);

        if !chunk_path.exists() {
            return Err(StorageError::NotFound(key.to_string()));
        }

        // Move the finished file to the final location
        let dest = self.resolve_path(key);
        if let Some(parent) = dest.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        tokio::fs::rename(&chunk_path, &dest).await?;

        Ok(())
    }

    async fn delete_chunks(&self, key: &str) -> Result<(), StorageError> {
        let chunks_dir = self.root.join(".tus-chunks");
        let chunk_path = chunks_dir.join(key);

        if chunk_path.exists() {
            tokio::fs::remove_file(&chunk_path).await?;
        }

        Ok(())
    }
}
