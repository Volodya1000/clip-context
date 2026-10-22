//! Temporary project fixtures shared by integration tests.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    path: PathBuf,
}

impl Fixture {
    pub fn new() -> Self {
        let unique = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = env::temp_dir().join(format!(
            "clip-context-fixture-{}-{unique}",
            std::process::id()
        ));
        fs::create_dir(&path).expect("temporary directory must be created");
        Self { path }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn file(&self, path: &str, content: &str) {
        let full_path = self.path.join(path);
        let parent = full_path
            .parent()
            .expect("fixtures have parent directories");
        fs::create_dir_all(parent).expect("fixture parent directory must exist");
        fs::write(full_path, content).expect("fixture file must be writable");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.path).expect("temporary fixture must be removable");
    }
}
