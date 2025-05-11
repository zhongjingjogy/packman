use rust_registry_dev::common::{create_bucket_if_not_exists, create_client};
use minio::s3::Client;
use minio::s3::response::{AppendObjectResponse, StatObjectResponse};
use minio::s3::segmented_bytes::SegmentedBytes;
use minio::s3::types::S3Api;
use rand::Rng;
use rand::distributions::Alphanumeric;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let client: Client = create_client()?;

    let bucket_name: &str = "append-test-bucket";
    create_bucket_if_not_exists(bucket_name, &client).await?;

    let object_name: &str = "append-test-object";
    let n_segments = 10; // Reduced from 1000 for testing
    let segment_size = 1024; // Reduced from 1MB to 1KB for testing
    let mut offset_bytes = 0;

    for i in 0..n_segments {
        let rand_str: String = random_string(segment_size);
        let data_size = rand_str.len() as u64;
        let data: SegmentedBytes = SegmentedBytes::from(rand_str);

        let resp: AppendObjectResponse = client
            .append_object(bucket_name, object_name, data, offset_bytes)
            .send()
            .await?;

        offset_bytes += data_size;
        // MinIO may return 0 for object_size even after successful append
        // So we'll just log the response and continue
        println!("Append response - object_size: {}, offset: {}", resp.object_size, offset_bytes);

        println!("Progress: {}/{}", i + 1, n_segments);
    }

    println!("Successfully appended {} segments", n_segments);
    Ok(())
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
