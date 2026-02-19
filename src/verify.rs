use crate::package::PackageInfo;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

pub fn verify_package(path: &Path, pkg: &PackageInfo) -> Result<()> {
    if pkg.sha256.is_empty() {
        eprintln!("  -> Warning: no checksum for {}, skipping verification", pkg.name);
        return Ok(());
    }

    eprintln!("  -> Verifying checksum...");

    let data = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;

    let mut hasher = Sha256::new();
    hasher.update(&data);
    let result = hex::encode(hasher.finalize());

    if result != pkg.sha256 {
        bail!(
            "Checksum mismatch for {}!\n  Expected: {}\n  Got:      {}",
            pkg.name,
            pkg.sha256,
            result
        );
    }

    eprintln!("  -> Checksum OK");
    Ok(())
}

/// Compute SHA256 of a file
pub fn sha256_file(path: &Path) -> Result<String> {
    let data = fs::read(path)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(hex::encode(hasher.finalize()))
}
