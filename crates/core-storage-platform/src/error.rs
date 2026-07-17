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
// Common: 0-99
#[repr(u32)]
pub enum Codes {
    InvalidInput = 1000,
    NotFound,
    PermissionDenied,
    Internal,
    Timeout,
    //
    Command = 2000,
    Corrupt,
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
    pub fn command(msg: impl Into<String>) -> Self {
        Self {
            code: Codes::Command as u32,
            kind: ErrorKind::System,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self {
            code: Codes::Internal as u32,
            kind: ErrorKind::Software,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn bad_response(msg: impl Into<String>) -> Self {
        Self {
            code: Codes::InvalidInput as u32,
            kind: ErrorKind::External,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            code: Codes::NotFound as u32,
            kind: ErrorKind::Software,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }
    pub fn timeout(msg: impl Into<String>) -> Self {
        Self {
            code: Codes::Timeout as u32,
            kind: ErrorKind::System,
            severity: ErrorSeverity::Warning,
            message: msg.into(),
            source: None,
        }
    }

    pub fn system_warning(msg: impl Into<String>, code: Codes) -> Self {
        Self {
            code: code as u32,
            kind: ErrorKind::System,
            severity: ErrorSeverity::Warning,
            message: msg.into(),
            source: None,
        }
    }
    pub fn system_error(msg: impl Into<String>, code: Codes) -> Self {
        Self {
            code: code as u32,
            kind: ErrorKind::System,
            severity: ErrorSeverity::Error,
            message: msg.into(),
            source: None,
        }
    }

}
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
            code: Codes::Internal as u32,
            kind: ErrorKind::Software,
            severity: ErrorSeverity::Fatal,
            message: err.to_string(),
            source: Some(Box::new(err)),
        }
    }
}
// ===== Result Extension Traits =====
pub type BaseResult<T> = std::result::Result<T, BaseError>;
pub trait BaseResultExt<T> {
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
