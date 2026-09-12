use super::handler::{bridge_tcp_channel, ClientHandler};
use crate::models::PortMapping;
use anyhow::{Context, Result};
use russh::client;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::Mutex;

pub type SharedSession = Arc<Mutex<client::Handle<ClientHandler>>>;

pub async fn run_local_forward(session: SharedSession, mapping: PortMapping) -> Result<()> {
    let bind = format!("127.0.0.1:{}", mapping.local_port);
    let listener = TcpListener::bind(&bind)
        .await
        .with_context(|| format!("bind local port {}", mapping.local_port))?;

    log::info!(
        "listening on {} -> {}:{}",
        bind,
        mapping.remote_host,
        mapping.remote_port
    );

    loop {
        let (tcp, peer) = listener.accept().await.context("accept local connection")?;
        log::debug!("accepted {} for mapping {}", peer, mapping.id);

        {
            let guard = session.lock().await;
            if guard.is_closed() {
                anyhow::bail!("ssh session closed");
            }
        }

        let session = session.clone();
        let remote_host = mapping.remote_host.clone();
        let remote_port = mapping.remote_port as u32;

        tokio::spawn(async move {
            let open_result = {
                let guard = session.lock().await;
                guard
                    .channel_open_direct_tcpip(
                        remote_host,
                        remote_port,
                        "127.0.0.1",
                        peer.port() as u32,
                    )
                    .await
            };
            match open_result {
                Ok(channel) => bridge_tcp_channel(tcp, channel).await,
                Err(e) => log::warn!("direct-tcpip open failed: {:#}", e),
            }
        });
    }
}
