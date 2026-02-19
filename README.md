# 🪲 scarab-pm

The package manager for [Scarab OS](https://github.com/scarab-os/scarab) — fast, minimal, written in Rust.

## Features

- **Fast** — Single static binary, instant operations
- **Prebuilt packages** — Download and install in seconds via GitHub Releases
- **Source builds** — Build from Portfiles when you need customization
- **Dependency resolution** — Automatic recursive dependency handling
- **Verification** — SHA256 checksums for package integrity
- **Patch support** — Apply patches from `ports/<pkg>/patches/` during source builds

## Usage

```sh
scarab sync              # Sync package database from repo
scarab install <pkg>     # Install a prebuilt package
scarab install -f <pkg>  # Force reinstall
scarab remove <pkg>      # Remove a package
scarab search <query>    # Search available packages
scarab list              # List installed packages
scarab info <pkg>        # Show package details
scarab upgrade           # Upgrade all installed packages
scarab build <pkg>       # Build from Portfile (source)
```

## Examples

```sh
# Setup networking
scarab install dhcpcd dropbear

# Install development tools
scarab install gcc make cmake git

# Search for editors
scarab search editor
  * extra/nano 8.7 - Simple text editor

# Build a package from source with custom patches
scarab build curl
```

## How It Works

### Prebuilt Packages (default)

```
scarab sync → downloads repo.json from GitHub Releases
scarab install foo → downloads foo-1.0-x86_64.tar.gz → verifies SHA256 → extracts to /
```

### Source Builds

```
scarab build foo → reads ports/<cat>/foo/Portfile
               → downloads source tarball
               → applies patches/ (if any)
               → runs build() function
               → installs to $PKG
```

## Package Format

Prebuilt packages are compressed tarballs:

```
<name>-<version>-<arch>.tar.gz
<name>-<version>-<arch>.tar.gz.sha256
```

The repository database (`repo.json`) lists all available packages:

```json
[
  {
    "name": "curl",
    "version": "8.18.0",
    "category": "core",
    "description": "URL transfer tool and library",
    "depends": ["mbedtls", "zlib"],
    "size": "1.6M",
    "sha256": "abc123...",
    "filename": "curl-8.18.0-x86_64.tar.gz"
  }
]
```

## Configuration

Config file: `/etc/scarab/scarab.conf`

```json
{
  "root": "/",
  "db_dir": "/var/lib/scarab",
  "cache_dir": "/var/cache/scarab",
  "ports_dir": "/usr/ports",
  "repo_url": "https://github.com/scarab-os/packages/releases/download",
  "arch": "x86_64"
}
```

## Building

```sh
# Native build
cargo build --release

# Cross-compile for Scarab OS (musl static)
cargo build --release --target x86_64-unknown-linux-musl
```

The resulting binary is ~2.7MB.

## Related

- [scarab-os/scarab](https://github.com/scarab-os/scarab) — Scarab OS (distro, ports, build scripts)
- [scarab-os/packages](https://github.com/scarab-os/packages) — Prebuilt binary packages

## License

MIT
