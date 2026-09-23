use anyhow::{Context, Result};
use fs2::FileExt;
use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, SystemTime},
};

pub struct Cache {
    _lock: fs::File,
    path: PathBuf,
    limit: u64,
    days: u64,
}

pub fn default_path(root: &Path) -> PathBuf {
    let base = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".cache")))
    };
    base.filter(|p| p.is_absolute())
        .map(|p| p.join("asiji"))
        .unwrap_or_else(|| root.join(".cache"))
}

impl Cache {
    pub fn open(path: &Path, limit_mb: u64, days: u64) -> Result<Self> {
        fs::create_dir_all(path)?;
        let lock = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(path.join(".lock"))?;
        FileExt::try_lock_exclusive(&lock)
            .context("Кэш занят другим плеером; закройте его или укажите --cache-dir")?;
        Ok(Self {
            _lock: lock,
            path: path.into(),
            limit: limit_mb.saturating_mul(1024 * 1024),
            days,
        })
    }

    pub fn prune(&self, protected: Option<&Path>, clear: bool) -> Result<u64> {
        let mut files = Vec::new();
        for item in fs::read_dir(&self.path)? {
            let item = item?;
            if !item.file_type()?.is_file() {
                continue;
            }
            let name = item.file_name().to_string_lossy().into_owned();
            let Some(hash) = name
                .strip_prefix("rust-pcm-")
                .and_then(|s| s.strip_suffix(".wav"))
            else {
                continue;
            };
            if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
                continue;
            }
            let meta = item.metadata()?;
            files.push((item.path(), meta.len(), meta.modified()?));
        }
        files.sort_by_key(|f| f.2);
        let mut size: u64 = files.iter().map(|f| f.1).sum();
        let mut removed = 0;
        for (path, len, modified) in files {
            if protected == Some(path.as_path()) {
                continue;
            }
            let expired = SystemTime::now()
                .duration_since(modified)
                .unwrap_or_default()
                > Duration::from_secs(self.days.saturating_mul(86400));
            if clear || expired || size > self.limit {
                fs::remove_file(&path)
                    .with_context(|| format!("Не удалось очистить {}", path.display()))?;
                size = size.saturating_sub(len);
                removed += len;
            }
        }
        Ok(removed)
    }
}

impl Drop for Cache {
    fn drop(&mut self) {
        let _ = self.prune(None, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cleanup_protects_active_and_unrelated_files_and_locks() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let active = dir.path().join(format!("rust-pcm-{}.wav", "a".repeat(64)));
        let old = dir.path().join(format!("rust-pcm-{}.wav", "b".repeat(64)));
        fs::write(&active, b"active")?;
        fs::write(&old, b"old")?;
        fs::write(dir.path().join("personal.wav"), b"keep")?;
        let cache = Cache::open(dir.path(), 0, 30)?;
        assert!(Cache::open(dir.path(), 1, 30).is_err());
        assert_eq!(cache.prune(Some(&active), false)?, 3);
        assert!(active.exists());
        drop(cache);
        assert!(!active.exists());
        assert!(dir.path().join("personal.wav").exists());
        Ok(())
    }
}
