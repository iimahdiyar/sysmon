use common::error::MonitorResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub listen_addr: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            listen_addr: "0.0.0.0:9000".to_string(),
        }
    }
}

impl ServerConfig {
    pub fn load_or_create(path: impl AsRef<Path>) -> MonitorResult<Self> {
        let path = path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            let cfg = ServerConfig::default();
            fs::write(path, serde_json::to_string_pretty(&cfg)?)?;
            Ok(cfg)
        }
    }
}
