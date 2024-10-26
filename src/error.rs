use std::num::ParseIntError;
// todo: more error types
#[derive(Debug)]
pub struct DBError(pub String);

impl From<std::io::Error> for DBError {
    fn from(item: std::io::Error) -> Self {
        DBError(item.to_string().clone())
    }
}

impl From<ParseIntError> for DBError {
    fn from(item: ParseIntError) -> Self {
        DBError(item.to_string().clone())
    }
}

impl From<std::str::Utf8Error> for DBError {
    fn from(item: std::str::Utf8Error) -> Self {
        DBError(item.to_string().clone())
    }
}

impl From<std::string::FromUtf8Error> for DBError {
    fn from(item: std::string::FromUtf8Error) -> Self {
        DBError(item.to_string().clone())
    }
}
