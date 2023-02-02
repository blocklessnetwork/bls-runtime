use std::{collections::HashMap, sync::Once, time::Duration, pin::Pin};

use bytes::{Bytes, Buf};
use futures_util::StreamExt;
use log::{error, debug};
use reqwest::Response;

use crate::HttpErrorKind;
use futures_core;
use futures_core::Stream;

type StreamInBox = Pin<Box<dyn Stream<Item = reqwest::Result<Bytes>> + Send >>;

struct StreamState {
    stream: StreamInBox,
    buffer: Option<Bytes>,
}


enum HttpCtx {
    Response(Response),
    StreamState(StreamState),
}

/// get the http context
fn get_ctx() -> Option<&'static mut HashMap<u32, HttpCtx>> {
    static mut CTX: Option<HashMap<u32, HttpCtx>> = None;
    static CTX_ONCE: Once = Once::new();
    CTX_ONCE.call_once(||{
        unsafe {
            CTX = Some(HashMap::new());
        }
    });
    unsafe {
        CTX.as_mut()
    }
}

fn increase_fd() -> Option<u32> {
    static mut MAX_HANDLE: u32 = 0;
    unsafe {
        MAX_HANDLE += 1;
        Some(MAX_HANDLE)
    }
}

/// request the url and the return the fd handle.
pub(crate) async fn http_req(
    url: &str, 
    opts: &str,
) -> Result<(u32, i32), HttpErrorKind> {
    let json = match json::parse(opts) {
        Ok(o) => o,
        Err(_) => return Err(HttpErrorKind::RequestError),
    };
    let method = match json["method"].as_str() {
        Some(s) => String::from(s),
        None => return Err(HttpErrorKind::RequestError),
    };
    let connect_timeout = json["connectTimeout"]
        .as_u64()
        .map(|s| Duration::from_secs(s));
    let read_timeout = json["readTimeout"]
        .as_u64()
        .map(|s| Duration::from_secs(s));

    // build the headers from the options json  
    let mut headers = reqwest::header::HeaderMap::new();
    let header_value = &json["headers"];
    let header_obj = match json::parse(header_value.as_str().unwrap()) {
        Ok(o) => o,
        Err(_) => return Err(HttpErrorKind::HeadersValidationError),
    };

    if header_obj.is_object() {
        for (key, value) in header_obj.entries() {
            headers.insert(
                reqwest::header::HeaderName::from_bytes(key.as_bytes()).unwrap(),
                reqwest::header::HeaderValue::from_str(value.as_str().unwrap()).unwrap()
            );
        }
    }

    let mut client_builder = reqwest::ClientBuilder::new();
    if connect_timeout.is_some() {
        client_builder = client_builder.connect_timeout(connect_timeout.unwrap());
    }
    if read_timeout.is_some() {
        client_builder = client_builder.timeout(read_timeout.unwrap());
    }
    let client = client_builder.build().unwrap();
    let req_method = method.to_lowercase();
    let req_builder = match req_method.as_str() {
        "get" => client.get(url),
        "post" => client.post(url),
        _ => return Err(HttpErrorKind::RequestError),
    };
    let resp = req_builder
        .headers(headers)
        .send()
        .await
        .map_err(|e| {
            error!("request send error, {}", e);
            HttpErrorKind::RuntimeError
        })?;
    let status = resp.status().as_u16() as i32;
    let fd = increase_fd().unwrap();
    let ctx = get_ctx().unwrap();
    ctx.insert(fd, HttpCtx::Response(resp));
    Ok((fd, status))
}

/// read from handle
pub(crate) fn http_read_head(
    fd: u32,
    head: &str,
) -> Result<String, HttpErrorKind> {
    let ctx = get_ctx().unwrap();
    let respone = match ctx.get_mut(&fd) {
        Some(HttpCtx::Response(ref h)) => h,
        Some(HttpCtx::StreamState(_)) => return Err(HttpErrorKind::RuntimeError),
        None => return Err(HttpErrorKind::InvalidHandle),
    };
    let headers = respone.headers();
    match headers.get(head) {
        Some(h) => {
            match h.to_str() {
                Ok(s) => Ok(s.into()),
                Err(_) => Err(HttpErrorKind::InvalidEncoding),
            }
        }
        None => Err(HttpErrorKind::HeaderNotFound)
    }
}

