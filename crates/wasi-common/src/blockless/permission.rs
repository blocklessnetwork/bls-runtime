#[derive(Clone)]
pub struct Permission {
    pub url: String,
    pub schema: String,
}

impl Permission {
    pub fn is_permision(&self, url: &str) -> bool {
        url.to_ascii_lowercase().starts_with(&self.url)
    }
}
