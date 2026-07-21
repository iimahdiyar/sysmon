use common::error::{MonitorError, MonitorResult};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System};

#[derive(Debug, Clone)]
pub enum CollectedValue {
    Cpu(f32),
    Ram { used_mb: u64, total_mb: u64 },
    Disk { used_gb: f32, total_gb: f32 },
    Network { rx_kbps: f64, tx_kbps: f64 },
    Ping(Option<u32>),
}

pub trait MetricCollector: Send {
    fn name(&self) -> &str;

    fn collect(&mut self) -> MonitorResult<CollectedValue>;
}

pub struct CpuCollector {
    sys: System,
}

impl CpuCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_cpu_usage();
        Self { sys }
    }
}

impl MetricCollector for CpuCollector {
    fn name(&self) -> &str {
        "cpu"
    }

    fn collect(&mut self) -> MonitorResult<CollectedValue> {
        self.sys.refresh_cpu_usage();
        let cpus = self.sys.cpus();
        let usage = if cpus.is_empty() {
            0.0
        } else {
            cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
        };
        Ok(CollectedValue::Cpu(usage))
    }
}

pub struct RamCollector {
    sys: System,
}

impl RamCollector {
    pub fn new() -> Self {
        let mut sys = System::new();
        sys.refresh_memory();
        Self { sys }
    }
}

impl MetricCollector for RamCollector {
    fn name(&self) -> &str {
        "ram"
    }

    fn collect(&mut self) -> MonitorResult<CollectedValue> {
        self.sys.refresh_memory();
        Ok(CollectedValue::Ram {
            used_mb: self.sys.used_memory() / 1024 / 1024,
            total_mb: self.sys.total_memory() / 1024 / 1024,
        })
    }
}

pub struct DiskCollector;

impl MetricCollector for DiskCollector {
    fn name(&self) -> &str {
        "disk"
    }

    fn collect(&mut self) -> MonitorResult<CollectedValue> {
        let disks = Disks::new_with_refreshed_list();
        let (total, used) = disks.iter().fold((0u64, 0u64), |(t, u), d| {
            let total_space = d.total_space();
            let free_space = d.available_space();
            (t + total_space, u + (total_space - free_space))
        });

        if total == 0 {
            return Err(MonitorError::CollectionFailed(
                "No disk found".to_string(),
            ));
        }

        const GB: f32 = 1024.0 * 1024.0 * 1024.0;
        Ok(CollectedValue::Disk {
            used_gb: used as f32 / GB,
            total_gb: total as f32 / GB,
        })
    }
}

pub struct NetworkCollector {
    networks: Networks,
    last_sample: Instant,
}

impl NetworkCollector {
    pub fn new() -> Self {
        Self {
            networks: Networks::new_with_refreshed_list(),
            last_sample: Instant::now(),
        }
    }
}

impl MetricCollector for NetworkCollector {
    fn name(&self) -> &str {
        "network"
    }

    fn collect(&mut self) -> MonitorResult<CollectedValue> {
        self.networks.refresh();
        let elapsed = self.last_sample.elapsed().as_secs_f64().max(0.001);
        self.last_sample = Instant::now();

        let (rx_bytes, tx_bytes) = self
            .networks
            .iter()
            .fold((0u64, 0u64), |(rx, tx), (_name, data)| {
                (rx + data.received(), tx + data.transmitted())
            });

        Ok(CollectedValue::Network {
            rx_kbps: (rx_bytes as f64 * 8.0 / 1024.0) / elapsed,
            tx_kbps: (tx_bytes as f64 * 8.0 / 1024.0) / elapsed,
        })
    }
}

pub struct PingCollector {
    target: String,
}

impl PingCollector {
    pub fn new(target: impl Into<String>) -> Self {
        Self {
            target: target.into(),
        }
    }
}

impl MetricCollector for PingCollector {
    fn name(&self) -> &str {
        "ping"
    }

    fn collect(&mut self) -> MonitorResult<CollectedValue> {
        let addr = match self.target.to_socket_addrs() {
            Ok(mut addrs) => addrs.next(),
            Err(_) => None,
        };

        let Some(addr) = addr else {
            return Ok(CollectedValue::Ping(None));
        };

        let start = Instant::now();
        let result = TcpStream::connect_timeout(&addr, Duration::from_secs(2));
        match result {
            Ok(_) => Ok(CollectedValue::Ping(Some(start.elapsed().as_millis() as u32))),
            Err(_) => Ok(CollectedValue::Ping(None)),
        }
    }
}
