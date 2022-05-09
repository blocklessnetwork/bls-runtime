use crate::{Driver, ErrorKind};
use std::future::Future;
use std::pin::Pin;
use tokio::net::TcpStream;
use wasi_cap_std_sync::net::Socket;
use wasi_common::WasiFile;

pub struct TcpDriver {}

impl Driver for TcpDriver {
    fn open(
        &mut self,
        socket: &str,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn WasiFile>, ErrorKind>> + Send>> {
        let socket: String = socket.into();
        return Box::pin(async move {
            let socket = socket;
            let stream = match TcpStream::connect(&socket).await {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error connect in driver {}: {}", &socket, e);
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
