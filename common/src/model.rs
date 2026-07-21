use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertLevel {
    Normal,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum MetricKind {
    Cpu,
    Ram,
    Disk,
    Network,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub cpu_usage_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk_used_gb: Option<f32>,
    pub disk_total_gb: Option<f32>,
    pub network_rx_kbps: f64,
    pub network_tx_kbps: f64,
    pub ping_ms: Option<u32>,
    pub uptime_secs: u64,
    pub timestamp_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    pub agent_id: String,
    pub hostname: String,
    pub os_name: String,
    pub os_version: String,
    pub cpu_name: String,
    pub cpu_cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentReport {
    pub agent_id: String,
    pub metrics: Metrics,
}

pub struct History<T> {
    buffer: VecDeque<T>,
    capacity: usize,
}

impl<T> History<T> {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, item: T) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(item);
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.buffer.iter()
    }

    pub fn latest(&self) -> Option<&T> {
        self.buffer.back()
    }

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

pub fn alert_level_for(kind: MetricKind, value: f32) -> AlertLevel {
    match (kind, value) {
        (MetricKind::Cpu, v) if v >= 90.0 => AlertLevel::Critical,
        (MetricKind::Cpu, v) if v >= 75.0 => AlertLevel::Warning,
        (MetricKind::Ram, v) if v >= 90.0 => AlertLevel::Critical,
        (MetricKind::Ram, v) if v >= 75.0 => AlertLevel::Warning,
        (MetricKind::Disk, v) if v >= 95.0 => AlertLevel::Critical,
        (MetricKind::Disk, v) if v >= 85.0 => AlertLevel::Warning,
        _ => AlertLevel::Normal,
    }
}
