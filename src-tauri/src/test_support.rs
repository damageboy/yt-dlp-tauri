use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct TestDirectory(pub PathBuf);

impl TestDirectory {
    pub fn new() -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "aria2c-test-{}-{nonce}-{sequence}",
            std::process::id()
        ));
        fs::create_dir(&path).unwrap();
        Self(path)
    }

    pub fn fixture(&self, mode: &str) -> PathBuf {
        let directory = self.0.join("tools with spaces");
        fs::create_dir_all(&directory).unwrap();
        let path = directory.join(if cfg!(windows) {
            "aria2c.exe"
        } else {
            "aria2c"
        });
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/aria2c_probe.rs");
        assert!(Command::new("rustc")
            .args(["--edition=2021"])
            .arg(source)
            .arg("-o")
            .arg(&path)
            .status()
            .unwrap()
            .success());
        fs::write(directory.join("mode"), mode).unwrap();
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
