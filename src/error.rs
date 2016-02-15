use ssh2;
use std::result;

#[derive(Debug)]
pub enum SSHError {
    Whatever
}

impl From<ssh2::Error> for SSHError {
    fn from(err: ssh2::Error) -> SSHError {
        SSHError::Whatever
    }
}

pub type Result<T> = result::Result<T, SSHError>;
