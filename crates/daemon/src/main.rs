mod app;
mod chunk_queue;
mod fast_queue;
mod object_queue;
mod clean;

pub use chunk_queue::ChunkQueue;
pub use fast_queue::FastQueue;
pub use object_queue::ObjectQueue;

//
use app::App;
use platform::paths;
use service::BaseResult;
use tracing_appender::rolling::{RollingFileAppender, Rotation};

#[tokio::main]
async fn main() -> BaseResult<()> {
    let (non_blocking, _guard) = tracing_appender::non_blocking(RollingFileAppender::new(
        Rotation::DAILY,
        paths::app_join("logs")?,
        "",
    ));
    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_level(true)
        .compact()
        .init();
    tracing::info!("app started");
    //
    App::new().start().await
}
