use crate::{FileStat, ReadOptions, StorageDriver, StorageError};
use async_trait::async_trait;
use futures::Stream;
use std::path::{Path, PathBuf};
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

        Ok(FileStat {
            size: metadata.len(),
            modified,
            content_type: None,
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

            let mut entries = tokio::fs::read_dir(&search_path).await?;
            while let Some(entry) = entries.next_entry().await? {
                if let Ok(relative) = entry.path().strip_prefix(&root) {
                    yield relative.to_string_lossy().to_string();
                }
            }
        })
    }
}
