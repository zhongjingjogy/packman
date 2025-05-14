use tempfile::TempDir;
use std::path::PathBuf;
use std::env;
use std::fs;

pub struct TestEnv {
    pub temp_dir: TempDir,
    pub workspace: PathBuf,
    pub s3_endpoint: String,
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
}

impl TestEnv {
    pub fn new() -> Self {
        // Load .env file if exists
        dotenv::dotenv().ok();

        let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
        let workspace = temp_dir.path().to_path_buf();
        
        // Create required subdirectories
        fs::create_dir_all(workspace.join("local-storage")).expect("Failed to create local-storage");
        fs::create_dir_all(workspace.join("remote-storage")).expect("Failed to create remote-storage");
        
        Self {
            temp_dir,
            workspace,
            s3_endpoint: env::var("S3_ENDPOINT").unwrap_or("http://localhost:9000".to_string()),
            access_key: env::var("S3_ACCESS_KEY").unwrap_or("test-access-key".to_string()),
            secret_key: env::var("S3_SECRET_KEY").unwrap_or("test-secret-key".to_string()),
            bucket: env::var("S3_BUCKET").unwrap_or("test-bucket".to_string()),
        }
    }
}

#[macro_export(local_inner_macros)]
macro_rules! test_setup {
    () => {
        {
            let test_env = TestEnv::new();
            std::env::set_current_dir(&test_env.workspace).unwrap();
            test_env
        }
    };
}
