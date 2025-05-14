use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub schema_version: String,
    pub packages: Vec<Package>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub dependencies: HashMap<String, String>,
    pub storage: Storage,
    #[serde(default)]
    pub is_locked: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Storage {
    pub path: String,
    #[serde(default)]
    pub checksum: String,
    pub size: u64,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub includes: Vec<String>,
    pub excludes: Vec<String>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PackageBackup {
    pub original_path: String, 
    pub backup_path: String,
    pub timestamp: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistryMetadata {
    pub registry_name: String,
    pub backup_enabled: bool,
    pub locked_packages: Vec<LockedPackage>,
    pub backups: Vec<PackageBackup>,
    pub last_updated: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub lock_reason: String,
    pub locked_at: String,
    pub locked_by: String,
    #[serde(default)]
    pub checksum: String,
}
