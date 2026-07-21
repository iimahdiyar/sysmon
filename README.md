# مانیتور سیستم و شبکه توزیع‌شده (Rust)

پیاده‌سازی پروپوزال «مانیتور سیستم و شبکه» به‌صورت معماری توزیع‌شده:
چند کامپیوتر هرکدام یک **Agent** اجرا می‌کنند که متریک‌های سیستم را
جمع‌آوری کرده و به یک **Server** مرکزی می‌فرستند. سرور داده‌ی همه‌ی
Agentها را در یک Dashboard گرافیکی واحد نمایش می‌دهد.

## ساختار پروژه (Workspace)

```
sysmon/
├── common/   # مدل‌های داده، خطاها و پروتکل مشترک بین agent و server
├── agent/    # روی هر کامپیوتری که باید مانیتور شود اجرا می‌شود
└── server/   # سرور مرکزی + GUI برای نمایش همه‌ی سیستم‌ها
```

## اجرا

```bash
# سرور مرکزی را ابتدا اجرا کنید (روی یک کامپیوتر)
cd server && cargo run

# روی هر کامپیوتری که می‌خواهید مانیتور شود، Agent را اجرا کنید
# (اگر روی همان دستگاه server باشد نیازی به تغییر config نیست)
cd agent && cargo run
```

اولین اجرا فایل‌های `server_config.json` و `agent_config.json` را
می‌سازد. برای اتصال یک Agent به سرور روی کامپیوتر دیگر، مقدار
`server_addr` در `agent_config.json` را به آی‌پی سرور تغییر دهید،
مثلاً: `"server_addr": "192.168.1.10:9000"`.

## نگاشت مفاهیم Rust به کد پروژه

| مفهوم | محل استفاده |
|---|---|
| Struct / Enum | `common/src/model.rs` (`Metrics`, `SystemInfo`, `AlertLevel`, `MetricKind`) |
| Trait | `agent/src/collector.rs` → `trait MetricCollector` |
| Generics | `common/src/model.rs` → `struct History<T>` |
| Ownership / Borrowing | استفاده از `&self`، `clone()` هدفمند در `storage.rs` و `gui.rs` |
| Option | `Metrics.disk_used_gb`, `Metrics.ping_ms` |
| Result / Error Handling | `common/src/error.rs` → `MonitorError` + عملگر `?` در کل پروژه |
| Threading | `agent/src/main.rs` → یک thread مستقل برای هر collector |
| mpsc Channel | `agent/src/main.rs` → `std::sync::mpsc` بین collectorها و aggregator |
| Async / Await | `agent/src/network.rs`, `server/src/network.rs` (Tokio) |
| Pattern Matching | `server/src/alert.rs`, `agent/src/main.rs` (روی `CollectedValue`) |
| String slice / casting | فرمت‌دهی پیام‌ها و تبدیل واحدها (`as f32`, `as u64`) در `collector.rs` |
| File Handling | `agent/src/logger.rs`, `config.rs` (خواندن/نوشتن JSON) |
| Serde | سریالایز/دیسریالایز `Message` روی TCP و فایل‌های کانفیگ |
| Modules | جداسازی `collector` / `model` / `gui` / `logger` / `config` / `network` |
| Vec / VecDeque / Iterator | `History<T>` و `fold`/`map` در collectorها |
| GUI Event Loop | `server/src/gui.rs` با `eframe`/`egui` |

## معماری ارتباطی

هر پیام بین Agent و Server به‌صورت JSON سریالایز شده و طول آن (۴ بایت،
big-endian) قبل از بدنه پیام فرستاده می‌شود (length-prefixed framing)
تا بشود چند پیام پشت‌سرهم را روی یک اتصال TCP بدون ابهام خواند.

سه نوع پیام وجود دارد:
- `Register(SystemInfo)`: یک‌بار هنگام اتصال Agent
- `Report(AgentReport)`: به‌صورت دوره‌ای (هر `interval_secs` ثانیه)
- `Ack`: تایید دریافت از سمت سرور

سرور برای هر Agent یک async task مجزا اجرا می‌کند، بنابراین چند
کامپیوتر می‌توانند هم‌زمان و مستقل از هم به سرور متصل باشند و داده
بفرستند.

## نکته درباره‌ی build

این پروژه به کتابخانه‌های `sysinfo`، `tokio`، `serde` و `eframe/egui`
نیاز دارد که هنگام اولین `cargo build` دانلود می‌شوند (نیاز به اتصال
اینترنت دارد).
