use api::tcp::client;

#[tokio::main]

async fn main() -> std::io::Result<()> {
    client::new("/home/tuanca/Projects/core-storage/a.py", "0.0.0.0", "7878").await?;
    Ok(())
}