
mod collector;
mod config;
mod logger;
mod network;

use collector::{
    CollectedValue, CpuCollector, DiskCollector, MetricCollector, NetworkCollector,
    PingCollector, RamCollector,
};
use common::model::{AgentReport, Metrics, SystemInfo};
use config::AgentConfig;
use logger::FileLogger;
use network::ServerConnection;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use sysinfo::System;

struct CollectorMessage {
    source: &'static str,
    value: CollectedValue,
}

#[derive(Default, Clone)]
struct SharedState {
    cpu: f32,
    ram_used_mb: u64,
    ram_total_mb: u64,
    disk_used_gb: Option<f32>,
    disk_total_gb: Option<f32>,
    net_rx_kbps: f64,
    net_tx_kbps: f64,
    ping_ms: Option<u32>,
}

fn spawn_collector_thread(
    mut collector: Box<dyn MetricCollector>,
    source: &'static str,
    interval: Duration,
    tx: std_mpsc::Sender<CollectorMessage>,
    logger: Arc<FileLogger>,
) {
    thread::spawn(move || loop {
        match collector.collect() {
            Ok(value) => {
                if tx.send(CollectorMessage { source, value }).is_err() {
                    break;
                }
            }
            Err(e) => {
                let _ = logger.log(&format!("collector '{}' failed: {e}", collector.name()));
            }
        }
        thread::sleep(interval);
    });
}

fn build_system_info(agent_id: &str) -> SystemInfo {
    let mut sys = System::new_all();
    sys.refresh_all();
    SystemInfo {
        agent_id: agent_id.to_string(),
        hostname: System::host_name().unwrap_or_else(|| "unknown".to_string()),
        os_name: System::name().unwrap_or_else(|| "unknown".to_string()),
        os_version: System::os_version().unwrap_or_else(|| "unknown".to_string()),
        cpu_name: sys
            .cpus()
            .first()
            .map(|c| c.brand().to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        cpu_cores: sys.cpus().len(),
    }
}

#[tokio::main]
async fn main() {
    let config = AgentConfig::load_or_create("agent_config.json")
        .expect("Failed to read or create agent_config.json");
    let logger = Arc::new(FileLogger::new(config.log_path.clone()));
    let _ = logger.log(&format!("agent '{}' is starting", config.agent_id));

    let interval = Duration::from_secs(config.interval_secs.max(1));
    let shared = Arc::new(Mutex::new(SharedState::default()));

    let (tx, rx) = std_mpsc::channel::<CollectorMessage>();

    spawn_collector_thread(Box::new(CpuCollector::new()), "cpu", interval, tx.clone(), logger.clone());
    spawn_collector_thread(Box::new(RamCollector::new()), "ram", interval, tx.clone(), logger.clone());
    spawn_collector_thread(Box::new(DiskCollector), "disk", interval * 2, tx.clone(), logger.clone());
    spawn_collector_thread(
        Box::new(NetworkCollector::new()),
        "network",
        interval,
        tx.clone(),
        logger.clone(),
    );
    spawn_collector_thread(
        Box::new(PingCollector::new(config.ping_target.clone())),
        "ping",
        interval,
        tx,
        logger.clone(),
    );

    {
        let shared = shared.clone();
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let mut state = shared.lock().expect("mutex poisoned");
                match msg.value {
                    CollectedValue::Cpu(v) => state.cpu = v,
                    CollectedValue::Ram { used_mb, total_mb } => {
                        state.ram_used_mb = used_mb;
                        state.ram_total_mb = total_mb;
                    }
                    CollectedValue::Disk { used_gb, total_gb } => {
                        state.disk_used_gb = Some(used_gb);
                        state.disk_total_gb = Some(total_gb);
                    }
                    CollectedValue::Network { rx_kbps, tx_kbps } => {
                        state.net_rx_kbps = rx_kbps;
                        state.net_tx_kbps = tx_kbps;
                    }
                    CollectedValue::Ping(ms) => state.ping_ms = ms,
                }
                drop(state);
                let _ = msg.source;
            }
        });
    }

    let sys_info = build_system_info(&config.agent_id);
    let mut conn = match ServerConnection::connect(&config.server_addr).await {
        Ok(c) => c,
        Err(e) => {
            let _ = logger.log(&format!("Initial connection to server failed: {e}"));
            eprintln!("Cannot connect to server {}: {e}", config.server_addr);
            return;
        }
    };

    if let Err(e) = conn.register(sys_info).await {
        let _ = logger.log(&format!("Registration with server failed: {e}"));
        eprintln!("Registration failed: {e}");
        return;
    }
    let _ = logger.log("Registration with the central server succeeded");

    let mut ticker = tokio::time::interval(interval);
    loop {
        ticker.tick().await;
        let snapshot = {
            let state = shared.lock().expect("mutex poisoned");
            state.clone()
        };

        let metrics = Metrics {
            cpu_usage_percent: snapshot.cpu,
            ram_used_mb: snapshot.ram_used_mb,
            ram_total_mb: snapshot.ram_total_mb,
            disk_used_gb: snapshot.disk_used_gb,
            disk_total_gb: snapshot.disk_total_gb,
            network_rx_kbps: snapshot.net_rx_kbps,
            network_tx_kbps: snapshot.net_tx_kbps,
            ping_ms: snapshot.ping_ms,
            uptime_secs: System::uptime(),
            timestamp_unix: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        };

        let report = AgentReport {
            agent_id: config.agent_id.clone(),
            metrics,
        };

        match conn.report(report).await {
            Ok(true) => {
                let _ = logger.log("Received stop command from server, halting monitoring");
                println!("Received stop command from server, halting monitoring");
                return;
            }
            Ok(false) => {}
            Err(e) => {
                let _ = logger.log(&format!("Failed to send report, attempting to reconnect: {e}"));
                match ServerConnection::connect(&config.server_addr).await {
                    Ok(new_conn) => conn = new_conn,
                    Err(e2) => {
                        let _ = logger.log(&format!("Reconnection failed: {e2}"));
                    }
                }
            }
        }
    }
}
