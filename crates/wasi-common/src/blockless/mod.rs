mod config;
mod permission;
pub use config::*;
pub use permission::*;

impl BlocklessConfig {
    pub fn resource_permission(&self, url: &str) -> bool {
        self.permisions_ref()
            .iter()
            .any(|p| p.is_permision(url))
    }
}
