use platform::paths;
use service::{BaseError, BaseResult};
use tracing_appender::rolling::{RollingFileAppender, Rotation};

#[tokio::main]
async fn main() -> BaseResult<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(RollingFileAppender::new(
        Rotation::DAILY,
        paths::app_join("logs")?,
        "app.log",
    ));
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_level(true)
        .compact()
        .init();

    tracing::info!("app started");
    
    Ok(())
}
