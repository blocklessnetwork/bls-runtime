use std::any::Any;
#[cfg(windows)]
use io_extras::os::windows::{AsRawHandleOrSocket, RawHandleOrSocket};
use io_lifetimes::AsSocketlike;
#[cfg(unix)]
use io_lifetimes::{AsFd, BorrowedFd};

use crate::{WasiFile, file::FdFlags, Error, ErrorExt, file::FileType};

pub enum Socket2Type {
    StreamListener,
    DgramListener,
    SocketStream,
    SocketDgram,
}

pub struct Socket2(socket2::Socket, Socket2Type);

impl Socket2 {

    fn new(s: socket2::Socket, t: Socket2Type) -> Self {
        Self(s, t)
    }

}

#[wiggle::async_trait]
impl WasiFile for Socket2 {
    fn as_any(&self) -> &dyn Any {
        self
    }
    #[cfg(unix)]
    fn pollable(&self) -> Option<rustix::fd::BorrowedFd> {
        Some(self.0.as_fd())
    }

    #[cfg(windows)]
    fn pollable(&self) -> Option<io_extras::os::windows::RawHandleOrSocket> {
        Some(self.0.as_raw_handle_or_socket())
    }
    
    async fn sock_accept(&self, fdflags: FdFlags) -> Result<Box<dyn WasiFile>, Error> {
        let (stream, _) = self.0.accept()?;
        let t = match self.1 {
            Socket2Type::StreamListener => Socket2Type::SocketStream,
            _ => Socket2Type::SocketDgram,
        };
        let stream = Socket2(stream, t);
        stream.set_fdflags(fdflags).await?;
        todo!();
        // Ok(Box::new(stream))
    }

    async fn get_filetype(&self) -> Result<FileType, Error> {
        Ok(match self.1 {
            Socket2Type::StreamListener| Socket2Type::SocketStream => FileType::SocketStream,
            Socket2Type::DgramListener| Socket2Type::SocketDgram => FileType::SocketDgram,
        })
    }

    #[cfg(unix)]
    async fn get_fdflags(&self) -> Result<FdFlags, Error> {
        let fdflags = get_fd_flags(&self.0)?;
        Ok(fdflags)
    }

    async fn set_fdflags(&mut self, fdflags: FdFlags) -> Result<(), Error> {
        if fdflags == FdFlags::NONBLOCK {
            self.0.set_nonblocking(true)?;
        } else if fdflags.is_empty() {
            self.0.set_nonblocking(false)?;
        } else {
            return Err(
                Error::invalid_argument().context("cannot set anything else than NONBLOCK")
            );
        }
        Ok(())
    }

    fn num_ready_bytes(&self) -> Result<u64, Error> {
        Ok(1)
    }
}

#[cfg(windows)]
impl AsSocket for Socket2 {
    #[inline]
    fn as_socket(&self) -> BorrowedSocket<'_> {
        self.0.as_socket()
    }
}

#[cfg(windows)]
impl AsRawHandleOrSocket for Socket2 {
    #[inline]
    fn as_raw_handle_or_socket(&self) -> RawHandleOrSocket {
        self.0.as_raw_handle_or_socket()
    }
}

#[cfg(unix)]
impl AsFd for Socket2 {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}
