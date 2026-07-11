// ===== Error Metadata =====
pub enum ErrorSeverity {
    Info = 0,
    Warning = 1,
    Error = 2,
    Fatal = 3,
}
impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "Info"),
            ErrorSeverity::Warning => write!(f, "Warning"),
            ErrorSeverity::Error => write!(f, "Error"),
            ErrorSeverity::Fatal => write!(f, "Fatal"),
        }
    }
}
pub enum ErrorKind {
    User,     // Lỗi do user input
    System,   // Lỗi hệ thống (network, disk, etc)
    Software, // Lỗi logic code
    External, // Lỗi từ service bên ngoài
}
impl std::fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorKind::User => write!(f, "User"),
            ErrorKind::System => write!(f, "System"),
            ErrorKind::Software => write!(f, "Software"),
            ErrorKind::External => write!(f, "External"),
        }
    }
}
// ===== Error Codes (ranges) =====
pub mod codes {
    // Common: 1000-1999
    pub const INVALID_INPUT: u32 = 1000;
    pub const NOT_FOUND: u32 = 1001;
    pub const PERMISSION_DENIED: u32 = 1002;
    pub const INTERNAL: u32 = 1003;
    pub const OUT_OF_SCOPE: u32 = 1004;
    pub const INVALID_DATA: u32 = 1005;
    pub const TIMEOUT: u32 = 1006;
    // Storage/File: 4000-4999
    pub const FILE_NOT_FOUND: u32 = 4000;
    pub const FILE_PERMISSION: u32 = 4001;
    pub const FILE_CORRUPT: u32 = 4002;
    // Network: 5000-5999
    pub const NETWORK_TIMEOUT: u32 = 5000;
    pub const NETWORK_UNREACHABLE: u32 = 5001;
}
// ===== Base Error Type =====
pub struct BaseError {
    pub code: u32,
    pub kind: ErrorKind,
    pub severity: ErrorSeverity,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
impl BaseError {
    pub fn new<E>(
        code: u32,
        kind: ErrorKind,
        severity: ErrorSeverity,
        message: impl Into<String>,
        err: E,
    ) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            code,
            kind,
            severity,
            message: message.into(),
            source: Some(Box::new(err)),
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: codes::INTERNAL,
            kind: ErrorKind::Software,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn bad_response(msg: impl Into<String>) -> Self {
        Self {
            code: codes::INTERNAL,
            kind: ErrorKind::External,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: codes::NOT_FOUND,
            kind: ErrorKind::Software,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self {
            code: codes::TIMEOUT,
            kind: ErrorKind::System,
            severity: ErrorSeverity::Warning,
            message: msg.into(),
            source: None,
        }
    }
}
// ===== Result Extension Traits =====
pub type BaseResult<T> = std::result::Result<T, BaseError>;
trait BaseResultExt<T> {
    fn message<M: Into<String>>(self, msg: M) -> BaseResult<T>;
    fn code(self, code: i32) -> BaseResult<T>;
    fn kind(self, kind: ErrorKind) -> BaseResult<T>;
}
impl<T, E> BaseResultExt<T> for std::result::Result<T, E>
where
    E: Into<BaseError>,
{
    fn message<M: Into<String>>(self, msg: M) -> BaseResult<T> {
        self.map_err(|e| {
            let mut base = e.into();
            base.message = msg.into();
            base
        })
    }
    fn code(self, code: i32) -> BaseResult<T> {
        self.map_err(|e| {
            let mut base = e.into();
            base.code = code as u32;
            base
        })
    }
    fn kind(self, kind: ErrorKind) -> BaseResult<T> {
        self.map_err(|e| {
            let mut base = e.into();
            base.kind = kind;
            base
        })
    }
}
// =====  =====
impl std::fmt::Display for BaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: [{}] From {} \n └─ {}",
            self.severity, self.code, self.kind, self.message
        )
    }
}
impl std::fmt::Debug for BaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: [{}] From {} \n └─ {}",
            self.severity, self.code, self.kind, self.message
        )
    }
}
impl<E> From<E> for BaseError
where
    E: std::error::Error + Send + Sync + 'static,
{
    fn from(err: E) -> Self {
        Self {
            code: codes::INTERNAL,
            kind: ErrorKind::Software,
            severity: ErrorSeverity::Fatal,
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}
