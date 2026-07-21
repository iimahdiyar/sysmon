use common::model::{AgentReport, History, Metrics, SystemInfo};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

const HISTORY_CAPACITY: usize = 120;

pub struct AgentEntry {
    pub info: SystemInfo,
    pub history: History<Metrics>,
    pub last_seen_unix: u64,
}

#[derive(Clone)]
pub struct SharedStore {
    inner: Arc<RwLock<HashMap<String, AgentEntry>>>,
    pending_stop: Arc<RwLock<HashSet<String>>>,
}

impl SharedStore {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(HashMap::new())),
            pending_stop: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn register_agent(&self, info: SystemInfo) {
        let mut map = self.inner.write().expect("lock poisoned");
        map.entry(info.agent_id.clone()).or_insert_with(|| AgentEntry {
            info,
            history: History::new(HISTORY_CAPACITY),
            last_seen_unix: 0,
        });
    }

    pub fn remove_agent(&self, agent_id: &str) {
        let mut map = self.inner.write().expect("lock poisoned");
        map.remove(agent_id);
    }

    pub fn request_stop(&self, agent_id: &str) {
        let mut pending = self.pending_stop.write().expect("lock poisoned");
        pending.insert(agent_id.to_string());
    }

    pub fn take_stop_request(&self, agent_id: &str) -> bool {
        let mut pending = self.pending_stop.write().expect("lock poisoned");
        pending.remove(agent_id)
    }

    pub fn push_report(&self, report: AgentReport) {
        let mut map = self.inner.write().expect("lock poisoned");
        if let Some(entry) = map.get_mut(&report.agent_id) {
            entry.last_seen_unix = report.metrics.timestamp_unix;
            entry.history.push(report.metrics);
        }
    }

    pub fn snapshot(&self) -> Vec<(String, SystemInfo, Option<Metrics>, u64)> {
        let map = self.inner.read().expect("lock poisoned");
        map.iter()
            .map(|(id, entry)| {
                (
                    id.clone(),
                    entry.info.clone(),
                    entry.history.latest().cloned(),
                    entry.last_seen_unix,
                )
            })
            .collect()
    }

    pub fn history_of(&self, agent_id: &str) -> Vec<Metrics> {
        let map = self.inner.read().expect("lock poisoned");
        map.get(agent_id)
            .map(|e| e.history.iter().cloned().collect())
            .unwrap_or_default()
    }
}
