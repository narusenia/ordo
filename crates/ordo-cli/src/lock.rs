use fs2::FileExt;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub struct BuildLock {
    _file: File,
    path: PathBuf,
}

impl BuildLock {
    pub fn acquire(build_dir: &Path, ctx: &super::context::Context) -> miette::Result<Self> {
        fs::create_dir_all(build_dir)
            .map_err(|e| miette::miette!("failed to create build dir: {e}"))?;

        let lock_path = build_dir.join(".ordo.lock");
        let file = File::create(&lock_path)
            .map_err(|e| miette::miette!("failed to create lock file: {e}"))?;

        if file.try_lock_exclusive().is_err() {
            ctx.style
                .warn("Blocking", "waiting for file lock on build directory...");
            file.lock_exclusive()
                .map_err(|e| miette::miette!("failed to acquire build lock: {e}"))?;
        }

        Ok(Self {
            _file: file,
            path: lock_path,
        })
    }
}

impl Drop for BuildLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
