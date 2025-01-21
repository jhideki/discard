use discard::utils::enums::RunMessage;
use std::fs;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::{info, warn};

pub fn setup() {}

pub struct Cleanup {
    pub test_paths: Vec<String>,
    pub runmessage_tx: mpsc::Sender<RunMessage>,
}

impl<'a> Cleanup {
    pub fn remove_test_paths(&self) {
        for path in &self.test_paths {
            let path_buf = PathBuf::from(path);
            if path_buf.exists() {
                if path_buf.is_dir() {
                    match fs::remove_dir_all(path) {
                        Ok(_) => info!("Removed test path: {:?}", path),
                        Err(e) => warn!("Failed to remove test paths: {:?} {}", path, e),
                    }
                } else {
                    match fs::remove_file(path) {
                        Ok(_) => info!("Removed test path: {:?}", path),
                        Err(e) => warn!("Failed to remove test paths: {:?} {}", path, e),
                    }
                }
            }
        }
    }
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.remove_test_paths();
    }
}
