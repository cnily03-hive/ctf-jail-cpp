use anyhow::{anyhow, Result};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tempfile::TempDir;
use tokio::sync::RwLock;

pub struct Sandbox {
    temp_dir: TempDir,
}

impl Sandbox {
    pub fn new() -> Result<Self> {
        let temp_dir = TempDir::new()?;
        Ok(Self { temp_dir })
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }
}

pub struct SandboxManager {
    sandboxes: RwLock<HashMap<String, Sandbox>>,
}

impl SandboxManager {
    pub fn new() -> Self {
        Self {
            sandboxes: RwLock::new(HashMap::new()),
        }
    }

    pub async fn create_sandbox(&self, id: &str) -> Result<Sandbox> {
        let sandbox = Sandbox::new()?;

        // Add sandbox to manager
        {
            let mut sandboxes = self.sandboxes.write().await;
            sandboxes.insert(id.to_string(), sandbox);
        }

        // Return a new sandbox instance for execution
        Sandbox::new()
    }

    pub async fn cleanup_sandbox(&self, id: &str) -> Result<()> {
        let mut sandboxes = self.sandboxes.write().await;

        if let Some(_sandbox) = sandboxes.remove(id) {
            // TempDir will be automatically cleaned up when dropped
            Ok(())
        } else {
            Err(anyhow!("Sandbox {} not found", id))
        }
    }

    pub async fn get_sandbox(&self, id: &str) -> Option<PathBuf> {
        let sandboxes = self.sandboxes.read().await;
        sandboxes.get(id).map(|s| s.path().to_path_buf())
    }
}
