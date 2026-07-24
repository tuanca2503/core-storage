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
    Raw,
}
// ===== Base Error Type =====
pub struct BaseError {
    pub code: u32,
    pub kind: ErrorKind,
    pub severity: ErrorSeverity,
    pub message: String,
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
    pub location: &'static std::panic::Location<'static>,
}

impl BaseError {
    #[track_caller]
    pub fn new(
        code: u32,
        kind: ErrorKind,
        severity: ErrorSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            kind,
            severity,
            message: message.into(),
            source: None,
            location: std::panic::Location::caller(),
        }
    }
    #[track_caller]
    pub fn with_err<E>(self, err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self {
            source: Some(Box::new(err)),
            ..self
        }
    }
    //
    pub fn print(&self) {
        eprintln!("{self}");
    }
    //
    #[track_caller]
    pub fn external_error(msg: impl Into<String>, code: Codes) -> Self {
        Self::new(
            code as u32,
            ErrorKind::External,
            ErrorSeverity::Error,
            msg.into(),
        )
    }
    //
    #[track_caller]
    pub fn software_warning(msg: impl Into<String>, code: Codes) -> Self {
        Self::new(
            code as u32,
            ErrorKind::Software,
            ErrorSeverity::Warning,
            msg.into(),
        )
    }
    #[track_caller]
    pub fn software_error(msg: impl Into<String>, code: Codes) -> Self {
        Self::new(
            code as u32,
            ErrorKind::Software,
            ErrorSeverity::Error,
            msg.into(),
        )
    }
    //
    #[track_caller]
    pub fn system_warning(msg: impl Into<String>, code: Codes) -> Self {
        Self::new(
            code as u32,
            ErrorKind::System,
            ErrorSeverity::Warning,
            msg.into(),
        )
    }
    #[track_caller]
    pub fn system_error(msg: impl Into<String>, code: Codes) -> Self {
        Self::new(
            code as u32,
            ErrorKind::System,
            ErrorSeverity::Error,
            msg.into(),
        )
    }
    //
    #[track_caller]
    pub fn user_error(msg: impl Into<String>, code: Codes) -> Self {
        Self::new(
            code as u32,
            ErrorKind::User,
            ErrorSeverity::Error,
            msg.into(),
        )
    }
}
impl std::fmt::Display for BaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: [{}] From {} \n └─ {}",
            self.severity, self.code, self.kind, self.message
        )?;
        Ok(())
    }
}
impl std::fmt::Debug for BaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: [{}] From {} ({}:{})\n └─ {}",
            self.severity,
            self.code,
            self.kind,
            self.location.file(),
            self.location.line(),
            self.message
        )?;
        if let Some(src) = &self.source {
            write!(f, "\n    caused by: {}", src)?;
        }
        Ok(())
    }
}
impl<E> From<E> for BaseError
where
    E: std::error::Error + Send + Sync + 'static,
{
    #[track_caller]
    fn from(err: E) -> Self {
        Self::new(
            Codes::Internal as u32,
            ErrorKind::Software,
            ErrorSeverity::Fatal,
            err.to_string(),
        )
        .with_err(Box::new(err))
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
    #[track_caller]
    fn message<M: Into<String>>(self, msg: M) -> BaseResult<T> {
        self.map_err(|e| {
            let mut base = e.into();
            base.message = msg.into();
            base
        })
    }
    #[track_caller]
    fn code(self, code: i32) -> BaseResult<T> {
        self.map_err(|e| {
            let mut base = e.into();
            base.code = code as u32;
            base
        })
    }
    #[track_caller]
    fn kind(self, kind: ErrorKind) -> BaseResult<T> {
        self.map_err(|e| {
            let mut base = e.into();
            base.kind = kind;
            base
        })
    }
}
