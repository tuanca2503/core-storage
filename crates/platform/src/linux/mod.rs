use model::{BaseError, BaseResult, ErrorCode};

pub mod disk;
pub mod format;


pub fn ensure_root() -> BaseResult<()> {
    if unsafe { libc::geteuid() } != 0 {
        return Err(BaseError::system_warning(
            "This operation requires root privileges.".to_string(),
            ErrorCode::PermissionDenied,
        ));
    }
    Ok(())
}