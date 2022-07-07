use std::{collections::HashMap, io::Write};

use crate::IpfsErrorKind;
use bytes::BytesMut;
use httparse::Status;
use log::trace;
use tokio::{io::AsyncReadExt, net::TcpStream};
use url::Url;

pub struct HttpRaw {
    url: Url,
    method: String,
    boundary: Option<String>,
    header: HashMap<String, Vec<String>>,
    tcp_stream: Option<TcpStream>,
}

const EOL: &[u8] = b"\r\n";
const HTTP1: &[u8] = b"HTTP/1.1";

impl HttpRaw {
    pub fn from_url(url: &str) -> Result<HttpRaw, IpfsErrorKind> {
        let url = Url::parse(url).map_err(|_| IpfsErrorKind::InvalidParameter)?;
        Ok(HttpRaw {
            url: url,
            method: "GET".into(),
            boundary: None,
            header: HashMap::new(),
            tcp_stream: None,
        })
    }

    pub fn boundary(&mut self, boundary: Option<String>) {
        self.boundary = boundary;
    }

    pub fn insert_header(&mut self, key: String, value: String) {
        let entry = self.header.get_mut(&key);
        if entry.is_some() {
            entry.map(|v| v.push(value));
        } else {
            self.header.insert(key, vec![value]);
        }
    }

    pub fn method(&mut self, method: &str) {
        self.method = method.into();
    }

