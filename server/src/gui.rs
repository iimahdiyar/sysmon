use crate::storage::SharedStore;
use common::model::{alert_level_for, AlertLevel, MetricKind};
use eframe::egui;
use egui::{Color32, Frame, RichText, Rounding, Stroke, Vec2};
use egui_plot::{Line, Plot, PlotPoints};

const COLOR_OK: Color32 = Color32::from_rgb(76, 201, 129);
const COLOR_WARN: Color32 = Color32::from_rgb(240, 180, 60);
const COLOR_CRIT: Color32 = Color32::from_rgb(230, 90, 90);
const COLOR_MUTED: Color32 = Color32::from_rgb(140, 140, 150);
const COLOR_ACCENT: Color32 = Color32::from_rgb(90, 160, 240);

fn level_color(level: AlertLevel) -> Color32 {
    match level {
        AlertLevel::Normal => COLOR_OK,
        AlertLevel::Warning => COLOR_WARN,
        AlertLevel::Critical => COLOR_CRIT,
    }
}

fn ping_color(ping_ms: Option<u32>) -> Color32 {
    match ping_ms {
        None => COLOR_CRIT,
        Some(ms) if ms < 80 => COLOR_OK,
        Some(ms) if ms < 200 => COLOR_WARN,
        Some(_) => COLOR_CRIT,
    }
}

fn card_frame() -> Frame {
    Frame::none()
        .fill(Color32::from_rgb(32, 34, 40))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(52, 55, 64)))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::same(14.0))
}

pub struct DashboardApp {
    store: SharedStore,
    selected_agent: Option<String>,
    styled: bool,
}

impl DashboardApp {
    pub fn new(store: SharedStore) -> Self {
        Self {
            store,
            selected_agent: None,
            styled: false,
        }
    }

    fn apply_style(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = Vec2::new(10.0, 10.0);
        style.spacing.button_padding = Vec2::new(10.0, 6.0);
        style.visuals.window_rounding = Rounding::same(8.0);
        style.visuals.widgets.noninteractive.rounding = Rounding::same(6.0);
        style.visuals.widgets.inactive.rounding = Rounding::same(6.0);
        style.visuals.widgets.hovered.rounding = Rounding::same(6.0);
        style.visuals.widgets.active.rounding = Rounding::same(6.0);
        style.visuals.selection.bg_fill = COLOR_ACCENT.linear_multiply(0.5);
        ctx.set_style(style);
    }
}

impl eframe::App for DashboardApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.styled {
            self.apply_style(ctx);
            self.styled = true;
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        let snapshot = self.store.snapshot();

