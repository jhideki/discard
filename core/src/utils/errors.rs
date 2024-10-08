use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct TimeoutError {
    pub operation: String,
    pub duration: std::time::Duration,
}

impl fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Operation '{}', timed out after {:?}",
            self.operation, self.duration
        )
    }
}

#[derive(Debug)]
pub enum ParseEnumError {
    InvalidVariant,
}

impl fmt::Display for ParseEnumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseEnumError::InvalidVariant => write!(f, "Invalid enum variant"),
        }
    }
}

impl Error for ParseEnumError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        None
    }
}
