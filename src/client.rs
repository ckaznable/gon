use anyhow::{anyhow, Result};
use std::{net::{IpAddr, Ipv4Addr, SocketAddr}, sync::Arc};

use tokio::{net::TcpStream, sync::Mutex};

use crate::{
    daemon::{
        node::Node,
        protocol::{Message, Method, Payload, Response},
    },
    notification::Notification, AppMode,
};

pub struct Client {
    node: Arc<Node<Response>>,
    host: Arc<Mutex<AppMode<SocketAddr>>>,
}

impl Client {
    pub fn new(node: Arc<Node<Response>>, host: Arc<Mutex<AppMode<SocketAddr>>>) -> Self {
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

    pub fn handle(&self) -> MessageHandler {
        MessageHandler {
            node: self.node.clone(),
            host: self.host.clone(),
        }
    }
}

pub struct StreamClient {
    node: Arc<Node<Response>>,
    host: Arc<Mutex<AppMode<SocketAddr>>>,
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
                println!("host changed to {}.{}.{}.{}:{}", a, b, c, d, p);
                *self.host.lock().await = AppMode::Client(Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), p)));
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

    pub async fn get_addr(&mut self) -> Result<()> {
        let res = self.send(Message {
            method: Method::GetHost,
            payload: Payload::Empty,
        }).await?;

        if res.is_failed() {
            return Err(anyhow!("failed to get addr"));
        }

        if let Some(Payload::Address(a, b, c, d, p)) = res.result {
            println!("get host addr: {}.{}.{}.{}:{}", a, b, c, d, p);
            *self.host.lock().await = AppMode::Client(Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), p)));
        }

        Ok(())
    }

    pub async fn im_host(&mut self) -> Result<()> {
        let addr = self.node.get_addr_v4().ok_or(anyhow!("failed to get addr"))?;
        self.send(Message {
            method: Method::ImHost,
            payload: Payload::Address(addr.0, addr.1, addr.2, addr.3, addr.4),
        }).await?;

        Ok(())
    }
}

pub struct MessageHandler {
    node: Arc<Node<Response>>,
    host: Arc<Mutex<AppMode<SocketAddr>>>,
}

impl MessageHandler {
    pub async fn handle(&self, msg: Message) -> Response {
        let res: Result<Response> = {
            match msg.method {
                Method::Ping => {
                    Ok(Response::success(Payload::Text("Pong".to_string())))
                },
                Method::NewNotification => {
                    println!("new notification {:?}", msg.payload);
                    if self.host.lock().await.is_host() {
                        if let Payload::Notification(notif) = msg.payload {
                            let _ = notify_rust::Notification::new()
                                .summary(&notif.title)
                                .body(&notif.message)
                                .show();
                        }
                    }

                    Ok(Response::empty())
                },
                Method::GetHost => {
                    let mode = self.host.lock().await;
                    if let Some(addr) = mode.get_host() {
                        if let IpAddr::V4(ipv4) = addr.ip() {
                            let ip = ipv4.octets();
                            Ok(Response::success(Payload::Address(ip[0], ip[1], ip[2], ip[3], addr.port())))
                        } else {
                            Ok(Response::failed())
                        }
                    } else if let Some((a, b, c, d, p)) = self.node.get_addr_v4() {
                        Ok(Response::success(Payload::Address(a, b, c, d, p)))
                    } else {
                        Ok(Response::failed())
                    }
                },
                Method::ImHost => {
                    let mut host = self.host.lock().await;
                    if let Some(addr) = host.get_host() {
                        println!("change host to client");
                        *host = AppMode::Client(Some(*addr));
                    };

                    Ok(Response::empty())
                },
                _ => Ok(Response::empty()),
            }
        };

        res.unwrap_or(Response::failed())
    }
}