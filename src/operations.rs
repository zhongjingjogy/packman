use crate::models;
use crate::security::SecurityManager;
use rusty_s3::{Bucket, Credentials, S3Action, UrlStyle};
use sha1::{Digest, Sha1};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PackageError {
    #[error("Checksum verification failed: {0}")]
    ChecksumMismatch(String),
    #[error("Missing checksum file")]
    MissingChecksum,
}

// Package conflict status enum
#[derive(Debug)]
pub enum PackageConflictStatus {
    NoConflict,                  // 没有冲突
    VersionExists,               // 完全相同的版本已存在
    HigherVersionExists(String), // 已存在更高版本
}
use chrono;
use quick_xml::de::from_str;
use reqwest::Client as ReqwestClient;
use semver;
use serde::Deserialize;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
use std::time::Duration;
use toml;
use url;

// 自定义结构体用于解析 XML 响应
#[derive(Debug, Deserialize)]
struct ListObjectsResponse {
    #[serde(rename = "Contents", default)]
    contents: Vec<S3Object>,
}

#[derive(Debug, Deserialize)]
struct S3Object {
    #[serde(rename = "Key")]
    key: String,
    #[serde(rename = "Size")]
    size: Option<u64>,
    #[serde(rename = "LastModified")]
    last_modified: Option<String>,
}

pub struct PackageManager {
    bucket: Bucket,
    client: ReqwestClient,
    credentials: Option<Credentials>,
}

impl PackageManager {
    pub fn new(
        endpoint: &str,
        access_key: &str,
        secret_key: &str,
        bucket: &str,
    ) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // 处理端点 URL，确保是正确的绝对 URL
        println!("原始端点: {}", endpoint);

