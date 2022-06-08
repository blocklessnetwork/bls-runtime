mod config;
mod permision;
pub use config::*;
pub use permision::*;

impl BlocklessConfig {
    pub fn is_permission(&self, url: &str) -> bool {
        self.permisions_ref()
            .iter()
            .find(|p| p.is_permision(url))
            .is_some()
    }
}
