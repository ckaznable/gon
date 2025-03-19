use std::{
    collections::HashMap, net::SocketAddr, sync::Arc,
};

use anyhow::{anyhow, Result};
use chacha20poly1305::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    ChaCha20Poly1305, Nonce
};
use serde::{Deserialize, Serialize};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::{TcpListener, TcpStream}, sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}, task::JoinHandle};

use crate::daemon::misc::get_preferred_local_ip;

use super::protocol::Message;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Payload {
    Text(String),
    Number(u64),
    List(Vec<String>),
    Dictionary(HashMap<String, String>),
    Raw(Vec<u8>),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ResponseStatus {
    Success,
    Faild,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    status: ResponseStatus,
    result: Option<Payload>,
}

impl Response {
    pub fn sucess(payload: Payload) -> Self {
        Self {
            status: ResponseStatus::Success,
            result: Some(payload),
        }
    }

    pub fn empty() -> Self {
        Self {
            status: ResponseStatus::Success,
            result: None,
        }
    }

    pub fn faild() -> Self {
        Self {
            status: ResponseStatus::Faild,
            result: None,
        }
    }
}

pub struct Node {
    socket: Option<TcpListener>,
    codec: Arc<NodeMessageCodec>,
}

impl Node {
    pub async fn new(password: &[u8]) -> Result<(Self, SocketAddr)> {
        let local_addr = get_preferred_local_ip()?;
        let socket = TcpListener::bind(format!("{}:0", local_addr)).await?;
        let addr = socket.local_addr()?;

        let codec = NodeMessageCodec::new(password)?;

        let node = Self {
            codec: Arc::new(codec),
            socket: Some(socket),
        };

        Ok((node, addr))
    }

    pub async fn send(&self, mut stream: TcpStream, message: Message) -> Result<Response> {
        let serialized = self.codec.encode(&message)?;

        // Send length prefix (4 bytes) followed by serialized data
        stream.write_all(&(serialized.len() as u32).to_be_bytes()).await?;
        stream.write_all(&serialized).await?;

        // Read response length
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        // Read response data
        let mut buffer = vec![0u8; response_len];
        stream.read_exact(&mut buffer).await?;

        // Deserialize response
        let response: Response = self.codec.decode(&buffer)?;
        Ok(response)
    }

    #[allow(clippy::let_underscore_future)]
    pub async fn listen(&mut self) -> Result<UnboundedReceiver<(TcpStream, Message)>> {
        let listener = self.socket.take().unwrap();
        let codec = self.codec.clone();
        let (tx, rx) = unbounded_channel();

        let _: JoinHandle<Result<()>> = tokio::spawn(async move {
            loop {
                let (socket, addr) = listener.accept().await?;
                println!("New client connected: {}", addr);

                let codec = codec.clone();
                let tx = tx.clone();

                // Spawn a new task for each client
                tokio::spawn(async move {
                    if let Err(e) = Self::handle_client(socket, codec, tx).await {
                        eprintln!("Error handling client: {}", e);
                    }
                });
            }
        });

        Ok(rx)
    }

    async fn handle_client(mut stream: TcpStream, codec: Arc<NodeMessageCodec>, tx: UnboundedSender<(TcpStream, Message)>) -> Result<()> {
        // Read message length (4 bytes)
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let msg_len = u32::from_be_bytes(len_bytes) as usize;

        // Read message content based on length
        let mut buffer = vec![0u8; msg_len];
        stream.read_exact(&mut buffer).await?; 

        // Deserialize data to Message
        let message: Message = codec.decode(&buffer)?;
        tx.send((stream, message))?;

        Ok(())
    }
}

struct NodeMessageCodec {
    cipher: ChaCha20Poly1305,
    nonce: Nonce,
}

impl NodeMessageCodec {
    fn new(password: &[u8]) -> Result<Self> {
        let cipher = ChaCha20Poly1305::new_from_slice(password).map_err(|_| anyhow!("cipher generate faild"))?;
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        Ok(Self {
            cipher,
            nonce,
        })
    }

    fn encode<T>(&self, message: &T) -> Result<Vec<u8>>
    where
        T: for<'de> serde::Serialize
    {
        let raw = serde_cbor::to_vec(message)?;
        let data = self.encrypt(&raw)?;
        Ok(data)
    }

    fn decode<T>(&self, data: &[u8]) -> Result<T> 
    where
        T: for<'de> serde::Deserialize<'de>
    {
        let data = self.decrypt(data)?;
        let message: T = serde_cbor::from_slice(&data)?;
        Ok(message)
    }

    fn encrypt(&self, msg: &[u8]) -> Result<Vec<u8>> {
        self.cipher.encrypt(&self.nonce, msg).map_err(|_| anyhow!("encrypt fail"))
    }

    fn decrypt(&self, msg: &[u8]) -> Result<Vec<u8>> {
        self.cipher.decrypt(&self.nonce, msg).map_err(|_| anyhow!("decrypt fail"))
    }
}
