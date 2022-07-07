use super::{
    api::{Api, Response},
    HttpRaw,
};
use crate::IpfsErrorKind;

pub struct FileApi(Api);

impl FileApi {
    pub fn new(api: Api) -> FileApi {
        FileApi(api)
    }

    pub async fn ls(&self, args: Option<String>) -> Result<Response, IpfsErrorKind> {
        static LS_API: &str = "api/v0/files/ls";
        self.0.simple_post(LS_API, args).await
    }

    pub async fn mkdir(&self, args: Option<String>) -> Result<Response, IpfsErrorKind> {
        static MKDIR_API: &str = "api/v0/files/mkdir";
        self.0.simple_post(MKDIR_API, args).await
    }

    pub async fn rm(&self, args: Option<String>) -> Result<Response, IpfsErrorKind> {
        static MKDIR_API: &str = "api/v0/files/rm";
        self.0.simple_post(MKDIR_API, args).await
    }

    pub async fn stat(&self, args: Option<String>) -> Result<Response, IpfsErrorKind> {
        static STAT_API: &str = "api/v0/files/stat";
        self.0.simple_post(STAT_API, args).await
    }

    pub async fn read(&self, args: Option<String>) -> Result<Response, IpfsErrorKind> {
        static READ_API: &str = "api/v0/files/read";
        self.0.simple_post(READ_API, args).await
    }

    pub async fn write(&self, args: Option<String>) -> Result<HttpRaw, IpfsErrorKind> {
        static WRITE_API: &str = "api/v0/files/write";
        self.0.multipart_raw(WRITE_API, args).await
    }
}
