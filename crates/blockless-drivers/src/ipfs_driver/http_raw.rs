use std::{io::Write, collections::HashMap};

use tokio::{net::TcpStream};
use url::Url;
use crate::IpfsErrorKind;

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
        headers.extend(self.header.iter().map(|(k, v)| {
            (k.clone(), v.iter().map(|i| i.clone()).collect())
        }));
        let _ = headers.iter().for_each(|(k, v)| {
            let _ = v.iter().for_each(|i| {
                buf.write(format!("{}: {}", k, i).as_bytes()).unwrap();
                buf.write(EOL).unwrap();
            });
        });
        buf
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
        buf.write(&self.header_raw()).unwrap();
        buf.write(EOL).unwrap();
        buf
    }

    pub async fn connect(&mut self) -> Result<(), IpfsErrorKind> {
        use tokio::io::AsyncWriteExt;
        let addr = self.url.socket_addrs(|| Some(5001)).map_err(|_| IpfsErrorKind::InvalidParameter)?;
        if addr.len() < 1  {
            return Err(IpfsErrorKind::InvalidParameter);
        }
        let mut stream = TcpStream::connect(addr[0]).await.map_err(|_| IpfsErrorKind::RequestError)?;
        stream.write_all(&self.get_req_raw()).await.map_err(|_| IpfsErrorKind::RequestError)?;
        self.tcp_stream = Some(stream);
        Ok(())
    }

    pub fn is_connect(&self) -> bool {
        self.tcp_stream.is_some()
    }

}