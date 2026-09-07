use api::tcp::client;

#[tokio::main]

async fn main() -> std::io::Result<()> {
    // client::new("/home/tuanca/Projects/core-storage/a.py", "0.0.0.0", "7878").await?;
    
    //RESUME TEST
    if let Some((uuid, file_path)) = client::get_tmp().await? {
        client::resume(file_path,uuid, "0.0.0.0".to_string(), "7878".to_string()).await?;
    }
    //END

    Ok(())
}