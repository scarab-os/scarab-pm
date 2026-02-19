use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub root: PathBuf,
    pub db_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub ports_dir: PathBuf,
    pub repo_url: String,
    pub arch: String,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = PathBuf::from("/etc/scarab/scarab.conf");

        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }

    fn default() -> Self {
        Self {
            root: PathBuf::from("/"),
            db_dir: PathBuf::from("/var/lib/scarab"),
            cache_dir: PathBuf::from("/var/cache/scarab"),
            ports_dir: PathBuf::from("/usr/ports"),
            repo_url: "https://github.com/scarab-os/packages/releases/download".to_string(),
            arch: "x86_64".to_string(),
        }
    }
}
