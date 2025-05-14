use super::test_helpers::*;
use beepkg::operations::PackageManager;
use std::fs;

#[test]
fn test_package_creation() {
    let env = test_setup!();
    println!("Test package creation started");
    
    // 1. Create test package structure
    let pkg_dir = env.workspace.join("test-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();
    
    // 2. Create pack.toml metadata
    let toml_content = r#"
        name = "test-pkg"
        version = "1.0.0"
        author = "Test User"
        description = "Test package"
        includes = []
        excludes = []
        
        [dependencies]
        dep1 = "1.0"
        dep2 = "2.0"
    "#;
    fs::write(pkg_dir.join("pack.toml"), toml_content).unwrap();
    
    // 3. Create test file
    fs::write(pkg_dir.join("main.rs"), "fn main() {}").unwrap();
    
    // 4. Verify package structure
    assert!(pkg_dir.join("pack.toml").exists());
    assert!(pkg_dir.join("main.rs").exists());
}

#[tokio::test]
async fn test_remote_push_pull() {
    let env = test_setup!();
    let pkg_dir = env.workspace.join("test-pkg");
    
    // 1. 创建测试包目录结构
    let pkg_dir = env.workspace.join("test-pkg");
    fs::create_dir_all(&pkg_dir).unwrap();
    
    // 2. 创建pack.toml元数据文件
    let toml_content = r#"
        name = "test-pkg"
        version = "1.0.0"
        author = "Test User"
        description = "Test package"
        includes = []
        excludes = []
        
        [dependencies]
        dep1 = "1.0"
        dep2 = "2.0"
    "#;
    fs::write(pkg_dir.join("pack.toml"), toml_content).unwrap();
    
    // 3. 创建测试文件
    fs::write(pkg_dir.join("main.rs"), "fn main() {}").unwrap();
    
    // 2. 创建远程存储目录 (模拟 S3 bucket)
    let remote_dir = env.workspace.join("remote-storage");
    fs::create_dir_all(&remote_dir).expect("Failed to create remote storage directory");
    println!("Created remote storage at: {:?}", remote_dir);
    
    // 3. 创建 PackageManager 实例
    let manager = PackageManager::new(
        &env.s3_endpoint,
        &env.access_key,
        &env.secret_key,
        &env.bucket
    ).unwrap();
    
    // 4. 执行推送操作
    println!("Pushing package to remote storage at: {:?}", remote_dir);
    manager.force_push_package(&pkg_dir).await.expect("Failed to push package to remote storage");
    
    // 5. 创建下载目录
    let download_dir = env.workspace.join("downloaded-pkg");
    fs::create_dir_all(&download_dir).expect("Failed to create download directory");
    println!("Download directory created at: {:?}", download_dir);
    
    // 6. 执行拉取操作
    println!("Pulling package to: {:?}", download_dir);
    println!("Verifying remote package exists...");
    let packages = manager.list_packages().await.expect("Failed to list packages");
    assert!(packages.iter().any(|p| p.name == "test-pkg" && p.version == "1.0.0"), "Package not found in remote storage");
    
    let result = manager.pull_package("test-pkg@1.0.0", &download_dir).await;
    if let Err(e) = &result {
        println!("Pull failed with error: {}", e);
        if let Some(checksum_err) = e.downcast_ref::<beepkg::operations::PackageError>() {
            if let beepkg::operations::PackageError::ChecksumMismatch(msg) = checksum_err {
                println!("Checksum mismatch details: {}", msg);
            }
        }
    }
    result.expect("Failed to pull package");
    
    // 7. 验证下载的包结构
    assert!(download_dir.join("pack.toml").exists());
    assert!(download_dir.join("main.rs").exists());
    
    // 8. 验证元数据
    let toml_content = fs::read_to_string(download_dir.join("pack.toml")).unwrap();
    assert!(toml_content.contains("name = \"test-pkg\""));
    assert!(toml_content.contains("version = \"1.0.0\""));
}
