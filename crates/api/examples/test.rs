use api::tcp::{TransferEvents, server::Server};
use async_trait::async_trait;
use model::{Chunk, Object};

struct TestHandler;

#[async_trait]
impl TransferEvents for TestHandler {
    async fn on_new(&self, object: Object) -> std::io::Result<()> {
        println!(
            "[on_new] uuid={} filename={:?} extension={:?} mime={:?} total_size={}",
            object.external_id,
            object.filename,
            object.extension,
            object.mime_type,
            object.total_size,
        );
        println!("[on_new] -> bắt đầu stream mode, object đã sẵn sàng nhận chunk");
        Ok(())
    }

    async fn on_resume(&self, uuid: &str) -> std::io::Result<(u64, u64, u64)> {
        println!("[on_resume] uuid={uuid} -> client yêu cầu resume upload");

        // TODO thật: tra trong queue xem uuid đã tồn tại chưa, lấy chunk_index hiện có
        let chunk_index: u64 = 0; // giả lập: chưa có chunk nào
        let bytes_received: u64 = 0; // giả lập: chunk_index * chunk_size
        let total_size: u64 = 0; // giả lập: lấy từ object đã lưu theo uuid

        println!(
            "[on_resume] uuid={uuid} -> tra được: chunk_index={chunk_index}, bytes_received={bytes_received}, total_size={total_size}"
        );

        Ok((chunk_index, bytes_received, total_size))
    }
    async fn on_chunk(&self, chunk: Chunk, faster: bool) -> std::io::Result<()> {
        // let preview_len = chunk.data.len().min(16);
        // let hex_preview: String = chunk.data[..preview_len]
        //     .iter()
        //     .map(|b| format!("{b:02x}"))
        //     .collect::<Vec<_>>()
        //     .join(" ");

        // println!(
        //     "[on_chunk] uuid={} chunk_index={} len={} preview=[{}{}]",
        //     chunk.uuid,
        //     chunk.chunk_index,
        //     chunk.len,
        //     hex_preview,
        //     if chunk.data.len() > preview_len { " ..." } else { "" },
        // );

        // TODO thật: push (chunk.uuid, chunk.chunk_index, chunk.data, chunk.len) vào processing queue
        println!("[on_chunk] -> (giả lập) đã đẩy chunk vào processing queue");

        Ok(())
    }

    async fn on_complete(&self, uuid: &str) -> std::io::Result<()> {
        println!("[on_complete] uuid={uuid} -> nhận đủ toàn bộ data, verify OK");
        println!("[on_complete] -> (giả lập) đã push object sang completed queue");
        Ok(())
    }

    async fn on_close(&self, uuid: &str) -> std::io::Result<()> {
        println!("[on_close] uuid={uuid} -> client đóng kết nối / hủy transfer giữa chừng");
        println!("[on_close] -> (giả lập) đã remove object khỏi pending queue");
        Ok(())
    }
}

//cargo run -p core --example test
#[tokio::main]
async fn main() -> std::io::Result<()> {
    let server = Server::start(7878, 120, 600, 50, TestHandler);

    println!("Server đang lắng nghe tại 0.0.0.0:7878");

    tokio::signal::ctrl_c().await?;
    println!("Nhận Ctrl+C, dừng nhận kết nối mới...");
    server.shutdown().await;
    println!("Server đã dừng.");
    Ok(())
}
