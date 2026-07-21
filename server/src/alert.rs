use common::model::{alert_level_for, AlertLevel, Metrics, MetricKind};

#[derive(Debug, Clone)]
pub struct Alert {
    pub agent_id: String,
    pub kind: MetricKind,
    pub level: AlertLevel,
    pub message: String,
}

pub fn analyze(agent_id: &str, metrics: &Metrics) -> Vec<Alert> {
    let mut alerts = Vec::new();

    let checks: [(MetricKind, f32); 2] = [
        (MetricKind::Cpu, metrics.cpu_usage_percent),
        (
            MetricKind::Ram,
            if metrics.ram_total_mb > 0 {
                metrics.ram_used_mb as f32 / metrics.ram_total_mb as f32 * 100.0
            } else {
                0.0
            },
        ),
    ];

    for (kind, value) in checks {
        let level = alert_level_for(kind, value);
        match level {
            AlertLevel::Normal => {}
            AlertLevel::Warning | AlertLevel::Critical => {
                alerts.push(Alert {
                    agent_id: agent_id.to_string(),
                    kind,
                    level,
                    message: format!("{kind:?} is at {value:.1}%"),
                });
            }
        }
    }

    if let (Some(used), Some(total)) = (metrics.disk_used_gb, metrics.disk_total_gb) {
        if total > 0.0 {
            let percent = used / total * 100.0;
            let level = alert_level_for(MetricKind::Disk, percent);
            if !matches!(level, AlertLevel::Normal) {
                alerts.push(Alert {
                    agent_id: agent_id.to_string(),
                    kind: MetricKind::Disk,
                    level,
                    message: format!("Disk space is {percent:.1}% full"),
                });
            }
        }
    }

    if metrics.ping_ms.is_none() {
        alerts.push(Alert {
            agent_id: agent_id.to_string(),
            kind: MetricKind::Ping,
            level: AlertLevel::Warning,
            message: "Ping failed / the system may be unreachable".to_string(),
        });
    }

    alerts
}
