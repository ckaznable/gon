use anyhow::Result;
use std::{net::SocketAddr, sync::Arc};

use tokio::net::TcpStream;

use crate::{daemon::{node::Node, protocol::{Message, Method, Payload, Response}}, notification::Notification};

pub struct Client {
    node: Arc<Node<Response>>,
}

impl Client {
    pub fn new(node: Arc<Node<Response>>) -> Self {
        Self {
            node
        }
    }

    pub async fn connect(&self, socket: SocketAddr) -> Result<StreamClient> {
        let stream = TcpStream::connect(socket).await?;
        Ok(StreamClient {
            node: self.node.clone(),
            stream,
        })
    }
}

pub struct StreamClient {
    node: Arc<Node<Response>>,
    stream: TcpStream,
}

impl StreamClient {
    pub async fn ping(&mut self) -> bool {
        let Ok(res) = self.node.send_and_wait_response(&mut self.stream, Message {
            method: Method::Ping,
            payload: Payload::Text("Ping".to_string()),
        }).await else {
            return false;
        };

        if let Some(Payload::Text(s)) = res.result {
            s == "Pong"
        } else {
            false
        }
    }

    pub async fn send_notification(&mut self, notif: Notification) -> Result<()> {
        self.node.send(&mut self.stream, Response::success(Payload::Notification(notif))).await
    }
}
