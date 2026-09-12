use russh::client;
use russh::ChannelMsg;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct ClientHandler {
    pub device_id: String,
}

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        let _ = &self.device_id;
        // Accept all hosts for a personal tunnel manager (TOFU can be added later).
        Ok(true)
    }
}

/// Bridge a TCP socket with an SSH direct-tcpip channel.
pub async fn bridge_tcp_channel(
    mut tcp: tokio::net::TcpStream,
    mut channel: russh::Channel<client::Msg>,
) {
    let mut buf = vec![0u8; 32 * 1024];
    loop {
        tokio::select! {
            biased;
            msg = channel.wait() => {
                match msg {
                    Some(ChannelMsg::Data { ref data }) => {
                        if tcp.write_all(data).await.is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof) | None => break,
                    Some(ChannelMsg::Close) => break,
                    _ => {}
                }
            }
            read = tcp.read(&mut buf) => {
                match read {
                    Ok(0) => {
                        let _ = channel.eof().await;
                        break;
                    }
                    Ok(n) => {
                        if channel.data(&buf[..n]).await.is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        }
    }
    let _ = channel.close().await;
}
