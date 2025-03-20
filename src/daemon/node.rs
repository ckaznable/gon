use std::{marker::PhantomData, net::SocketAddr, sync::Arc};

use anyhow::{Result, anyhow};
use chacha20poly1305::{
    ChaCha20Poly1305, Nonce,
    aead::{Aead, AeadCore, KeyInit, OsRng},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel},
    task::JoinHandle,
};

use crate::daemon::misc::get_preferred_local_ip;

use super::protocol::Message;

pub struct Node<R> {
    socket: Option<TcpListener>,
    codec: Arc<NodeMessageCodec>,
    _phamtom_response: PhantomData<R>,
}

impl<R> Node<R> {
    pub async fn new(password: &[u8]) -> Result<(Self, SocketAddr)> {
        let local_addr = get_preferred_local_ip()?;
        let socket = TcpListener::bind(format!("{}:0", local_addr)).await?;
        let addr = socket.local_addr()?;

        let codec = NodeMessageCodec::new(password)?;

        let node = Self {
            codec: Arc::new(codec),
            socket: Some(socket),
            _phamtom_response: PhantomData,
        };

        Ok((node, addr))
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

    async fn handle_client(
        mut stream: TcpStream,
        codec: Arc<NodeMessageCodec>,
        tx: UnboundedSender<(TcpStream, Message)>,
    ) -> Result<()> {
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

impl<R> Node<R>
where
    R: for<'de> serde::Deserialize<'de> + serde::Serialize,
{
    pub async fn reply(&self, stream: &mut TcpStream, data: R) -> Result<()> {
        self.send(stream, data).await
    }

    pub async fn send_and_wait_response<M>(&self, stream: &mut TcpStream, data: M) -> Result<R>
    where
        M: for<'de> serde::Serialize,
    {
        self.send(stream, data).await?;

        // Read response length
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await?;
        let response_len = u32::from_be_bytes(len_bytes) as usize;

        // Read response data
        let mut buffer = vec![0u8; response_len];
        stream.read_exact(&mut buffer).await?;

        // Deserialize response
        let response: R = self.codec.decode(&buffer)?;
        Ok(response)
    }

    pub async fn send<T>(&self, stream: &mut TcpStream, data: T) -> Result<()>
    where
        T: for<'de> serde::Serialize
    {
        let serialized = self.codec.encode(&data)?;

        // Send length prefix (4 bytes) followed by serialized data
        stream
            .write_all(&(serialized.len() as u32).to_be_bytes())
            .await?;
        stream.write_all(&serialized).await?;
        Ok(())
    }
}

struct NodeMessageCodec {
    cipher: ChaCha20Poly1305,
    nonce: Nonce,
}

impl NodeMessageCodec {
    fn new(password: &[u8]) -> Result<Self> {
        let cipher = ChaCha20Poly1305::new_from_slice(password)
            .unwrap_or_else(|_| {
                eprintln!("generate cipher failed");
                let key = ChaCha20Poly1305::generate_key(&mut OsRng);
                ChaCha20Poly1305::new(&key)
            });
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        Ok(Self { cipher, nonce })
    }

    fn encode<T>(&self, message: &T) -> Result<Vec<u8>>
    where
        T: for<'de> serde::Serialize,
    {
        let raw = serde_cbor::to_vec(message)?;
        let data = self.encrypt(&raw)?;
        Ok(data)
    }

    fn decode<T>(&self, data: &[u8]) -> Result<T>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        let data = self.decrypt(data)?;
        let message: T = serde_cbor::from_slice(&data)?;
        Ok(message)
    }

    fn encrypt(&self, msg: &[u8]) -> Result<Vec<u8>> {
        self.cipher
            .encrypt(&self.nonce, msg)
        .map_err(|_| anyhow!("encrypt fail"))
    }

    fn decrypt(&self, msg: &[u8]) -> Result<Vec<u8>> {
        self.cipher
            .decrypt(&self.nonce, msg)
            .map_err(|_| anyhow!("decrypt fail"))
    }
}
