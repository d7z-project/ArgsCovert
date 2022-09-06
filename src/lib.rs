use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum SoftError {
    StdErr(std::fmt::Error),
    IoErr(std::io::Error),
}

impl Display for SoftError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftError::StdErr(err) => write!(f, "StdError: {}", err.to_string()),
            SoftError::IoErr(err) => write!(f, "IOError: {}", err.to_string())
        }
    }
}

impl From<std::io::Error> for SoftError {
    fn from(value: std::io::Error) -> Self {
        Self::IoErr(value)
    }
}

impl From<std::fmt::Error> for SoftError {
    fn from(value: std::fmt::Error) -> Self {
        Self::StdErr(value)
    }
}

impl std::error::Error for SoftError {}
