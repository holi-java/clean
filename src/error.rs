use std::{fmt::Display, io};

#[derive(Debug)]
pub enum Error {
    IO(io::Error),
    Message(String),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IO(err) => err.fmt(f),
            Error::Message(err) => format!("error: {err}").fmt(f),
        }
    }
}

impl Error {
    pub fn other<S: Display>(err: S) -> Self {
        Error::Message(err.to_string())
    }
}

impl<T> From<T> for Error
where
    io::Error: From<T>,
{
    fn from(err: T) -> Self {
        Error::IO(From::from(err))
    }
}
