use log::trace;
use s3::{Region, creds::Credentials, BucketConfiguration, Bucket};

use crate::S3ErrorKind;


pub(crate) async fn create(cfg: &str) -> Result<String, S3ErrorKind> {
    let json = match json::parse(cfg) {
        Ok(o) => o,
        Err(_) => return Err(S3ErrorKind::InvalidParameter),
    };
    let access_key = match json["access_key"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let secret_key = match json["secret_key"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };

    let bucket_name = match json["bucket_name"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };

    let endpoint = match json["endpoint"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };

    let region = match json["region"].as_str() {
        Some(s) => String::from(s),
        None => String::from("us-east-1"),
    };

    let region = Region::Custom {
        region: region.into(),
        endpoint: endpoint,
    };

    let credentials = Credentials::new(
        Some(&access_key),
        Some(&secret_key),
        None,
        None,
        None,
    ).unwrap();
    let config = BucketConfiguration::default();
    let response = match Bucket::create(&bucket_name, region, credentials, config).await {
        Ok(respone) => respone,
        Err(e) => {
            trace!("create error: {}", e);
            return Err(S3ErrorKind::RequestError);
        }
    };
    let mut rs = json::JsonValue::new_object();
    rs["code"] = response.response_code.into();
    rs["response_text"] = response.response_text.into();
    let rs = json::stringify(rs);
    Ok(rs)
}