pub mod cli;
pub mod models;
pub mod operations;
pub mod security;


pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

// 添加简化的通用模块
pub mod common {
    use crate::Result;
    use reqwest::Client;

    pub fn create_client() -> Result<Client> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        Ok(client)
    }

    pub async fn create_bucket_if_not_exists(bucket_name: &str, _client: &Client) -> Result<()> {
        // 简化版本 - 实际实现中应该检查并创建 bucket
        println!("Would create bucket: {}", bucket_name);
        Ok(())
    }
}
