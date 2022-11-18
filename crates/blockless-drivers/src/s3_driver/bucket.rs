use log::{error, trace};
use s3::{creds::Credentials, Bucket, BucketConfiguration, Region};

use crate::S3ErrorKind;

struct S3Config {
    access_key: String,
    secret_key: String,
    endpoint: String,
    region: String,
}

fn get_aws_config(json: &json::JsonValue) -> Result<S3Config, S3ErrorKind> {
    let access_key = match json["access_key"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let secret_key = match json["secret_key"].as_str() {
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
    Ok(S3Config {
        access_key,
        secret_key,
        endpoint,
        region,
    })
}

pub(crate) async fn create(cfg: &str) -> Result<String, S3ErrorKind> {
    let json = match json::parse(cfg) {
        Ok(o) => o,
        Err(_) => return Err(S3ErrorKind::InvalidParameter),
    };
    let S3Config {
        access_key,
        secret_key,
        endpoint,
        region,
    } = get_aws_config(&json)?;

    let bucket_name = match json["bucket_name"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let region = Region::Custom {
        region: region.into(),
        endpoint: endpoint,
    };
    let credentials =
        Credentials::new(Some(&access_key), Some(&secret_key), None, None, None).unwrap();
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

pub(crate) async fn list(cfg: &str) -> Result<String, S3ErrorKind> {
    let json = match json::parse(cfg) {
        Ok(o) => o,
        Err(_) => return Err(S3ErrorKind::InvalidParameter),
    };
    let prefix = match json["prefix"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let bucket = new_bucket(&json)?;
    let list_rs = bucket.list(prefix, None).await.map_err(|e| {
        error!("list bucket error:{}", e);
        S3ErrorKind::RequestError
    })?;

    let rs = list_rs
        .iter()
        .map(|rs| {
            let mut obj = json::JsonValue::new_object();
            obj["name"] = rs.name.clone().into();
            obj["is_truncated"] = rs.is_truncated.into();
            rs.prefix.as_ref().map(|prefix| {
                obj["prefix"] = prefix.clone().into();
            });
            let contents = rs
                .contents
                .iter()
                .map(|c| {
                    let mut obj = json::JsonValue::new_object();
                    obj["last_modified"] = c.last_modified.clone().into();
                    obj["e_tag"] = c.e_tag.clone().into();
                    obj["storage_class"] = c.storage_class.clone().into();
                    obj["key"] = c.key.clone().into();
                    obj["size"] = c.size.clone().into();
                    obj
                })
                .collect::<Vec<_>>();
            obj["contents"] = json::JsonValue::Array(contents);
            obj
        })
        .collect::<Vec<_>>();
    let rs = json::JsonValue::Array(rs);
    Ok(json::stringify(rs))
}

fn new_bucket(json: &json::JsonValue) -> Result<Bucket, S3ErrorKind> {
    let bucket_name = match json["bucket_name"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let S3Config {
        access_key,
        secret_key,
        endpoint,
        region,
    } = get_aws_config(&json)?;
    let region = Region::Custom {
        region: region.into(),
        endpoint: endpoint,
    };
    let credentials =
        Credentials::new(Some(&access_key), Some(&secret_key), None, None, None).unwrap();
    let bucket = Bucket::new(&bucket_name, region, credentials).map_err(|e| {
        error!("new bucket error:{}", e);
        S3ErrorKind::InvalidParameter
    })?;
    Ok(bucket)
}

pub(crate) async fn put_object(cfg: &str, buf: &[u8]) -> Result<(), S3ErrorKind> {
    let json = match json::parse(cfg) {
        Ok(o) => o,
        Err(_) => return Err(S3ErrorKind::InvalidParameter),
    };

    let path = match json["path"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let bucket = new_bucket(&json)?;
    let resp = bucket.put_object(path, buf).await.map_err(|e| {
        error!("{}", e);
        S3ErrorKind::RequestError
    })?;
    if resp.status_code() != 200 {
        return Err(S3ErrorKind::RequestError);
    }
    Ok(())
}

pub(crate) async fn get_object(cfg: &str) -> Result<Vec<u8>, S3ErrorKind> {
    let json = match json::parse(cfg) {
        Ok(o) => o,
        Err(_) => return Err(S3ErrorKind::InvalidParameter),
    };

    let path = match json["path"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let bucket = new_bucket(&json)?;
    let resp = bucket.get_object(path).await.map_err(|e| {
        error!("{}", e);
        S3ErrorKind::RequestError
    })?;
    if resp.status_code() != 200 {
        return Err(S3ErrorKind::RequestError);
    }
    Ok(Vec::from(resp.bytes()))
}

pub(crate) async fn delete_object(cfg: &str) -> Result<(), S3ErrorKind> {
    let json = match json::parse(cfg) {
        Ok(o) => o,
        Err(_) => return Err(S3ErrorKind::InvalidParameter),
    };

    let path = match json["path"].as_str() {
        Some(s) => String::from(s),
        None => return Err(S3ErrorKind::InvalidParameter),
    };
    let bucket = new_bucket(&json)?;
    let resp = bucket.delete_object(path).await.map_err(|e| {
        error!("{}", e);
        S3ErrorKind::RequestError
    })?;
    if resp.status_code() != 200 {
        return Err(S3ErrorKind::RequestError);
    }
    Ok(())
}
