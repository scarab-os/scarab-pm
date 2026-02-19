pub use crate::db::PackageInfo;
use crate::config::Config;
use crate::db::InstalledPackage;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Extract a package tarball to the root filesystem
pub fn extract_package(tarball: &Path, root: &Path) -> Result<()> {
    eprintln!("  -> Extracting to {}...", root.display());

    let file = fs::File::open(tarball)?;

    // Detect compression from filename
    let filename = tarball.to_string_lossy();

    if filename.ends_with(".tar.zst") {
        let decoder = zstd::Decoder::new(file)?;
        let mut archive = tar::Archive::new(decoder);
        archive.set_preserve_permissions(true);
        archive.unpack(root)?;
    } else if filename.ends_with(".tar.gz") || filename.ends_with(".tgz") {
        let decoder = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);
        archive.set_preserve_permissions(true);
        archive.unpack(root)?;
    } else if filename.ends_with(".tar.xz") {
        // Use xz command
        let status = Command::new("tar")
            .args(["xJf", &tarball.to_string_lossy(), "-C", &root.to_string_lossy()])
            .status()?;
        if !status.success() {
            bail!("Failed to extract {}", filename);
        }
    } else {
        bail!("Unknown archive format: {}", filename);
    }

    Ok(())
}

/// Remove files belonging to a package
pub fn remove_package_files(cfg: &Config, pkg: &InstalledPackage) -> Result<()> {
    for file in &pkg.files {
        let path = cfg.root.join(file);
        if path.is_file() {
            fs::remove_file(&path).ok();
        }
    }

    // Clean empty directories (reverse order)
    for file in pkg.files.iter().rev() {
        let path = cfg.root.join(file);
        if let Some(parent) = path.parent() {
            fs::remove_dir(parent).ok(); // Only removes if empty
        }
    }

    Ok(())
}

/// Find a Portfile for a package
pub fn find_portfile(ports_dir: &Path, name: &str) -> Result<PathBuf> {
    for category in &["core", "lib", "devel", "net", "extra"] {
        let portfile = ports_dir.join(category).join(name).join("Portfile");
        if portfile.exists() {
            return Ok(portfile);
        }
    }
    bail!(
        "Portfile not found for '{}' in {}",
        name,
        ports_dir.display()
    )
}

/// Build a package from its Portfile (shell-based, delegates to sh)
pub fn build_from_portfile(portfile: &Path, cfg: &Config) -> Result<()> {
    let port_dir = portfile
        .parent()
        .context("Invalid Portfile path")?;
    let name = port_dir
        .file_name()
        .context("Invalid port directory")?
        .to_string_lossy();

    let work_dir = cfg.cache_dir.join("work").join(name.as_ref());
    let pkg_dir = cfg.cache_dir.join("pkg").join(name.as_ref());

    fs::create_dir_all(&work_dir)?;
    fs::create_dir_all(&pkg_dir)?;

    // Source the Portfile and run build()
    let script = format!(
        r#"
set -e
PKG="{pkg_dir}"
SRC="{work_dir}"
MAKEFLAGS="-j$(nproc)"
export PKG SRC MAKEFLAGS

. "{portfile}"

# Download source
if [ -n "$source" ]; then
    cd "$SRC"
    file=$(basename "$source")
    [ -f "$file" ] || curl -fL -o "$file" "$source"
    case "$file" in
        *.tar.gz|*.tgz) tar xzf "$file" ;;
        *.tar.xz) tar xJf "$file" ;;
        *.tar.bz2) tar xjf "$file" ;;
    esac
fi

# Apply patches
if [ -d "{port_dir}/patches" ]; then
    for p in "{port_dir}/patches"/*.patch; do
        [ -f "$p" ] || continue
        echo "  -> Applying patch: $(basename "$p")"
        patch -d "$SRC" -p1 < "$p"
    done
fi

cd "$SRC"
build
"#,
        pkg_dir = pkg_dir.display(),
        work_dir = work_dir.display(),
        portfile = portfile.display(),
        port_dir = port_dir.display(),
    );

    let status = Command::new("sh")
        .arg("-c")
        .arg(&script)
        .status()?;

    if !status.success() {
        bail!("Build failed for {}", name);
    }

    eprintln!("  -> Build complete: {}", pkg_dir.display());
    Ok(())
}