        egui::SidePanel::left("agents_list")
            .resizable(true)
            .default_width(240.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.heading("Agents");
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{}", snapshot.len()))
                                .color(COLOR_MUTED)
                                .size(14.0),
                        );
                    });
                });
                ui.separator();
                ui.add_space(4.0);

                if snapshot.is_empty() {
                    ui.label(RichText::new("No agent has connected yet...").color(COLOR_MUTED));
                }

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (id, info, latest, _last_seen) in snapshot.iter() {
                        let is_selected = self.selected_agent.as_deref() == Some(id.as_str());
                        let status_color = match latest {
                            Some(m) => level_color(alert_level_for(
                                MetricKind::Cpu,
                                m.cpu_usage_percent,
                            )),
                            None => COLOR_MUTED,
                        };

                        let mut frame = card_frame();
                        if is_selected {
                            frame = frame.stroke(Stroke::new(1.5_f32, COLOR_ACCENT));
                        }

                        let mut stop_clicked = false;
                        let mut select_clicked = false;
                        frame.show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let (rect, _) = ui.allocate_exact_size(
                                    Vec2::new(10.0, 10.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().circle_filled(rect.center(), 5.0, status_color);

                                let name_rect = ui
                                    .vertical(|ui| {
                                        ui.label(RichText::new(&info.hostname).strong());
                                        ui.label(RichText::new(id).color(COLOR_MUTED).size(11.0));
                                    })
                                    .response
                                    .rect;
                                let name_response = ui.interact(
                                    name_rect,
                                    ui.id().with(("agent_select", id.as_str())),
                                    egui::Sense::click(),
                                );
                                if name_response.clicked() {
                                    select_clicked = true;
                                }

                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        let stop_button = egui::Button::new(
                                            RichText::new("Stop")
                                                .color(Color32::WHITE)
                                                .size(11.0),
                                        )
                                        .fill(COLOR_CRIT)
                                        .rounding(Rounding::same(5.0));
                                        if ui
                                            .add_sized(Vec2::new(48.0, 22.0), stop_button)
                                            .clicked()
                                        {
                                            stop_clicked = true;
                                        }
                                    },
                                );
                            });
                        });

                        if stop_clicked {
                            self.store.request_stop(id);
                        } else if select_clicked {
                            self.selected_agent = Some(id.clone());
                        }
                        ui.add_space(6.0);
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let Some(selected_id) = self
                .selected_agent
                .clone()
                .or_else(|| snapshot.first().map(|(id, ..)| id.clone()))
            else {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.heading(RichText::new("No system available to display").color(COLOR_MUTED));
                    ui.label(
                        RichText::new("Waiting for an agent to connect...")
                            .color(COLOR_MUTED)
                            .size(13.0),
                    );
                });
                return;
            };

            let entry = snapshot.iter().find(|(id, ..)| id == &selected_id);
            let Some((id, info, latest, last_seen)) = entry else {
                ui.label("The selected agent is no longer available");
                return;
            };

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(format!("{}  —  {}", info.hostname, info.os_name));
            });
            ui.label(
                RichText::new(format!(
                    "{} ({} cores)  ·  last update (unix): {last_seen}",
                    info.cpu_name, info.cpu_cores
                ))
                .color(COLOR_MUTED)
                .size(13.0),
            );
            ui.add_space(10.0);

            match latest {
                Some(m) => {
                    let ram_percent = if m.ram_total_mb > 0 {
                        m.ram_used_mb as f32 / m.ram_total_mb as f32 * 100.0
                    } else {
                        0.0
                    };
                    let disk_percent = match (m.disk_used_gb, m.disk_total_gb) {
                        (Some(u), Some(t)) if t > 0.0 => Some(u / t * 100.0),
                        _ => None,
                    };

                    ui.columns(2, |cols| {
                        card_frame().show(&mut cols[0], |ui| {
                            let level = alert_level_for(MetricKind::Cpu, m.cpu_usage_percent);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("CPU").strong());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!("{:.1}%", m.cpu_usage_percent))
                                            .color(level_color(level))
                                            .strong(),
                                    );
                                });
                            });
                            ui.add(
                                egui::ProgressBar::new(m.cpu_usage_percent / 100.0)
                                    .fill(level_color(level))
                                    .desired_height(16.0),
                            );
                        });

                        card_frame().show(&mut cols[1], |ui| {
                            let level = alert_level_for(MetricKind::Ram, ram_percent);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("RAM").strong());
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    ui.label(
                                        RichText::new(format!(
                                            "{} / {} MB",
                                            m.ram_used_mb, m.ram_total_mb
                                        ))
                                        .color(level_color(level))
                                        .strong(),
                                    );
                                });
                            });
                            ui.add(
                                egui::ProgressBar::new(ram_percent / 100.0)
                                    .fill(level_color(level))
                                    .desired_height(16.0),
                            );
                        });
                    });

                    ui.add_space(8.0);

                    ui.columns(2, |cols| {
                        card_frame().show(&mut cols[0], |ui| {
                            ui.label(RichText::new("Disk").strong());
                            match disk_percent {
                                Some(percent) => {
                                    let level = alert_level_for(MetricKind::Disk, percent);
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            egui::Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{:.1} / {:.1} GB",
                                                        m.disk_used_gb.unwrap_or(0.0),
                                                        m.disk_total_gb.unwrap_or(0.0)
                                                    ))
                                                    .color(level_color(level))
                                                    .strong(),
                                                );
                                            },
                                        );
                                    });
                                    ui.add(
                                        egui::ProgressBar::new(percent / 100.0)
                                            .fill(level_color(level))
                                            .desired_height(16.0),
                                    );
                                }
                                None => {
                                    ui.label(RichText::new("unknown").color(COLOR_MUTED));
                                }
                            }
                        });

                        card_frame().show(&mut cols[1], |ui| {
                            ui.label(RichText::new("Network").strong());
                            ui.label(format!(
                                "\u{2193} {:.0} kbps    \u{2191} {:.0} kbps",
                                m.network_rx_kbps, m.network_tx_kbps
                            ));
                            ui.add_space(4.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Ping:").strong());
                                match m.ping_ms {
                                    Some(ms) => {
                                        ui.label(
                                            RichText::new(format!("{ms} ms"))
                                                .color(ping_color(Some(ms))),
                                        );
                                    }
                                    None => {
                                        ui.label(
                                            RichText::new("unavailable").color(ping_color(None)),
                                        );
                                    }
                                }
                            });
                            ui.label(
                                RichText::new(format!("Uptime: {} seconds", m.uptime_secs))
                                    .color(COLOR_MUTED)
                                    .size(12.0),
                            );
                        });
                    });
                }
                None => {
                    card_frame().show(ui, |ui| {
                        ui.label(
                            RichText::new("No data received yet for this agent.")
                                .color(COLOR_MUTED),
                        );
                    });
                }
            }

            ui.add_space(12.0);
            card_frame().show(ui, |ui| {
                ui.label(RichText::new("CPU history (%)").strong());
                ui.add_space(4.0);

                let history = self.store.history_of(id);
                let points: PlotPoints = history
                    .iter()
                    .enumerate()
                    .map(|(i, m)| [i as f64, m.cpu_usage_percent as f64])
                    .collect();

                Plot::new("cpu_history")
                    .height(180.0)
                    .include_y(0.0)
                    .include_y(100.0)
                    .show_axes([false, true])
                    .label_formatter(|_name, value| format!("{:.1}%", value.y))
                    .show(ui, |plot_ui| {
                        plot_ui.line(
                            Line::new(points)
                                .name("CPU %")
                                .color(COLOR_ACCENT)
                                .fill(0.0_f32),
                        );
                    });
            });
        });
    }
}

pub fn run_dashboard(store: SharedStore) -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "System & Network Monitor - Server",
        options,
        Box::new(|_cc| Box::new(DashboardApp::new(store))),
    )
}
