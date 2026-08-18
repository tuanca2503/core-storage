use async_trait::async_trait;
use model::Object;

#[async_trait]
pub trait TransferEvents: Send + Sync {
    async fn on_new(&self, object: Object) -> std::io::Result<()>{
        // TODO: push object to pending queue 
        todo!()
    }
    async fn on_resume(&self, uuid: &str) -> std::io::Result<u64> {
        // TODO: nếu không có trong DB, check trong queue
        //       nếu tồn tại trong queue -> trả về số chunk hiện có trong queue

        // TODO: get chunk from queue * chunk size to get continue size

        // TODO: có trả về không có raise lỗi
        todo!()
    }
    async fn on_chunk(&self, ) -> std::io::Result<()>{
        // TODO: when call check uuid and push chunk to process queue 
        todo!()
    }


    async fn on_close(&self, uuid: &str) -> std::io::Result<()>{
        // TODO: remove object to pending queue 
        todo!()
    }
    async fn on_complete(&self, uuid: &str) -> std::io::Result<()>{
        // TODO: push to completed queue 
        todo!()
    }
}
