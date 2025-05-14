use dotenv::dotenv;
use rusty_s3::{Bucket, Credentials, UrlStyle};
use std::env;
use url::Url;

#[allow(dead_code)]
pub fn create_client() -> Result<Bucket, Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    
    let endpoint = env::var("S3_ENDPOINT")?;
    let access_key = env::var("S3_ACCESS_KEY")?;
    let secret_key = env::var("S3_SECRET_KEY")?;
    let bucket_name = env::var("S3_BUCKET").unwrap_or_else(|_| "default".to_string());

    // 确保有http(s)://前缀
    let base_url = if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        format!("https://{}", endpoint)
    } else {
        endpoint.to_string()
    };
    
    // 删除末尾的斜杠
    let base_url = base_url.trim_end_matches('/').to_string();
    
    let url = Url::parse(&base_url)?;
    let bucket = Bucket::new(
        url,
        UrlStyle::Path,
        bucket_name,
        "us-east-1".to_string(),
    )?;
    
    Ok(bucket)
}

#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    Ok(())
}
