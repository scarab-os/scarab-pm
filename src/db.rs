use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageInfo {
    pub name: String,
    pub version: String,
    pub category: String,
    pub description: String,
    pub depends: Vec<String>,
    pub size: String,
    pub sha256: String,
    pub filename: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPackage {
    pub name: String,
    pub version: String,
    pub installed_at: String,
    pub files: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Database {
    #[serde(skip)]
    config: Option<Config>,
    pub packages: Vec<PackageInfo>,
    pub installed: HashMap<String, InstalledPackage>,
}

impl Database {
    pub fn load(cfg: &Config) -> Result<Self> {
        let repo_db_path = cfg.db_dir.join("repo.json");
        let installed_db_path = cfg.db_dir.join("installed.json");

        let packages: Vec<PackageInfo> = if repo_db_path.exists() {
            let content = fs::read_to_string(&repo_db_path)?;
            serde_json::from_str(&content)?
        } else {
            Vec::new()
        };

        let installed: HashMap<String, InstalledPackage> = if installed_db_path.exists() {
            let content = fs::read_to_string(&installed_db_path)?;
            serde_json::from_str(&content)?
        } else {
            HashMap::new()
        };

        Ok(Self {
            config: Some(Config {
                root: cfg.root.clone(),
                db_dir: cfg.db_dir.clone(),
                cache_dir: cfg.cache_dir.clone(),
                ports_dir: cfg.ports_dir.clone(),
                repo_url: cfg.repo_url.clone(),
                arch: cfg.arch.clone(),
            }),
            packages,
            installed,
        })
    }

    pub fn save(&self) -> Result<()> {
        let cfg = self.config.as_ref().unwrap();
        fs::create_dir_all(&cfg.db_dir)?;

        let repo_json = serde_json::to_string_pretty(&self.packages)?;
        fs::write(cfg.db_dir.join("repo.json"), repo_json)?;

        let installed_json = serde_json::to_string_pretty(&self.installed)?;
        fs::write(cfg.db_dir.join("installed.json"), installed_json)?;

        Ok(())
    }

    pub fn find_package(&self, name: &str) -> Result<PackageInfo> {
        self.packages
            .iter()
            .find(|p| p.name == name)
            .cloned()
            .with_context(|| format!("Package '{}' not found. Run 'scarab sync' first?", name))
    }

    pub fn get_installed(&self, name: &str) -> Option<&InstalledPackage> {
        self.installed.get(name)
    }

    pub fn list_installed(&self) -> Vec<&InstalledPackage> {
        let mut list: Vec<_> = self.installed.values().collect();
        list.sort_by(|a, b| a.name.cmp(&b.name));
        list
    }

    pub fn search(&self, query: &str) -> Vec<&PackageInfo> {
        let q = query.to_lowercase();
        self.packages
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&q) || p.description.to_lowercase().contains(&q)
            })
            .collect()
    }

    pub fn resolve_deps(&self, pkg: &PackageInfo) -> Result<Vec<String>> {
        let mut deps = Vec::new();
        let mut visited = Vec::new();
        self.resolve_deps_recursive(pkg, &mut deps, &mut visited)?;
        Ok(deps)
    }

    fn resolve_deps_recursive(
        &self,
        pkg: &PackageInfo,
        deps: &mut Vec<String>,
        visited: &mut Vec<String>,
    ) -> Result<()> {
        for dep_name in &pkg.depends {
            if visited.contains(dep_name) {
                continue;
            }
            visited.push(dep_name.clone());

            if let Ok(dep_pkg) = self.find_package(dep_name) {
                self.resolve_deps_recursive(&dep_pkg, deps, visited)?;

                if self.get_installed(dep_name).is_none() && !deps.contains(dep_name) {
                    deps.push(dep_name.clone());
                }
            }
        }
        Ok(())
    }

    pub fn check_upgrades(&self) -> Vec<(String, String, String)> {
        let mut upgrades = Vec::new();
        for (name, installed) in &self.installed {
            if let Some(repo_pkg) = self.packages.iter().find(|p| &p.name == name) {
                if repo_pkg.version != installed.version {
                    upgrades.push((
                        name.clone(),
                        installed.version.clone(),
                        repo_pkg.version.clone(),
                    ));
                }
            }
        }
        upgrades
    }

    pub fn record_install(&mut self, pkg: &PackageInfo) -> Result<()> {
        let now = chrono_now();
        self.installed.insert(
            pkg.name.clone(),
            InstalledPackage {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                installed_at: now,
                files: Vec::new(), // TODO: track files from tar
            },
        );
        self.save()
    }

    pub fn remove_installed(&mut self, name: &str) -> Result<()> {
        self.installed.remove(name);
        self.save()
    }
}

fn chrono_now() -> String {
    // Simple timestamp without chrono dependency
    let output = std::process::Command::new("date")
        .arg("+%Y-%m-%d %H:%M:%S")
        .output();
    match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        Err(_) => "unknown".to_string(),
    }
}
