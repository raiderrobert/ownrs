use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use serde::{de::DeserializeOwned, Serialize};

pub struct FileCache {
    dir: PathBuf,
    ttl: Duration,
}

impl FileCache {
    pub fn new(dir: PathBuf, ttl_secs: u64) -> Result<Self> {
        std::fs::create_dir_all(&dir)?;
        Ok(FileCache {
            dir,
            ttl: Duration::from_secs(ttl_secs),
        })
    }

    pub fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>> {
        let path = self.path_for(key);
        if !path.exists() {
            return Ok(None);
        }

        let metadata = std::fs::metadata(&path)?;
        let modified = metadata.modified()?;
        if SystemTime::now().duration_since(modified)? > self.ttl {
            return Ok(None);
        }

        let data = std::fs::read_to_string(&path)?;
        let value: T = serde_json::from_str(&data)?;
        Ok(Some(value))
    }

    pub fn set<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let path = self.path_for(key);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string(value)?;
        std::fs::write(&path, data)?;
        Ok(())
    }

    pub fn invalidate(&self, key: &str) -> Result<()> {
        let path = self.path_for(key);
        if path.exists() {
            std::fs::remove_file(&path)?;
        }
        Ok(())
    }

    fn path_for(&self, key: &str) -> PathBuf {
        let safe_key = key.replace('/', "__");
        self.dir.join(format!("{safe_key}.json"))
    }
}
