use std::io::{ErrorKind::*, Result};
use std::task::Poll;

pub fn may_block<T>(result: Result<T>) -> Poll<Result<T>> {
    match result {
        Err(ref err) if err.kind() == WouldBlock => Poll::Pending,
        res => Poll::Ready(res),
    }
}
