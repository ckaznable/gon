use anyhow::{anyhow, Result};
use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use tokio::{net::TcpStream, sync::RwLock};

use crate::{
    daemon::{
        node::Node,
        protocol::{Message, Method, Payload, Response},
    },
    notification::Notification,
};

pub struct Client {
    node: Arc<Node<Response>>,
    host: Arc<RwLock<Option<SocketAddr>>>,
}

impl Client {
    pub fn new(node: Arc<Node<Response>>, host: Arc<RwLock<Option<SocketAddr>>>) -> Self {
        Self { node, host }
    }

    pub async fn connect(&self, socket: SocketAddr) -> Result<StreamClient> {
        let stream = TcpStream::connect(socket).await?;
        Ok(StreamClient {
            node: self.node.clone(),
            host: self.host.clone(),
            stream,
        })
    }
}

pub struct StreamClient {
    node: Arc<Node<Response>>,
    host: Arc<RwLock<Option<SocketAddr>>>,
    stream: TcpStream,
}

impl StreamClient {
    async fn send(&mut self, msg: Message) -> Result<Response> {
        let res = self
            .node
            .send_and_wait_response(&mut self.stream, msg)
            .await?;

        if res.is_host_changed() {
            if let Some(Payload::Address(a, b, c, d, p)) = res.result {
                *self.host.write().await = Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), p));
                return Err(anyhow!("host changed"))
            }
        }

        Ok(res)
    }

    pub async fn ping(&mut self) -> bool {
        let Ok(res) = self
            .send(
                Message {
                    method: Method::Ping,
                    payload: Payload::Text("Ping".to_string()),
                },
            )
            .await
        else {
            return false;
        };

        if let Some(Payload::Text(s)) = res.result {
            s == "Pong"
        } else {
            false
        }
    }

    pub async fn send_notification(&mut self, notif: Notification) -> Result<()> {
        self.send(
                Message {
                    method: Method::NewNotification,
                    payload: Payload::Notification(notif),
                }
            )
            .await?;

        Ok(())
    }
}
