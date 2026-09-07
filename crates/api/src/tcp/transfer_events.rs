use async_trait::async_trait;
use model::{Chunk, Object};

#[async_trait]
pub trait TransferEvents: Send + Sync {
    async fn on_new(&self, object: Object) -> std::io::Result<()> {
        // TODO: push object to pending queue
        //insert to db get & return id

        todo!()
    }
    async fn on_resume(&self, uuid: &str) -> std::io::Result<(u64, u64, u64)> {
        // TODO: nếu tồn tại trong queue -> trả về số chunk hiện có trong queue

        // TODO: get chunk from queue * chunk size to get continue size

        // TODO: có trả về không có raise lỗi
        todo!()
    }
    async fn on_chunk(&self, chunk: Chunk, faster: bool) -> std::io::Result<()> {
        /*
        if faster {
            //TODO: push to fast queue here
        } else {
            //TODO: normal queue
        } */

        // TODO: when call check uuid and push chunk to process queue
        //self.tx.send((chunk_index, data, len)).await  // <- nếu channel ĐẦY, dòng này TỰ ĐỘNG CHỜ ở đây
        //.map_err(|e| e.to_string())?;

        // create new chunk > with {obj_id,sequence,checksum} other field set default
        // and then when write task get from queue, he will insert to disk and get & fill other field
        // write > {disk_uuid,chunk_index,segment_index} > complete field > push to insert sql queue
        todo!()
    }

    async fn on_close(&self, uuid: &str) -> std::io::Result<()> {
        // TODO: remove object to pending queue
        // delete from sql > get all chunk pending of this obj > maybe insert to sql in table empty_chunk
        // to set all chunk index not write to db > when all disk full > get chunks_index on this table to
        // set the empty chunk
        todo!()
    }
    async fn on_complete(&self, uuid: &str) -> std::io::Result<()> {
        // TODO: push to completed queue
        // when all done > mark is completed > to insert sql set state committed(1) > push all queue chunk to sql
        todo!()
    }
}
