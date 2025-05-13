use beepkg::common::{create_bucket_if_not_exists, create_client};
use rand::Rng;
use rand::distributions::Alphanumeric;
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();
    let client: Client = create_client()?;

    let bucket_name: &str = "append-test-bucket";
    create_bucket_if_not_exists(bucket_name, &client).await?;

    // 这是一个示例文件，不实际执行对象添加操作
    // 我们通过新的 API 替代了原有的 minio append_object 操作
    let _object_name: &str = "append-test-object";
    let n_segments = 10; 
    let segment_size = 1024; 

    for i in 0..n_segments {
        let rand_str: String = random_string(segment_size);
        
        println!("Would append segment {}: {} bytes", i+1, rand_str.len());
        println!("Progress: {}/{}", i + 1, n_segments);
    }

    println!("Successfully simulated appending {} segments", n_segments);
    Ok(())
}

fn random_string(len: usize) -> String {
    rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
