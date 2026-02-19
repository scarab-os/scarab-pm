mod config;
mod db;
mod fetch;
mod package;
mod verify;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "scarab", version, about = "🪲 Scarab OS package manager")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Install a package
    Install {
        /// Package name(s)
        packages: Vec<String>,
        /// Force reinstall
        #[arg(short, long)]
        force: bool,
    },
    /// Remove a package
    Remove {
        /// Package name(s)
        packages: Vec<String>,
    },
    /// Search for packages
    Search {
        /// Search query
        query: String,
    },
    /// List installed packages
    List,
    /// Show package info
    Info {
        /// Package name
        package: String,
    },
    /// Sync package database
    Sync,
    /// Upgrade installed packages
    Upgrade,
    /// Build a package from Portfile
    Build {
        /// Package name
        package: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::Config::load()?;

    match cli.command {
        Commands::Install { packages, force } => {
            for pkg in &packages {
                install_package(&cfg, pkg, force)?;
            }
        }
        Commands::Remove { packages } => {
            for pkg in &packages {
                remove_package(&cfg, pkg)?;
            }
        }
        Commands::Search { query } => search_packages(&cfg, &query)?,
        Commands::List => list_packages(&cfg)?,
        Commands::Info { package } => show_info(&cfg, &package)?,
        Commands::Sync => sync_db(&cfg)?,
        Commands::Upgrade => upgrade_packages(&cfg)?,
        Commands::Build { package } => build_package(&cfg, &package)?,
    }

    Ok(())
}

fn install_package(cfg: &config::Config, name: &str, force: bool) -> Result<()> {
    let db = db::Database::load(cfg)?;

    if !force {
        if let Some(installed) = db.get_installed(name) {
            println!(
                "{} {} {} is already installed (use -f to force)",
                "==>".green().bold(),
                name.bold(),
                installed.version
            );
            return Ok(());
        }
    }

    // Find package in repo
    let pkg = db.find_package(name)?;
    println!(
        "{} Installing {} {}...",
        "==>".green().bold(),
        pkg.name.bold(),
        pkg.version
    );

    // Resolve dependencies
    let deps = db.resolve_deps(&pkg)?;
    if !deps.is_empty() {
        println!("{} Dependencies: {}", "  ->".blue(), deps.join(", "));
        for dep in &deps {
            install_package(cfg, dep, false)?;
        }
    }

    // Download
    let tarball = fetch::download_package(cfg, &pkg)?;

    // Verify
    verify::verify_package(&tarball, &pkg)?;

    // Extract to root
    package::extract_package(&tarball, &cfg.root)?;

    // Record installation
    let mut db = db::Database::load(cfg)?;
    db.record_install(&pkg)?;

    println!(
        "{} Installed {} {}",
        "==>".green().bold(),
        pkg.name.bold(),
        pkg.version
    );
    Ok(())
}

fn remove_package(cfg: &config::Config, name: &str) -> Result<()> {
    let mut db = db::Database::load(cfg)?;

    let installed = db
        .get_installed(name)
        .ok_or_else(|| anyhow::anyhow!("{} is not installed", name))?
        .clone();

    println!(
        "{} Removing {} {}...",
        "==>".green().bold(),
        name.bold(),
        installed.version
    );

    // Remove files
    package::remove_package_files(cfg, &installed)?;

    // Remove from db
    db.remove_installed(name)?;

    println!("{} Removed {}", "==>".green().bold(), name.bold());
    Ok(())
}

fn search_packages(cfg: &config::Config, query: &str) -> Result<()> {
    let db = db::Database::load(cfg)?;
    let results = db.search(query);

    if results.is_empty() {
        println!("No packages found for '{}'", query);
        return Ok(());
    }

    for pkg in results {
        let status = if db.get_installed(&pkg.name).is_some() {
            "*".green().to_string()
        } else {
            " ".to_string()
        };
        println!(
            "{} {}/{} {} - {}",
            status,
            pkg.category.dimmed(),
            pkg.name.bold(),
            pkg.version,
            pkg.description
        );
    }
    Ok(())
}

fn list_packages(cfg: &config::Config) -> Result<()> {
    let db = db::Database::load(cfg)?;
    let installed = db.list_installed();

    if installed.is_empty() {
        println!("No packages installed");
        return Ok(());
    }

    for pkg in installed {
        println!("{:<20} {:<12} {}", pkg.name.bold(), pkg.version, pkg.installed_at);
    }
    Ok(())
}

fn show_info(cfg: &config::Config, name: &str) -> Result<()> {
    let db = db::Database::load(cfg)?;
    let pkg = db.find_package(name)?;

    println!("{:<14} {}", "Name:".bold(), pkg.name);
    println!("{:<14} {}", "Version:".bold(), pkg.version);
    println!("{:<14} {}", "Category:".bold(), pkg.category);
    println!("{:<14} {}", "Description:".bold(), pkg.description);
    println!(
        "{:<14} {}",
        "Depends:".bold(),
        if pkg.depends.is_empty() {
            "none".to_string()
        } else {
            pkg.depends.join(", ")
        }
    );
    println!("{:<14} {}", "Size:".bold(), pkg.size);

    if let Some(installed) = db.get_installed(name) {
        println!(
            "{:<14} {} ({})",
            "Status:".bold(),
            "installed".green(),
            installed.version
        );
    } else {
        println!("{:<14} {}", "Status:".bold(), "not installed".yellow());
    }

    Ok(())
}

fn sync_db(cfg: &config::Config) -> Result<()> {
    println!("{} Syncing package database...", "==>".green().bold());
    fetch::sync_repo_db(cfg)?;
    println!("{} Database synced", "==>".green().bold());
    Ok(())
}

fn upgrade_packages(cfg: &config::Config) -> Result<()> {
    let db = db::Database::load(cfg)?;
    let upgrades = db.check_upgrades();

    if upgrades.is_empty() {
        println!("{} System is up to date", "==>".green().bold());
        return Ok(());
    }

    for (name, old_ver, new_ver) in &upgrades {
        println!("  {} {} -> {}", name.bold(), old_ver.dimmed(), new_ver.green());
    }

    for (name, _, _) in &upgrades {
        install_package(cfg, name, true)?;
    }

    Ok(())
}

fn build_package(cfg: &config::Config, name: &str) -> Result<()> {
    println!(
        "{} Building {} from Portfile...",
        "==>".green().bold(),
        name.bold()
    );

    // Find Portfile
    let portfile = package::find_portfile(&cfg.ports_dir, name)?;
    package::build_from_portfile(&portfile, cfg)?;

    Ok(())
}
