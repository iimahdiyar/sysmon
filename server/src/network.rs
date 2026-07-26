use crate::alert::analyze;
use crate::storage::SharedStore;
use common::error::{MonitorError, MonitorResult};
use common::protocol::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

pub async fn run_server(addr: &str, store: SharedStore) -> MonitorResult<()> {
    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| MonitorError::NetworkError(format!("Failed to bind on {addr}: {e}")))?;

    println!("Central server listening on {addr}...");

    loop {
        let (socket, peer) = match listener.accept().await {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("Error accepting new connection: {e}");
                continue;
            }
        };

        let store = store.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(socket, store).await {
                eprintln!("Connection {peer} closed with an error: {e}");
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, store: SharedStore) -> MonitorResult<()> {
    let mut agent_id = String::new();

    loop {
        let msg = match read_message(&mut socket).await {
            Ok(m) => m,
            Err(_) => break,
        };

        match msg {
            Message::Register(info) => {
                agent_id = info.agent_id.clone();
                println!("New agent registered: {} ({})", info.agent_id, info.hostname);
                store.register_agent(info);
                send_message(&mut socket, &Message::Ack).await?;
            }
            Message::Report(report) => {
                let rid = report.agent_id.clone();
                let alerts = analyze(&report.agent_id, &report.metrics);
                for alert in &alerts {
                    println!(
                        "[ALERT:{:?}:{:?}] {} -> {}",
                        alert.level, alert.kind, alert.agent_id, alert.message
                    );
                }
                store.push_report(report);

                if store.take_stop_request(&rid) {
                    send_message(&mut socket, &Message::Stop).await?;
                    store.remove_agent(&rid);
                    println!("Sent stop command to agent '{rid}'");
                    break;
                }
                send_message(&mut socket, &Message::Ack).await?;
            }
            Message::Ack | Message::Stop => {}
        }
    }

    if !agent_id.is_empty() {
        println!("Agent '{agent_id}' connection closed");
    }
    Ok(())
}

async fn read_message(socket: &mut TcpStream) -> MonitorResult<Message> {
    let mut len_buf = [0u8; 4];
    socket.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf) as usize;

    let mut body = vec![0u8; len];
    socket.read_exact(&mut body).await?;
    Message::from_bytes(&body)
}

async fn send_message(socket: &mut TcpStream, msg: &Message) -> MonitorResult<()> {
    let bytes = msg.to_bytes()?;
    let len = bytes.len() as u32;
    socket.write_all(&len.to_be_bytes()).await?;
    socket.write_all(&bytes).await?;
    Ok(())
}
