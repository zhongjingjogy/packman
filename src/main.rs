use clap::Parser;
use packman::{cli, operations, Result};
use std::path::Path;
use tokio;
use dotenv::dotenv;

#[tokio::main]
async fn main() -> Result<()> {
    // 加载 .env 文件
    dotenv().ok();
    
    env_logger::init();
    let args = cli::Cli::parse();

    match args.command {
        cli::Commands::List { endpoint, bucket } => {
            let manager = operations::PackageManager::new(
                &endpoint,
                "", // Access key from env
                "", // Secret key from env  
                &bucket,
            )?;
            let packages = manager.list_packages().await?;
            println!("Packages:");
            for pkg in packages {
                println!("- {}@{}: {}", pkg.name, pkg.version, pkg.description);
            }
        }
        cli::Commands::Push { key, secret, package, force } => {
            let endpoint = std::env::var("MINIO_ENDPOINT")?;
            let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "packages".to_string());
            
            // 优先使用命令行参数，其次使用环境变量
            let access_key = key.or_else(|| std::env::var("MINIO_ACCESS_KEY").ok());
            let secret_key = secret.or_else(|| std::env::var("MINIO_SECRET_KEY").ok());
            
            println!("使用凭证: 访问密钥={}, 密钥={}", 
                access_key.as_deref().unwrap_or("<未提供>"), 
                if secret_key.is_some() { "<已提供>" } else { "<未提供>" });
            
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key.as_deref().unwrap_or(""), 
                &secret_key.as_deref().unwrap_or(""),
                &bucket,
            )?;
            
            // 根据 force 标志选择调用普通 push 还是强制 push
            if force {
                println!("使用强制推送模式，将忽略版本冲突");
                manager.force_push_package(Path::new(&package)).await?;
            } else {
                manager.push_package(Path::new(&package)).await?;
            }
            
            println!("Package pushed successfully");
        }
        cli::Commands::Pull { package, output } => {
            let endpoint = std::env::var("MINIO_ENDPOINT")?;
            let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "packages".to_string());
            
            // 尝试从环境变量中读取凭证
            let access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_default();
            let secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_default();
            
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key, 
                &secret_key,
                &bucket,
            )?;
            
            // 为输出创建默认路径
            let output_path = match output {
                Some(path) => Path::new(&path).to_path_buf(),
                None => std::env::current_dir()?.join("package"),
            };
            
            manager.pull_package(&package, &output_path).await?;
            println!("Package pulled to {}", output_path.display());
        }
        cli::Commands::Test { endpoint, bucket, key, secret } => {
            // 获取端点和 bucket，优先使用命令行参数
            let endpoint = endpoint
                .or_else(|| std::env::var("MINIO_ENDPOINT").ok())
                .ok_or("未指定 MinIO 端点，请使用 --endpoint 参数或设置 MINIO_ENDPOINT 环境变量")?;
                
            let bucket = bucket
                .or_else(|| std::env::var("MINIO_BUCKET").ok())
                .unwrap_or_else(|| "packages".to_string());
            
            // 优先使用命令行参数，其次使用环境变量
            let access_key = key.or_else(|| std::env::var("MINIO_ACCESS_KEY").ok());
            let secret_key = secret.or_else(|| std::env::var("MINIO_SECRET_KEY").ok());
                
            // 创建 PackageManager
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key.as_deref().unwrap_or(""),
                &secret_key.as_deref().unwrap_or(""),
                &bucket,
            )?;
            
            println!("测试连接到端点 {} 和 bucket {}", endpoint, bucket);
            println!("使用凭证: 访问密钥={}, 密钥={}", 
                access_key.as_deref().unwrap_or("<未提供>"), 
                if secret_key.is_some() { "<已提供>" } else { "<未提供>" });
            
            // 执行测试
            let (success, message) = manager.test_connection().await?;
            
            if success {
                println!("✅ {}", message);
            } else {
                println!("❌ {}", message);
            }
        }
        cli::Commands::Lock { package, reason, user } => {
            let endpoint = std::env::var("MINIO_ENDPOINT")?;
            let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "packages".to_string());
            
            // 尝试从环境变量中读取凭证
            let access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_default();
            let secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_default();
            
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key, 
                &secret_key,
                &bucket,
            )?;
            
            // 解析包名和版本
            let (name, version) = match package.split_once('@') {
                Some((n, v)) => (n, v),
                None => return Err("Invalid package format, expected name@version".into()),
            };
            
            manager.lock_package(name, version, &reason, &user).await?;
            println!("Package {}@{} has been locked", name, version);
        }
        cli::Commands::Unlock { package } => {
            let endpoint = std::env::var("MINIO_ENDPOINT")?;
            let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "packages".to_string());
            
            // 尝试从环境变量中读取凭证
            let access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_default();
            let secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_default();
            
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key, 
                &secret_key,
                &bucket,
            )?;
            
            // 解析包名和版本
            let (name, version) = match package.split_once('@') {
                Some((n, v)) => (n, v),
                None => return Err("Invalid package format, expected name@version".into()),
            };
            
            manager.unlock_package(name, version).await?;
            println!("Package {}@{} has been unlocked", name, version);
        }
        cli::Commands::Backup { package, reason } => {
            let endpoint = std::env::var("MINIO_ENDPOINT")?;
            let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "packages".to_string());
            
            // 尝试从环境变量中读取凭证
            let access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_default();
            let secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_default();
            
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key, 
                &secret_key,
                &bucket,
            )?;
            
            // 解析包名和版本
            let (name, version) = match package.split_once('@') {
                Some((n, v)) => (n, v),
                None => return Err("Invalid package format, expected name@version".into()),
            };
            
            manager.backup_package(name, version, &reason).await?;
            println!("Package {}@{} has been backed up", name, version);
        }
        cli::Commands::Restore { package, timestamp } => {
            let endpoint = std::env::var("MINIO_ENDPOINT")?;
            let bucket = std::env::var("MINIO_BUCKET").unwrap_or_else(|_| "packages".to_string());
            
            // 尝试从环境变量中读取凭证
            let access_key = std::env::var("MINIO_ACCESS_KEY").unwrap_or_default();
            let secret_key = std::env::var("MINIO_SECRET_KEY").unwrap_or_default();
            
            let manager = operations::PackageManager::new(
                &endpoint,
                &access_key, 
                &secret_key,
                &bucket,
            )?;
            
            // 解析包名和版本
            let (name, version) = match package.split_once('@') {
                Some((n, v)) => (n, v),
                None => return Err("Invalid package format, expected name@version".into()),
            };
            
            manager.restore_package_from_backup(name, version, timestamp.as_deref()).await?;
            println!("Package {}@{} has been restored from backup", name, version);
        }
    }

    Ok(())
}
