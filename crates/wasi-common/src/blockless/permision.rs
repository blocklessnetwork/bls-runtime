#[derive(Clone)]
pub struct Permision {
    pub schema: String,
    pub url: String,
}

impl Permision {
    pub fn is_permision(&self, url: &str) -> bool {
        url.to_ascii_lowercase().starts_with(&self.url)
    }
}