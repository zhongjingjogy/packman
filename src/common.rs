use dotenv::dotenv;
use minio::s3::creds::StaticProvider;
use minio::s3::http::BaseUrl;
use minio::s3::response::BucketExistsResponse;
use minio::s3::types::S3Api;
use minio::s3::{Client, ClientBuilder};
use std::env;

#[allow(dead_code)]
pub fn create_client() -> Result<Client, Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    
    let endpoint = env::var("MINIO_ENDPOINT")?;
    let access_key = env::var("MINIO_ACCESS_KEY")?;
    let secret_key = env::var("MINIO_SECRET_KEY")?;

    let base_url = endpoint.parse::<BaseUrl>()?;
    log::info!("Connecting to MinIO at: {:?}", base_url);

    let static_provider = StaticProvider::new(&access_key, &secret_key, None);
    
    let client = ClientBuilder::new(base_url)
        .provider(Some(Box::new(static_provider)))
        .build()?;
        
    Ok(client)
}

pub async fn create_bucket_if_not_exists(
    bucket_name: &str,
    client: &Client,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let resp: BucketExistsResponse = client.bucket_exists(bucket_name).send().await?;

    if !resp.exists {
        client.create_bucket(bucket_name).send().await?;
        log::info!("Created bucket: {}", bucket_name);
    }
    Ok(())
}

#[allow(dead_code)]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    Ok(())
}
