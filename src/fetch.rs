use crate::config::Config;
use crate::package::PackageInfo;
use anyhow::{Context, Result};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

pub fn download_package(cfg: &Config, pkg: &PackageInfo) -> Result<PathBuf> {
    let cache_dir = cfg.cache_dir.join("packages");
    fs::create_dir_all(&cache_dir)?;

    let dest = cache_dir.join(&pkg.filename);

    if dest.exists() {
        eprintln!("  -> Using cached {}", pkg.filename);
        return Ok(dest);
    }

    let url = format!("v{}/{}", pkg.version, pkg.filename);
    let full_url = format!("{}/{}", cfg.repo_url, url);

    eprintln!("  -> Downloading {}...", pkg.filename);

    let resp = ureq::get(&full_url)
        .call()
        .with_context(|| format!("Failed to download {}", full_url))?;

    let mut file = fs::File::create(&dest)?;
    let mut reader = resp.into_body().into_reader();
    std::io::copy(&mut reader, &mut file)?;
    file.flush()?;

    Ok(dest)
}

pub fn sync_repo_db(cfg: &Config) -> Result<()> {
    let db_dir = &cfg.db_dir;
    fs::create_dir_all(db_dir)?;

    let url = format!("{}/latest/repo.json", cfg.repo_url);
    eprintln!("  -> Fetching {}...", url);

    let resp = ureq::get(&url)
        .call()
        .with_context(|| format!("Failed to sync from {}", url))?;

    let body = resp.into_body().read_to_string()?;
    fs::write(db_dir.join("repo.json"), &body)?;

    // Count packages
    let packages: Vec<serde_json::Value> = serde_json::from_str(&body)?;
    eprintln!("  -> {} packages in repository", packages.len());

    Ok(())
}
