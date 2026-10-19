use rustix::io::{FdFlags, fcntl_getfd, fcntl_setfd};
use std::os::fd::OwnedFd;

pub(crate) fn pipe_cloexec() -> rustix::io::Result<(OwnedFd, OwnedFd)> {
    let (read, write) = rustix::pipe::pipe()?;
    for fd in [&read, &write] {
        fcntl_setfd(fd, fcntl_getfd(fd)? | FdFlags::CLOEXEC)?;
    }
    Ok((read, write))
}

#[cfg(test)]
mod tests {
    #[test]
    fn both_pipe_ends_are_close_on_exec() {
        let (read, write) = super::pipe_cloexec().unwrap();
        for fd in [&read, &write] {
            assert!(rustix::io::fcntl_getfd(fd).unwrap().contains(rustix::io::FdFlags::CLOEXEC));
        }
    }
}