        // 确保有 http(s):// 前缀
        let base_url = if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            format!("https://{}", endpoint)
        } else {
            endpoint.to_string()
        };

        // 删除末尾的斜杠
        let base_url = base_url.trim_end_matches('/').to_string();

        println!("处理后的端点: {}", base_url);

        // 创建 rusty-s3 bucket，使用 Url::parse 解析 URL
        let url = url::Url::parse(&base_url)?;
        println!("解析的 URL: {}", url);

        let bucket = Bucket::new(
            url,
            UrlStyle::Path,
            bucket.to_string(),
            "us-east-1".to_string(),
        )?;

        println!("创建的 bucket URL: {}", bucket.base_url());

        // 准备凭证
        let credentials = if !access_key.is_empty() && !secret_key.is_empty() {
            Some(Credentials::new(
                access_key.to_string(),
                secret_key.to_string(),
            ))
        } else {
            None
        };

        // 创建 HTTP 客户端
        let client = ReqwestClient::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            bucket,
            client,
            credentials,
        })
    }

    pub async fn list_packages(
        &self,
    ) -> Result<Vec<models::Package>, Box<dyn Error + Send + Sync>> {
        let mut packages = Vec::new();

        // 创建列表对象的操作
        let action = self.bucket.list_objects_v2(self.credentials.as_ref());
        let url = action.sign(Duration::from_secs(3600));

        // 执行请求
        let response = self.client.get(url).send().await?;
        let content = response.text().await?;

        // 解析 XML 响应
        let list_result: ListObjectsResponse = from_str(&content)?;

        for obj in list_result.contents {
            if let Some(name) = obj.key.strip_suffix(".zip") {
                let parts: Vec<&str> = name.split('-').collect();
                if parts.len() >= 2 {
                    packages.push(models::Package {
                        name: parts[0..parts.len() - 1].join("-"),
                        version: parts.last().unwrap().to_string(),
                        author: String::new(), // Will be populated from metadata
                        description: String::new(), // Will be populated from metadata
                        dependencies: HashMap::new(), // Will be populated from metadata
                        encryption: None,
                        is_locked: false,
                        lock_reason: None,
                        storage: models::Storage {
                            path: obj.key.clone(),
                            checksum: String::new(),
                            size: obj.size.unwrap_or(0),
                            created_at: obj.last_modified.unwrap_or_default(),
                        },
                    });
                }
            }
        }
        Ok(packages)
    }

    pub async fn push_package(
        &self,
        package_path: &Path,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Validate package path exists
        if !package_path.exists() {
            return Err("Package path does not exist".into());
        }

        // 先尝试读取pack.toml，如果不存在再尝试pack.json
        let toml_path = package_path.join("pack.toml");
        let json_path = package_path.join("pack.json");

        let mut metadata: models::PackageMetadata = if toml_path.exists() {
            // 读取TOML格式
            let toml_content = std::fs::read_to_string(&toml_path)?;
            toml::from_str(&toml_content)?
        } else if json_path.exists() {
            // 读取JSON格式
            let json_content = std::fs::read_to_string(&json_path)?;
            serde_json::from_str(&json_content)?
        } else {
            return Err("Neither pack.toml nor pack.json found in package directory".into());
        };

        // 检查包是否已存在以及版本冲突
        match self
            .check_package_conflict(&metadata.name, &metadata.version)
            .await
        {
            Ok(conflict_status) => match conflict_status {
                PackageConflictStatus::NoConflict => {
                    // 继续处理，没有冲突
                }
                PackageConflictStatus::VersionExists => {
                    return Err(format!("Package {}@{} already exists. Use --force to overwrite or choose a different version.", 
                        metadata.name, metadata.version).into());
                }
                PackageConflictStatus::HigherVersionExists(existing_version) => {
                    return Err(format!("A higher version ({}) of package {} already exists. Current version: {}. Use --force to ignore this warning or choose a higher version.", 
                        existing_version, metadata.name, metadata.version).into());
                }
            },
            Err(e) => {
                return Err(format!("Error checking package conflicts: {}", e).into());
            }
        }

        // Create zip archive
        let zip_name = format!("{}-{}.zip", metadata.name, metadata.version);
        let zip_path = std::env::temp_dir().join(&zip_name);
        let file = std::fs::File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        // Add files to zip
        for entry in walkdir::WalkDir::new(package_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                let relative_path = path.strip_prefix(package_path)?;
                zip.start_file(relative_path.to_string_lossy(), Default::default())?;
                std::io::copy(&mut std::fs::File::open(path)?, &mut zip)?;
            }
        }
        zip.finish()?;

        // Read zip file content
        let mut file_content = std::fs::read(&zip_path)?;

        // Check if encryption is enabled in pack.toml
        if let Some(encryption) = &metadata.encryption {
            if encryption.enabled {
                let security = SecurityManager::new();
                let (encrypted_data, salt) = SecurityManager::encrypt_data(&file_content)
                    .map_err(|e| format!("Encryption failed: {}", e))?;

                // Update encryption config with salt
                if let Some(encryption) = &mut metadata.encryption {
                    encryption.salt = Some(salt);
                }

                file_content = encrypted_data.into_bytes();
            }
        }

        // Calculate sha1 hash
        let mut hasher = Sha1::new();
        hasher.update(&file_content);
        let checksum = format!("{:x}", hasher.finalize());

        // Upload package file
        let action = self.bucket.put_object(self.credentials.as_ref(), &zip_name);
        let url = action.sign(Duration::from_secs(3600));

        let response = self
            .client
            .put(url)
            .header("Content-Type", "application/zip")
            .body(file_content)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to upload object: {}", response.status()).into());
        }

        // Upload checksum file
        let checksum_name = format!("{}.sha1", zip_name);
        let action = self
            .bucket
            .put_object(self.credentials.as_ref(), &checksum_name);
        let url = action.sign(Duration::from_secs(3600));

        let response = self
            .client
            .put(url)
            .header("Content-Type", "text/plain")
            .body(checksum.clone())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to upload checksum file: {}", response.status()).into());
        }

        // Clean up temp file
        std::fs::remove_file(zip_path)?;

        // Update package checksum in registry metadata
        let mut registry_meta = self.get_registry_metadata().await?;
        if let Some(pkg) = registry_meta
            .locked_packages
            .iter_mut()
            .find(|p| p.name == metadata.name && p.version == metadata.version)
        {
            pkg.checksum = checksum;
        }
        self.save_registry_metadata(&registry_meta).await?;

        Ok(())
    }

    // 检查包是否存在以及版本冲突
    pub async fn check_package_conflict(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<PackageConflictStatus, Box<dyn Error + Send + Sync>> {
        // 获取所有可用包
        let packages = self.list_packages().await?;

        // 过滤出与给定包名相同的包
        let same_name_packages: Vec<&models::Package> =
            packages.iter().filter(|p| p.name == package_name).collect();

        if same_name_packages.is_empty() {
            // 没有同名包，没有冲突
            return Ok(PackageConflictStatus::NoConflict);
        }

        // 检查是否有相同版本
        for pkg in &same_name_packages {
            if pkg.version == version {
                // 检查包是否被锁定
                if pkg.is_locked {
                    return Err(format!(
                        "Package {}@{} is locked and cannot be modified. Reason: {}",
                        package_name,
                        version,
                        pkg.lock_reason.as_deref().unwrap_or("Unknown")
                    )
                    .into());
                }
                return Ok(PackageConflictStatus::VersionExists);
            }
        }

        // 解析当前版本
        let current_version = semver::Version::parse(version)
            .map_err(|_| format!("Invalid version format: {}", version))?;

        // 检查是否有更高版本
        let mut higher_versions = Vec::new();

        for pkg in same_name_packages {
            if let Ok(existing_version) = semver::Version::parse(&pkg.version) {
                if existing_version > current_version {
                    higher_versions.push(pkg.version.clone());
                }
            }
        }

        if !higher_versions.is_empty() {
            // 找出最高版本
            let highest_version = higher_versions
                .iter()
                .max_by(|a, b| {
                    let a_ver =
                        semver::Version::parse(a).unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                    let b_ver =
                        semver::Version::parse(b).unwrap_or_else(|_| semver::Version::new(0, 0, 0));
                    a_ver.cmp(&b_ver)
                })
                .unwrap();

            return Ok(PackageConflictStatus::HigherVersionExists(
                highest_version.to_string(),
            ));
        }

        // 没有冲突
        Ok(PackageConflictStatus::NoConflict)
    }

    // 强制推送包，忽略冲突
    pub async fn force_push_package(
        &self,
        package_path: &Path,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Validate package path exists
        if !package_path.exists() {
            return Err("Package path does not exist".into());
        }

        // 先尝试读取pack.toml，如果不存在再尝试pack.json
        let toml_path = package_path.join("pack.toml");
        let json_path = package_path.join("pack.json");

        let metadata: models::PackageMetadata = if toml_path.exists() {
            // 读取TOML格式
            let toml_content = std::fs::read_to_string(&toml_path)?;
            toml::from_str(&toml_content)?
        } else if json_path.exists() {
            // 读取JSON格式
            let json_content = std::fs::read_to_string(&json_path)?;
            serde_json::from_str(&json_content)?
        } else {
            return Err("Neither pack.toml nor pack.json found in package directory".into());
        };

        // Create zip archive (不进行冲突检查)
        let zip_name = format!("{}-{}.zip", metadata.name, metadata.version);
        let zip_path = std::env::temp_dir().join(&zip_name);
        let file = std::fs::File::create(&zip_path)?;
        let mut zip = zip::ZipWriter::new(file);

        // Add files to zip
        for entry in walkdir::WalkDir::new(package_path) {
            let entry = entry?;
            if entry.file_type().is_file() {
                let path = entry.path();
                let relative_path = path.strip_prefix(package_path)?;
                zip.start_file(relative_path.to_string_lossy(), Default::default())?;
                std::io::copy(&mut std::fs::File::open(path)?, &mut zip)?;
            }
        }
        zip.finish()?;

        // Read zip file content
        let file_content = std::fs::read(&zip_path)?;

        // 创建 PUT 对象操作
        let action = self.bucket.put_object(self.credentials.as_ref(), &zip_name);
        let url = action.sign(Duration::from_secs(3600));

        // 上传对象
        let response = self
            .client
            .put(url)
            .header("Content-Type", "application/zip")
            .body(file_content)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to upload object: {}", response.status()).into());
        }

        // Clean up temp file
        std::fs::remove_file(zip_path)?;

        Ok(())
    }

    pub async fn pull_package(
        &self,
        package_name: &str,
        output_dir: &Path,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Parse package name and version
        let (name, version) = match package_name.split_once('@') {
            Some((n, v)) => (n, v),
            None => return Err("Invalid package format, expected name@version".into()),
        };

        // Create temp directory
        let temp_dir = std::env::temp_dir().join(format!("{}-{}", name, version));
        std::fs::create_dir_all(&temp_dir)?;

        // Download package and checksum
        let zip_name = format!("{}-{}.zip", name, version);
        let checksum_name = format!("{}.sha1", zip_name);
        let zip_path = temp_dir.join(&zip_name);
        let _checksum_path = temp_dir.join(&checksum_name);

        // Download package file
        let action = self.bucket.get_object(self.credentials.as_ref(), &zip_name);
        let url = action.sign(Duration::from_secs(3600));

        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(format!("Failed to download package: {}", response.status()).into());
        }

        let bytes = response.bytes().await?;
        std::fs::write(&zip_path, &bytes)?;

        // Download checksum file
        let action = self
            .bucket
            .get_object(self.credentials.as_ref(), &checksum_name);
        let url = action.sign(Duration::from_secs(3600));

        let response = self.client.get(url).send().await;
        let expected_checksum = match response {
            Ok(resp) if resp.status().is_success() => resp.text().await?,
            _ => return Err(PackageError::MissingChecksum.into()),
        };

        // Verify checksum
        let mut hasher = Sha1::new();
        hasher.update(&bytes);
        let actual_checksum = format!("{:x}", hasher.finalize());

        if actual_checksum != expected_checksum {
            return Err(PackageError::ChecksumMismatch(format!(
                "Package {}@{}: expected {}, got {}",
                name, version, expected_checksum, actual_checksum
            ))
            .into());
        }

        // Extract package if checksum matches
        let file = std::fs::File::open(&zip_path)?;
        let content = std::fs::read(&zip_path)?;

        // Check if decryption is needed
        let metadata = self.get_package_metadata(&zip_path)?;
        let content = if let Some(encryption) = &metadata.encryption {
            if encryption.enabled {
                if let (Some(encrypted_password), Some(salt)) =
                    (&encryption.encrypted_password, &encryption.salt)
                {
                    let security = SecurityManager::new();
                    SecurityManager::decrypt_data(encrypted_password, salt)
                        .map_err(|e| format!("Decryption failed: {}", e))?
                } else {
                    return Err("Missing encrypted password or salt for decryption".into());
                }
            } else {
                content
            }
        } else {
            content
        };

        // Write decrypted content back to temp file
        std::fs::write(&zip_path, &content)?;

        let file = std::fs::File::open(&zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(output_dir)?;

        // Verify metadata - 先检查pack.toml，然后是pack.json
        let toml_path = output_dir.join("pack.toml");
        let json_path = output_dir.join("pack.json");

        let metadata: models::PackageMetadata = if toml_path.exists() {
            // 读取TOML格式
            let toml_content = std::fs::read_to_string(&toml_path)?;
            toml::from_str(&toml_content)?
        } else if json_path.exists() {
            // 读取JSON格式
            let json_content = std::fs::read_to_string(&json_path)?;
            serde_json::from_str(&json_content)?
        } else {
            return Err("Neither pack.toml nor pack.json found in downloaded package".into());
        };

        if metadata.name != name || metadata.version != version {
            return Err("Downloaded package metadata mismatch".into());
        }

        // Clean up temp files
        std::fs::remove_file(zip_path)?;
        std::fs::remove_dir_all(temp_dir)?;

        Ok(())
    }

    /// 测试连接到 MinIO 存储和 bucket 的可用性
    pub async fn test_connection(&self) -> Result<(bool, String), Box<dyn Error + Send + Sync>> {
        // 测试 MinIO 连接
        let action = self.bucket.list_objects_v2(self.credentials.as_ref());
        let url = action.sign(Duration::from_secs(10));

        // 尝试发送请求
        let response = match self.client.get(url).send().await {
            Ok(resp) => resp,
            Err(e) => return Ok((false, format!("无法连接到存储服务: {}", e))),
        };

        // 检查状态码
        if !response.status().is_success() {
            return Ok((
                false,
                format!("存储服务返回错误状态码: {}", response.status()),
            ));
        }

        // 尝试解析 XML 响应，检查 bucket 是否可用
        let content = match response.text().await {
            Ok(text) => text,
            Err(e) => return Ok((false, format!("无法读取响应内容: {}", e))),
        };

        // 尝试解析 XML 内容
        match from_str::<ListObjectsResponse>(&content) {
            Ok(_) => Ok((
                true,
                format!("成功连接到存储服务，bucket '{}' 可用", self.bucket.name()),
            )),
            Err(e) => Ok((false, format!("无法解析响应内容，bucket 可能不存在: {}", e))),
        }
    }

    // 锁定特定版本的包，防止被修改
    pub async fn lock_package(
        &self,
        package_name: &str,
        version: &str,
        reason: &str,
        user: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 获取注册表元数据
        let mut metadata = self.get_registry_metadata().await?;

        // 检查包是否存在
        let packages = self.list_packages().await?;
        let found = packages
            .iter()
            .any(|p| p.name == package_name && p.version == version);

        if !found {
            return Err(format!("Package {}@{} does not exist", package_name, version).into());
        }

        // 检查包是否已经被锁定
        if metadata
            .locked_packages
            .iter()
            .any(|lp| lp.name == package_name && lp.version == version)
        {
            return Err(format!("Package {}@{} is already locked", package_name, version).into());
        }

        // 添加锁定信息
        let now = chrono::Utc::now().to_rfc3339();
        // Get package checksum if available
        let package = packages
            .iter()
            .find(|p| p.name == package_name && p.version == version);
        let checksum = package.map_or("".to_string(), |p| p.storage.checksum.clone());

        metadata.locked_packages.push(models::LockedPackage {
            name: package_name.to_string(),
            version: version.to_string(),
            lock_reason: reason.to_string(),
            locked_at: now.clone(),
            locked_by: user.to_string(),
            checksum,
        });

        metadata.last_updated = now;

        // 保存更新后的元数据
        self.save_registry_metadata(&metadata).await?;

        Ok(())
    }

    // 解锁特定版本的包
    pub async fn unlock_package(
        &self,
        package_name: &str,
        version: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 获取注册表元数据
        let mut metadata = self.get_registry_metadata().await?;

        // 查找锁定的包索引
        let index = metadata
            .locked_packages
            .iter()
            .position(|lp| lp.name == package_name && lp.version == version);

        if let Some(idx) = index {
            // 移除锁定信息
            metadata.locked_packages.remove(idx);
            metadata.last_updated = chrono::Utc::now().to_rfc3339();

            // 保存更新后的元数据
            self.save_registry_metadata(&metadata).await?;
            Ok(())
        } else {
            Err(format!("Package {}@{} is not locked", package_name, version).into())
        }
    }

    // 备份特定版本的包
    pub async fn backup_package(
        &self,
        package_name: &str,
        version: &str,
        reason: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 检查包是否存在
        let packages = self.list_packages().await?;
        let package = packages
            .iter()
            .find(|p| p.name == package_name && p.version == version);

        let package = match package {
            Some(pkg) => pkg,
            None => {
                return Err(format!("Package {}@{} does not exist", package_name, version).into());
            }
        };

        // 获取注册表元数据
        let mut metadata = self.get_registry_metadata().await?;

        // 如果备份未启用，则启用它
        if !metadata.backup_enabled {
            metadata.backup_enabled = true;
        }

        // 创建备份名称
        let now = chrono::Utc::now();
        let timestamp = now.to_rfc3339();
        let backup_name = format!(
            "{}-{}-backup-{}.zip",
            package_name,
            version,
            now.timestamp()
        );

        // 复制包到备份位置
        let source_key = &package.storage.path;
        let action = self
            .bucket
            .get_object(self.credentials.as_ref(), source_key);
        let url = action.sign(Duration::from_secs(3600));

        // 下载原始对象
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(format!(
                "Failed to download object for backup: {}",
                response.status()
            )
            .into());
        }

        let bytes = response.bytes().await?;

        // 上传到备份位置
        let action = self
            .bucket
            .put_object(self.credentials.as_ref(), &backup_name);
        let url = action.sign(Duration::from_secs(3600));

        // 上传备份对象
        let response = self
            .client
            .put(url)
            .header("Content-Type", "application/zip")
            .body(bytes)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to upload backup: {}", response.status()).into());
        }

        // 更新元数据
        metadata.backups.push(models::PackageBackup {
            original_path: source_key.to_string(),
            backup_path: backup_name,
            timestamp,
            reason: reason.to_string(),
        });

        metadata.last_updated = chrono::Utc::now().to_rfc3339();

        // 保存更新后的元数据
        self.save_registry_metadata(&metadata).await?;

        Ok(())
    }

    // 从备份恢复特定版本的包
    pub async fn restore_package_from_backup(
        &self,
        package_name: &str,
        version: &str,
        timestamp: Option<&str>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 获取注册表元数据
        let metadata = self.get_registry_metadata().await?;

        // 查找备份
        let mut filtered_backups: Vec<&models::PackageBackup> = metadata
            .backups
            .iter()
            .filter(|b| {
                let parts: Vec<&str> = b
                    .original_path
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .split('-')
                    .collect();

                if parts.len() >= 2 {
                    let name = parts[0..parts.len() - 1].join("-");
                    let ver = parts.last().unwrap_or(&"");
                    name == package_name && *ver == version
                } else {
                    false
                }
            })
            .collect();

        if filtered_backups.is_empty() {
            return Err(
                format!("No backups found for package {}@{}", package_name, version).into(),
            );
        }

        // 如果指定了时间戳，找到特定备份
        let backup = if let Some(ts) = timestamp {
            filtered_backups
                .iter()
                .find(|b| b.timestamp.starts_with(ts))
                .ok_or_else(|| format!("No backup found with timestamp {}", ts))?
        } else {
            // 否则使用最新的备份
            filtered_backups.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
            filtered_backups
                .first()
                .ok_or_else(|| "Failed to get latest backup".to_string())?
        };

        // 从备份恢复
        let backup_key = &backup.backup_path;
        let action = self
            .bucket
            .get_object(self.credentials.as_ref(), backup_key);
        let url = action.sign(Duration::from_secs(3600));

        // 下载备份对象
        let response = self.client.get(url).send().await?;
        if !response.status().is_success() {
            return Err(format!("Failed to download backup: {}", response.status()).into());
        }

        let bytes = response.bytes().await?;

        // 确定原始路径
        let original_key = &backup.original_path;

        // 上传回原始位置
        let action = self
            .bucket
            .put_object(self.credentials.as_ref(), original_key);
        let url = action.sign(Duration::from_secs(3600));

        // 上传恢复的对象
        let response = self
            .client
            .put(url)
            .header("Content-Type", "application/zip")
            .body(bytes)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to restore package: {}", response.status()).into());
        }

        Ok(())
    }

    // 获取注册表元数据
    async fn get_registry_metadata(
        &self,
    ) -> Result<models::RegistryMetadata, Box<dyn Error + Send + Sync>> {
        // 元数据文件名
        let metadata_key = "registry-metadata.json";

        // 尝试获取元数据
        let action = self
            .bucket
            .get_object(self.credentials.as_ref(), metadata_key);
        let url = action.sign(Duration::from_secs(3600));

        // 下载元数据
        let response = self.client.get(url).send().await;

        match response {
            Ok(resp) if resp.status().is_success() => {
                // 解析元数据
                let content = resp.text().await?;
                let metadata: models::RegistryMetadata = serde_json::from_str(&content)?;
                Ok(metadata)
            }
            _ => {
                // 如果不存在，创建新的元数据
                let now = chrono::Utc::now().to_rfc3339();
                Ok(models::RegistryMetadata {
                    registry_name: "MinIO Package Registry".to_string(),
                    backup_enabled: false,
                    locked_packages: Vec::new(),
                    backups: Vec::new(),
                    last_updated: now,
                })
            }
        }
    }

    // 保存注册表元数据
    fn get_package_metadata(
        &self,
        zip_path: &Path,
    ) -> Result<models::PackageMetadata, Box<dyn Error + Send + Sync>> {
        // 创建临时目录解压zip文件
        let temp_dir = tempfile::tempdir()?;
        let file = std::fs::File::open(zip_path)?;
        let mut archive = zip::ZipArchive::new(file)?;
        archive.extract(&temp_dir)?;

        // 查找pack.toml或pack.json
        let toml_path = temp_dir.path().join("pack.toml");
        let json_path = temp_dir.path().join("pack.json");

        let metadata: models::PackageMetadata = if toml_path.exists() {
            let toml_content = std::fs::read_to_string(&toml_path)?;
            toml::from_str(&toml_content)?
        } else if json_path.exists() {
            let json_content = std::fs::read_to_string(&json_path)?;
            serde_json::from_str(&json_content)?
        } else {
            return Err("Neither pack.toml nor pack.json found in package".into());
        };

        Ok(metadata)
    }

    async fn save_registry_metadata(
        &self,
        metadata: &models::RegistryMetadata,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        // 元数据文件名
        let metadata_key = "registry-metadata.json";

        // 序列化元数据
        let content = serde_json::to_string_pretty(metadata)?;

        // 上传元数据
        let action = self
            .bucket
            .put_object(self.credentials.as_ref(), metadata_key);
        let url = action.sign(Duration::from_secs(3600));

        // 上传对象
        let response = self
            .client
            .put(url)
            .header("Content-Type", "application/json")
            .body(content)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(format!("Failed to save registry metadata: {}", response.status()).into());
        }

        Ok(())
    }
}
