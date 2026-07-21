use common::error::MonitorResult;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub agent_id: String,
    pub server_addr: String,
    pub interval_secs: u64,
    pub ping_target: String,
    pub log_path: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            agent_id: format!("agent-{}", uuid_like()),
            server_addr: "127.0.0.1:9000".to_string(),
            interval_secs: 3,
            ping_target: "8.8.8.8:53".to_string(),
            log_path: "agent.log".to_string(),
        }
    }
}

impl AgentConfig {
    pub fn load_or_create(path: impl AsRef<Path>) -> MonitorResult<Self> {
        let path = path.as_ref();
        if path.exists() {
            let content = fs::read_to_string(path)?;
            let cfg: AgentConfig = serde_json::from_str(&content)?;
            Ok(cfg)
        } else {
            let cfg = AgentConfig::default();
            cfg.save(path)?;
            Ok(cfg)
        }
    }

    pub fn save(&self, path: impl AsRef<Path>) -> MonitorResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }
}

fn uuid_like() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos % 0xFFFFFF)
}
