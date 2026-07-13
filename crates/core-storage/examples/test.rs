use core_storage::{error::BaseResult, store::Store};

//cargo run -p core --example test
pub fn main() -> BaseResult<()> {
    let mut configmanager = Store::new("D:\\core-storage.db")?;
    Ok(())
}
