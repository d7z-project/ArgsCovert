use std::convert::Infallible;
use std::fmt::{Debug, Display, Formatter};
use std::io::ErrorKind::Other;

pub enum SoftError {
    StdErr(std::fmt::Error),
    IoErr(std::io::Error),
    AppError(String),
}

impl Display for SoftError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftError::StdErr(err) => write!(f, "StdError: {}", err),
            SoftError::IoErr(err) => write!(f, "IOError: {}", err),
            SoftError::AppError(err) => write!(f, "AppError: {}", err)
        }
    }
}

impl Debug for SoftError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SoftError::StdErr(err) => write!(f, "{}", err),
            SoftError::IoErr(err) => write!(f, "{}", err),
            SoftError::AppError(err) => write!(f, "{}", err)
        }
    }
}

impl From<std::io::Error> for SoftError {
    fn from(value: std::io::Error) -> Self {
        Self::IoErr(value)
    }
}
impl From<Infallible> for SoftError {
    fn from(value: Infallible) -> Self {
        Self::IoErr(std::io::Error::new(Other,value))
    }
}

impl From<std::fmt::Error> for SoftError {
    fn from(value: std::fmt::Error) -> Self {
        Self::StdErr(value)
    }
}


impl std::error::Error for SoftError {}
