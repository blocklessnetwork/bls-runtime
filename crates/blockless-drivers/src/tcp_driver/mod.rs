use crate::{Driver, ErrorKind};
use std::future::Future;
use std::pin::Pin;
use tokio::net::TcpStream;
use wasi_cap_std_sync::net::Socket;
use wasi_common::WasiFile;
use blockless_multiaddr as multiaddr;

pub struct TcpDriver {}

impl Driver for TcpDriver {
    fn open(
        &self,
        socket: &str,
        opts: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn WasiFile>, ErrorKind>> + Send>> {
        let socket: String = socket.into();
        //this open options.
        let _opts: String = opts.into();
        return Box::pin(async move {
            let ma = multiaddr::parse(socket.as_bytes()).map_err(|e| {
                eprintln!("error open:{:?}", e);
                ErrorKind::DriverBadOpen
            })?;
            if ma.paths_ref().len() < 1 {
                eprintln!("error open error path : {}", socket);
                return Err(ErrorKind::DriverBadOpen);
            }
            let socket = ma.paths_ref()[1].value_to_str();
            let stream = match TcpStream::connect(socket).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error connect in driver {}: {}", socket, e);
                    return Err(ErrorKind::ConnectError);
                }
            };
            let stream = cap_std::net::TcpStream::from_std(stream.into_std().unwrap());
            let socket: Socket = Socket::from(stream);
            let stream: Box<dyn WasiFile> = Box::<dyn WasiFile>::from(socket);
            Ok(stream)
        });
    }
}

unsafe impl std::marker::Send for TcpDriver {}
unsafe impl std::marker::Sync for TcpDriver {}
