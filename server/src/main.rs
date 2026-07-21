mod alert;
mod config;
mod gui;
mod network;
mod storage;

use config::ServerConfig;
use storage::SharedStore;

fn main() {
    let cfg = ServerConfig::load_or_create("server_config.json")
        .expect("Failed to read or create server_config.json");

    let store = SharedStore::new();

    {
        let store = store.clone();
        let listen_addr = cfg.listen_addr.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
            rt.block_on(async move {
                if let Err(e) = network::run_server(&listen_addr, store).await {
                    eprintln!("Server stopped with an error: {e}");
                }
            });
        });
    }

    if let Err(e) = gui::run_dashboard(store) {
        eprintln!("Error running the GUI: {e}");
    }
}