    fn header_raw(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1024);
        let mut headers = HashMap::<String, Vec<String>>::with_capacity(1024);
        self.url.host_str().map(|h| {
            let mut host = format!("{}", h);
            self.url.port_or_known_default().map(|p| {
                host += &format!(":{}", p);
            });
            headers.insert("Host".into(), vec![host]);
        });
        headers.insert("Accept".into(), vec!["*/*".into()]);
        headers.extend(
            self.header
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().map(|i| i.clone()).collect())),
        );
        let _ = headers.iter().for_each(|(k, v)| {
            let _ = v.iter().for_each(|i| {
                buf.write(format!("{}: {}", k, i).as_bytes()).unwrap();
                buf.write(EOL).unwrap();
            });
        });
        buf
    }

    fn boundary_begin(boundary: &str) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(1024);
        buf.write(b"--").unwrap();
        buf.write(boundary.as_bytes()).unwrap();
        buf.write(EOL).unwrap();
        buf.write(b"Content-Disposition: form-data").unwrap();
        buf.write(EOL).unwrap();
        buf.write(b"Content-Type: application/octet-stream")
            .unwrap();
        buf.write(EOL).unwrap();
        buf.write(EOL).unwrap();
        buf
    }

    fn boundary_end(boundary: &str) -> Vec<u8> {
        let mut buf = Vec::<u8>::with_capacity(1024);
        buf.write(EOL).unwrap();
        buf.write(b"--").unwrap();
        buf.write(boundary.as_bytes()).unwrap();
        buf.write(b"--").unwrap();
        buf.write(EOL).unwrap();
        buf
    }

    pub async fn write_boundary(&mut self, val: &[u8]) -> Result<u64, IpfsErrorKind> {
        let tcp_stream = self
            .tcp_stream
            .as_mut()
            .ok_or(IpfsErrorKind::RequestError)?;
        let boundary = self.boundary.as_ref().ok_or(IpfsErrorKind::RequestError)?;
        let mut body_buf = Self::boundary_begin(boundary);
        body_buf.write(val).unwrap();
        body_buf.write(&Self::boundary_end(boundary)).unwrap();
        let mut buf = Vec::new();
        buf.write(format!("Content-Length: {}", body_buf.len()).as_bytes())
            .unwrap();
        buf.write(EOL).unwrap();
        buf.write(format!("Content-Type: multipart/form-data; boundary={}", boundary).as_bytes())
            .unwrap();
        buf.write(EOL).unwrap();
        buf.write(EOL).unwrap();
        buf.extend_from_slice(&body_buf);
        Self::write_all(tcp_stream, buf).await?;
        Ok(val.len() as _)
    }

    pub async fn write_all(tcp_stream: &mut TcpStream, v: Vec<u8>) -> Result<(), IpfsErrorKind> {
        use tokio::io::AsyncWriteExt;
        tcp_stream
            .write_all(&v)
            .await
            .map_err(|_| IpfsErrorKind::RequestError)?;
        Ok(())
    }

    pub async fn read_response(&mut self) -> Result<(u16, Vec<u8>), IpfsErrorKind> {
        let tcp_stream = self
            .tcp_stream
            .as_mut()
            .ok_or(IpfsErrorKind::RequestError)?;
        let mut parsed_headers;
        let mut readn = 0;
        let mut parsed_pos = 0;
        let mut status_code = 0;
        let mut bulk = BytesMut::with_capacity(1024 * 10);
        for i in 1..10 {
            let mut headers = vec![httparse::EMPTY_HEADER; 128 * i];
            let mut buf = BytesMut::with_capacity(1024);
            let n = tcp_stream
                .read_buf(&mut buf)
                .await
                .map_err(|_| IpfsErrorKind::RequestError)?;
            readn += n;
            bulk.extend_from_slice(&buf[..n]);
            let mut resp = httparse::Response::new(&mut headers);
            let parsed = resp.parse(&bulk[..readn]).map_err(|e| {
                trace!("{}", e);
                IpfsErrorKind::RequestError
            })?;
            parsed_pos = match parsed {
                Status::Complete(sized) => sized,
                Status::Partial => {
                    continue;
                }
            };
            status_code = resp.code.unwrap();
            parsed_headers = headers;
            break;
        }

        let (pos, len) = loop {
            let parsed = httparse::parse_chunk_size(&bulk[parsed_pos..])
                .map_err(|_| IpfsErrorKind::RequestError)?;
            match parsed {
                Status::Complete((pos, len)) => break (pos, len),
                Status::Partial => {
                    let mut buf = BytesMut::with_capacity(1024);
                    let n = tcp_stream
                        .read_buf(&mut buf)
                        .await
                        .map_err(|_| IpfsErrorKind::RequestError)?;
                    readn += n;
                    bulk.extend_from_slice(&buf[..n]);
                    continue;
                }
            };
        };

        Ok((
            status_code,
            bulk.split_off(parsed_pos + pos).split_to(len as _).to_vec(),
        ))
    }

    fn get_req_raw(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(1024);
        buf.write(self.method.as_bytes()).unwrap();
        buf.write(b" ").unwrap();
        buf.write(self.url.path().as_bytes()).unwrap();
        self.url.query().map(|q| {
            buf.write(b"?").unwrap();
            buf.write(q.as_bytes()).unwrap();
        });
        buf.write(b" ").unwrap();
        buf.write(HTTP1).unwrap();
        buf.write(EOL).unwrap();
        let h = self.header_raw();
        buf.write(&h).unwrap();
        buf
    }

    pub async fn connect(&mut self) -> Result<(), IpfsErrorKind> {
        use tokio::io::AsyncWriteExt;
        let addr = self
            .url
            .socket_addrs(|| Some(5001))
            .map_err(|_| IpfsErrorKind::InvalidParameter)?;
        if addr.len() < 1 {
            return Err(IpfsErrorKind::InvalidParameter);
        }
        let mut stream = TcpStream::connect(addr[0])
            .await
            .map_err(|_| IpfsErrorKind::RequestError)?;
        let headers = self.get_req_raw();
        stream
            .write_all(&headers)
            .await
            .map_err(|_| IpfsErrorKind::RequestError)?;
        self.tcp_stream = Some(stream);
        Ok(())
    }

    pub fn is_connect(&self) -> bool {
        self.tcp_stream.is_some()
    }
}
