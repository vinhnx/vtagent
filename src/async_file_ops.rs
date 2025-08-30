use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::fs as async_fs;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct FileOperation {
    pub path: PathBuf,
    pub content: String,
    pub operation_type: OperationType,
    pub backup_content: Option<String>,
}

#[derive(Debug, Clone)]
pub enum OperationType {
    Create,
    Update,
    Delete,
}

#[derive(Debug)]
pub struct AsyncFileWriter {
    operation_queue: mpsc::Sender<FileOperation>,
    file_backups: Arc<RwLock<HashMap<PathBuf, String>>>,
    max_concurrent: usize,
}

#[derive(Debug)]
pub struct FileOperationResult {
    pub path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
    pub duration_ms: u128,
}

impl AsyncFileWriter {
    pub fn new(max_concurrent: usize) -> Self {
        let (tx, rx) = mpsc::channel(100);

        let writer = Self {
            operation_queue: tx,
            file_backups: Arc::new(RwLock::new(HashMap::new())),
            max_concurrent,
        };

        // Start the background processing task
        writer.start_processor(rx);

        writer
    }

    fn start_processor(&self, mut rx: mpsc::Receiver<FileOperation>) {
        let backups = Arc::clone(&self.file_backups);
        let max_concurrent = self.max_concurrent;

        task::spawn(async move {
            let semaphore = Arc::new(tokio::sync::Semaphore::new(max_concurrent));

            while let Some(operation) = rx.recv().await {
                let permit = semaphore.clone().acquire_owned().await.unwrap();
                let backups_clone = Arc::clone(&backups);

                task::spawn(async move {
                    let _permit = permit;
                    let result = Self::process_operation(operation, backups_clone).await;
                    if let Err(e) = result {
                        eprintln!("Async file operation failed: {}", e);
                    }
                });
            }
        });
    }

    async fn process_operation(
        operation: FileOperation,
        backups: Arc<RwLock<HashMap<PathBuf, String>>>,
    ) -> Result<()> {
        match operation.operation_type {
            OperationType::Create | OperationType::Update => {
                // Create backup before writing
                if let Some(backup) = operation.backup_content {
                    let mut backups_lock = backups.write().await;
                    backups_lock.insert(operation.path.clone(), backup);
                }

                // Write the file asynchronously
                async_fs::write(&operation.path, &operation.content).await?;
            }
            OperationType::Delete => {
                // Create backup before deleting
                if let Some(backup) = operation.backup_content {
                    let mut backups_lock = backups.write().await;
                    backups_lock.insert(operation.path.clone(), backup);
                }

                // Delete the file
                async_fs::remove_file(&operation.path).await?;
            }
        }

        Ok(())
    }

    pub async fn write_file(&self, path: PathBuf, content: String) -> Result<()> {
        // Read existing content for backup
        let backup_content = if path.exists() {
            Some(fs::read_to_string(&path)?)
        } else {
            None
        };

        let operation = FileOperation {
            path,
            content,
            operation_type: OperationType::Update,
            backup_content,
        };

        // Send to async processor
        self.operation_queue
            .send(operation)
            .await
            .map_err(|_| anyhow!("Failed to queue file operation"))?;

        Ok(())
    }

    pub async fn create_file(&self, path: PathBuf, content: String) -> Result<()> {
        let operation = FileOperation {
            path,
            content,
            operation_type: OperationType::Create,
            backup_content: None,
        };

        self.operation_queue
            .send(operation)
            .await
            .map_err(|_| anyhow!("Failed to queue file operation"))?;

        Ok(())
    }

    pub async fn delete_file(&self, path: PathBuf) -> Result<()> {
        // Read content for backup
        let backup_content = if path.exists() {
            Some(fs::read_to_string(&path)?)
        } else {
            None
        };

        let operation = FileOperation {
            path,
            content: String::new(),
            operation_type: OperationType::Delete,
            backup_content,
        };

        self.operation_queue
            .send(operation)
            .await
            .map_err(|_| anyhow!("Failed to queue file operation"))?;

        Ok(())
    }

    pub async fn get_backup(&self, path: &Path) -> Option<String> {
        let backups = self.file_backups.read().await;
        backups.get(path).cloned()
    }

    pub async fn restore_backup(&self, path: PathBuf) -> Result<()> {
        let backup_content = {
            let backups = self.file_backups.read().await;
            backups.get(&path).cloned()
        };

        if let Some(content) = backup_content {
            async_fs::write(&path, content).await?;
            Ok(())
        } else {
            Err(anyhow!("No backup found for file: {:?}", path))
        }
    }

    pub async fn wait_for_completion(&self, timeout_ms: u64) -> Result<()> {
        // Simple timeout-based waiting - in practice, you'd want a more sophisticated approach
        tokio::time::sleep(Duration::from_millis(timeout_ms)).await;
        Ok(())
    }
}

pub struct FileWatcher {
    watched_files: Arc<RwLock<HashMap<PathBuf, String>>>,
}

impl FileWatcher {
    pub fn new() -> Self {
        Self {
            watched_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn start_watching(&self, paths: Vec<PathBuf>) -> Result<()> {
        for path in paths {
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                let mut watched = self.watched_files.write().await;
                watched.insert(path, content);
            }
        }
        Ok(())
    }

    pub async fn get_changes(&self, path: &Path) -> Result<Option<(String, String)>> {
        let watched = self.watched_files.read().await;

        if let Some(original) = watched.get(path) {
            if path.exists() {
                let current = fs::read_to_string(path)?;
                if original != &current {
                    return Ok(Some((original.clone(), current)));
                }
            }
        }

        Ok(None)
    }

    pub async fn update_watched_file(&self, path: PathBuf, content: String) {
        let mut watched = self.watched_files.write().await;
        watched.insert(path, content);
    }
}
