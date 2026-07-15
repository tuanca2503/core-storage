use core_storage::store::Store;
use core_storage_platform::error::BaseResult;


//cargo run -p core --example test
pub fn main() -> BaseResult<()> {
    let mut configmanager = Store::new("D:\\core-storage.db")?;
    Ok(())
}
