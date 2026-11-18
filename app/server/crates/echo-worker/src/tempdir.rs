use std::path::{Path, PathBuf};

use uuid::Uuid;

// Simple  temp dir, probably doesnt alway work ?!?!?
pub struct TempDir {
    pub path_buf: PathBuf,
}

impl TempDir {
    pub async fn new() -> std::io::Result<Self> {
        Self::from_parent(&Path::new("temp")).await
    }

    /// This function should only be used in testing
    pub fn unstable_new_sync() -> Self {
        let temp_dir = Uuid::new_v4();
        let path_buf = PathBuf::from(format!("temp/{}", temp_dir));

        let _ = std::fs::create_dir_all(&path_buf).unwrap();

        TempDir { path_buf }
    }

    pub async fn from_parent(parent: &Path) -> std::io::Result<Self> {
        tokio::fs::create_dir_all(parent).await?;

        let path_buf = parent.join(Uuid::new_v4().to_string());

        tokio::fs::create_dir(&path_buf).await?;

        Ok(Self { path_buf })
    }

    // pub fn from_pathbuf(path: &PathBuf) -> Self {
    //     TempDir {
    //         path_buf: path.to_path_buf(),
    //     }
    // }

    // /// Adds the directory to the path, creating the directory if it doesn't exist.
    // pub async fn join_dir(&self, dir_path: &str) -> PathBuf {
    //     let new_path = self.path_buf.join(dir_path);
    //     let _ = tokio::fs::create_dir_all(&new_path).await.unwrap();

    //     new_path
    // }

    pub fn join(&self, path: &str) -> PathBuf {
        self.path_buf.join(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_dir_all(&self.path_buf) {
            tracing::warn!(
                path = %self.path_buf.display(),
                error = %err,
                "Failed to remove temp directory",
            );
        }
    }
}
