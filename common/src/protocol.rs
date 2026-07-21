
use crate::error::MonitorResult;
use crate::model::{AgentReport, SystemInfo};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    Register(SystemInfo),
    Report(AgentReport),
    Ack,
    Stop,
}

impl Message {
    pub fn to_bytes(&self) -> MonitorResult<Vec<u8>> {
        Ok(serde_json::to_vec(self)?)
    }

    pub fn from_bytes(bytes: &[u8]) -> MonitorResult<Self> {
        Ok(serde_json::from_slice(bytes)?)
    }
}
