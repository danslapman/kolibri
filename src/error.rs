use std::error::Error as StdError;
use std::fmt::{Debug, Display, Formatter};

pub struct Error {
    pub cause: String
}

impl Error {
    pub fn new(message: String) -> Error {
        Error { cause: message }
    }

    pub fn from<T: Display>(underlying: T) -> Error {
        Error { cause: format!("{}", underlying) }
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cause)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.cause)
    }
}

impl StdError for Error {}