use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::network::messages::Message;

pub struct Node {
    pub address: String,
    pub known_peers: Vec<String>,
}

impl Node {
    pub async fn start_server(&self) -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind(&self.address).await?;
        println!("Node listening in: {}", self.address);

        loop {
            let (mut socket, addr) = listener.accept().await?;
            tokio::spawn(async move {
                let mut buffer = [0; 4096];
                match socket.read(&mut buffer).await {
                    Ok(n) if n > 0 => {
                        let msg_str = String::from_utf8_lossy(&buffer[..n]);
                        println!(" Mensagem recebida: {}", msg_str);
                    }
                    _ => {}
                }
            });
        }
    }
}