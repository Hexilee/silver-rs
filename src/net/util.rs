use std::io::{ErrorKind::*, Result};
use std::task::Poll;

pub fn may_block<T>(result: Result<T>) -> Poll<Result<T>> {
    match result {
        Ok(s) => Poll::Ready(Ok(s)),
        Err(ref err) if err.kind() == WouldBlock => Poll::Pending,
        Err(err) => Poll::Ready(Err(err)),
    }
}
