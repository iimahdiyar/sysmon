
use common::error::{MonitorError, MonitorResult};
use common::model::{AgentReport, SystemInfo};
use common::protocol::Message;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub struct ServerConnection {
    stream: TcpStream,
}

impl ServerConnection {
    pub async fn connect(addr: &str) -> MonitorResult<Self> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| MonitorError::NetworkError(format!("Failed to connect to server: {e}")))?;
        Ok(Self { stream })
    }

    pub async fn send(&mut self, msg: &Message) -> MonitorResult<()> {
        let bytes = msg.to_bytes()?;
        let len = bytes.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await?;
        self.stream.write_all(&bytes).await?;
        Ok(())
    }

    pub async fn recv(&mut self) -> MonitorResult<Message> {
        let mut len_buf = [0u8; 4];
        self.stream.read_exact(&mut len_buf).await?;
        let len = u32::from_be_bytes(len_buf) as usize;

        let mut body = vec![0u8; len];
        self.stream.read_exact(&mut body).await?;
        Message::from_bytes(&body)
    }

    pub async fn register(&mut self, info: SystemInfo) -> MonitorResult<()> {
        self.send(&Message::Register(info)).await?;
        match self.recv().await? {
            Message::Ack => Ok(()),
            other => Err(MonitorError::ProtocolError(format!(
                "Unexpected response during registration: {other:?}"
            ))),
        }
    }

    pub async fn report(&mut self, report: AgentReport) -> MonitorResult<bool> {
        self.send(&Message::Report(report)).await?;
        match self.recv().await? {
            Message::Ack => Ok(false),
            Message::Stop => Ok(true),
            other => Err(MonitorError::ProtocolError(format!(
                "Unexpected response while sending report: {other:?}"
            ))),
        }
    }
}