async fn stream_read(state: &mut StreamState, dest: &mut [u8]) -> usize {
    let read_call = |buffer: &mut Bytes, dest: &mut [u8]| -> usize {
        let remaining = buffer.remaining();
        if remaining > 0 {
            let n = dest.len().min(remaining);
            buffer.copy_to_slice(&mut dest[..n]);
        }
        if remaining >= dest.len() {
            return dest.len();
        } else if remaining > 0 {
            return remaining;
        }
        0
    };
    let mut readn = 0;
    loop {
        match state.buffer {
            Some(ref mut buffer) => {
                let n = read_call(buffer, &mut dest[readn..]);    
                if n + readn <= dest.len() {
                    readn += n;
                }
                if buffer.remaining() == 0 {
                    state.buffer.take();
                }
                if dest.len() == readn {
                    return readn;
                }
            }
            None => {
                let mut buffer = match state.stream.next().await {
                    Some(Ok(s)) => s,
                    Some(Err(e)) => {
                        debug!("error get messgae {}", e);
                        return readn;
                    }
                    None => return readn,
                };
                let n = read_call(&mut buffer, &mut dest[readn..]);
                if buffer.remaining() > 0 {
                    state.buffer = Some(buffer);
                }
                if readn + n < dest.len() {
                    readn += n;
                } else if  n + readn == dest.len() {
                    return readn + n;
                } else {
                    unreachable!("can't be happend!");
                }
            }
        }
    }
}

pub async fn http_read_body(
    fd: u32, 
    buf: &mut [u8],
) -> Result<u32, HttpErrorKind> {
    let ctx = get_ctx().unwrap();
    match ctx.remove(&fd) {
        Some(HttpCtx::Response(resp)) => {
            let stream = Box::pin(resp.bytes_stream());
            let mut stream_state = StreamState {
                stream,
                buffer: None,
            };
            let readn = stream_read(&mut stream_state, buf).await;
            ctx.insert(fd, HttpCtx::StreamState(stream_state));
            Ok(readn as u32)
        }
        Some(HttpCtx::StreamState(mut stream_state)) => {
            let readn = stream_read(&mut stream_state, buf).await;
            ctx.insert(fd, HttpCtx::StreamState(stream_state));
            Ok(readn as u32)
        }
        None => return Err(HttpErrorKind::InvalidHandle),
    }
}

/// close the handle, destroy the memory.
pub(crate) fn http_close(fd: u32) -> Result<(), HttpErrorKind> {
    let ctx = get_ctx().unwrap();
    match ctx.remove(&fd) {
        Some(_) => Ok(()),
        None => Err(HttpErrorKind::InvalidHandle),
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bytes::BytesMut;
    use tokio::runtime::{Builder, Runtime};
    use std::task::Poll;

    struct TestStream(Vec<Bytes>);
    
    impl Stream for TestStream {
        type Item = reqwest::Result<Bytes>;

        fn poll_next(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Option<Self::Item>> {
            let s = self.get_mut().0.pop().map(|s| Ok(s));
            Poll::Ready(s)
        }
        
    }

    fn get_runtime() -> Runtime {
        let rt = Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        return rt;
    }

    #[test]
    fn test_stream_read_full() {
        let rt = get_runtime();
        rt.block_on(async move {
            let data: &[u8] = &[1, 2, 3, 4, 5, 6];
            let bytes = BytesMut::from(data);
            let mut state = StreamState {
                stream: Box::pin(TestStream(vec![bytes.freeze()])),
                buffer: None,
            };
            let mut dest: [u8; 16] = [0; 16];
            let n = stream_read(&mut state, &mut dest[..]).await;
            assert!(n == data.len());
            assert!(data == &dest[..n]);
        });
    }

    #[test]
    fn test_stream_read_2step() {
        let rt = get_runtime();
        rt.block_on(async move {
            let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,12];
            let bytes = BytesMut::from(data);
            let mut state = StreamState {
                stream: Box::pin(TestStream(vec![bytes.freeze()])),
                buffer: None,
            };
            let mut tmp: [u8; 8] = [0; 8];
            let mut dest: Vec<u8> = Vec::new();
            let mut total = stream_read(&mut state, &mut tmp[..]).await;
            dest.extend(&tmp[..]);
            let n = stream_read(&mut state, &mut tmp[..]).await;
            dest.extend(&tmp[..n]);
            total += n;
            assert!(total == data.len());
            assert!(data == &dest[..total]);
        });
    }

    #[test]
    fn test_stream_read_3step() {
        let rt = get_runtime();
        rt.block_on(async move {
            let data: &[u8] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,12];
            let data2: &[u8] = &[13, 14,15,16];
            let mut state = StreamState {
                stream: Box::pin(TestStream(vec![Bytes::from(data2), Bytes::from(data)])),
                buffer: None,
            };
            let mut src: Vec<u8> = Vec::new();
            src.extend(&data[..]);
            src.extend(&data2[..]);
            let mut tmp: [u8; 8] = [0; 8];
            let mut dest: Vec<u8> = Vec::new();
            let _ = stream_read(&mut state, &mut tmp[..]).await;
            dest.extend(&tmp[..]);
            let n = stream_read(&mut state, &mut tmp[..]).await;
            dest.extend(&tmp[..n]);
            let n = stream_read(&mut state, &mut tmp[..]).await;
            assert!(n == 0);
            assert!(src == dest);
        });
    }
}