use std::fs;
use std::path::PathBuf;
use tracing::{info, warn};

pub struct Cleanup<'a> {
    pub test_paths: &'a Vec<&'a str>,
}

impl<'a> Cleanup<'a> {
    pub fn remove_test_paths(&self) {
        for path in self.test_paths {
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
impl<'a> Drop for Cleanup<'a> {
    fn drop(&mut self) {
        self.remove_test_paths();
    }
}
