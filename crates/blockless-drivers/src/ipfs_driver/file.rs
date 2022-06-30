use super::api::{Api, Respone};
use crate::IpfsErrorKind;

pub struct FileApi(Api);

impl FileApi {
    pub fn new(api: Api) -> FileApi {
        FileApi(api)
    }

    pub async fn ls(&self) -> Result<Respone, IpfsErrorKind> {
        static LS_API: &str = "api/v0/files/ls";
        let url = self.0.build_url(LS_API);
        self.0.simple_post(&url).await
    }
}
