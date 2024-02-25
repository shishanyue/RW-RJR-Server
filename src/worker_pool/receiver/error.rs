use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    InvalidInput,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    err: Box<dyn std::error::Error + Sync + Send>,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for Error {}

#[allow(dead_code)]
impl Error {
    pub fn new<E>(kind: ErrorKind, err: E) -> Self
    where
        E: Into<Box<dyn std::error::Error + Sync + Send>>,
    {
        Self {
            kind,
            err: err.into(),
        }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    pub fn err(&self) -> &dyn std::error::Error {
        self.err.as_ref()
    }
}
