#[cfg(feature="builtin_http")]
mod reqwest_driver;
#[cfg(not(feature="builtin_http"))]
mod cdylib_driver;
#[cfg(not(feature="builtin_http"))]
use cdylib_driver::get_http_driver;
#[cfg(not(feature="builtin_http"))]
pub(crate) use cdylib_driver::init_http_driver;


use crate::HttpErrorKind;


#[cfg(not(feature="builtin_http"))]
pub async fn http_req(url: &str, opts: &str) -> Result<(u32, i32), HttpErrorKind> {
    let driver = get_http_driver().ok_or(HttpErrorKind::InvalidDriver)?;
    driver.http_req(url, opts)
}

#[cfg(feature="builtin_http")]
pub async fn http_req(url: &str, opts: &str) -> Result<(u32, i32), HttpErrorKind> {
    reqwest_driver::http_req(url, opts).await
}

#[cfg(not(feature="builtin_http"))]
pub async fn http_close(fd: u32) -> Result<(), HttpErrorKind> {
    let driver = get_http_driver().ok_or(HttpErrorKind::InvalidDriver)?;
    driver.http_close(fd)?;
    Ok(())
}

#[cfg(feature="builtin_http")]
pub async fn http_close(fd: u32) -> Result<(), HttpErrorKind> {
    reqwest_driver::http_close(fd)
}

#[cfg(not(feature="builtin_http"))]
pub async fn http_read_head(fd: u32, head: &str, buf: &mut [u8]) -> Result<u32, HttpErrorKind> {
    let driver = get_http_driver().ok_or(HttpErrorKind::InvalidDriver)?;
    driver.http_read_head(fd, head.as_bytes(), buf)
}

#[cfg(feature="builtin_http")]
pub async fn http_read_head(fd: u32, head: &str, buf: &mut [u8]) -> Result<u32, HttpErrorKind> {
    let h = reqwest_driver::http_read_head(fd, head)?;
    let sbuf = h.as_bytes();
    let copyn = buf.len().min(sbuf.len());
    buf[..copyn].copy_from_slice(&sbuf);
    Ok(copyn as u32)
}


#[cfg(feature="builtin_http")]
pub async fn http_read_body(fd: u32, buf: &mut [u8]) -> Result<u32, HttpErrorKind> {
    reqwest_driver::http_read_body(fd, buf).await
}

#[cfg(not(feature="builtin_http"))]
pub async fn http_read_body(fd: u32, buf: &mut [u8]) -> Result<u32, HttpErrorKind> {
    let driver = get_http_driver().ok_or(HttpErrorKind::InvalidDriver)?;
    driver.http_read_body(fd, buf)
}