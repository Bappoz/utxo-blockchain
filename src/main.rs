use std::env;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use utxo_blockchain::crypto::wallet::Wallet;
use utxo_blockchain::models::block::Block;
use utxo_blockchain::models::blockchain::Blockchain;
use utxo_blockchain::models::transaction::Transaction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configuração de Argumentos (Ex: cargo run 8080)
    let args: Vec<String> = env::args().collect();
    let port = args.get(1).map(|s| s.as_str()).unwrap_or("8080");
    let addr = format!("127.0.0.1:{}", port);

    println!("🚀 Iniciando Nó na porta {}...", port);

    // Inicializar Blockchain e Carteira
    let miner_wallet = Wallet::from_seed("minerador_secreto_123");
    let mut blockchain = carregar_ou_criar_blockchain("blockchain.json", &miner_wallet)?;

    // CLONAR ESTADO PARA AS THREADS
    // Nota: Em um sistema real devo usar Arc<Mutex<Blockchain>>,
    // mas para este rascunho vamos focar na escuta de rede.

    let addr_clone = addr.clone();

    //  O SERVIDOR DE REDE (Escuta outros nós)
    tokio::spawn(async move {
        let listener = TcpListener::bind(&addr_clone).await.unwrap();
        println!("📡 Servidor P2P ouvindo em {}", addr_clone);

        loop {
            let (mut socket, peer_addr) = listener.accept().await.unwrap();
            println!("🤝 Novo peer conectado: {}", peer_addr);

            tokio::spawn(async move {
                let mut buffer = [0; 1024];
                loop {
                    let n = socket.read(&mut buffer).await.unwrap();
                    if n == 0 {
                        break;
                    }

                    let recebido = String::from_utf8_lossy(&buffer[..n]);
                    println!("📩 Mensagem de {}: {}", peer_addr, recebido);

                    // Responder apenas para confirmar
                    socket
                        .write_all(b"Mensagem recebida pelo no!")
                        .await
                        .unwrap();
                }
            });
        }
    });

    println!("⛏️  Minerador pronto. Pressione Enter para minerar um bloco ou 'q' para sair.");

    let mut input = String::new();
    loop {
        std::io::stdin().read_line(&mut input)?;
        if input.trim() == "q" {
            break;
        }

        println!("⛏️  Criando e minerando novo bloco...");
        let mut novo_bloco = blockchain.create_next_block(&miner_wallet.address(), 16);
        novo_bloco.mine()?;

        if blockchain.add_block(novo_bloco) {
            blockchain.save_to_file("blockchain.json")?;
            println!("✅ Bloco minerado e salvo!");
        }

        input.clear();
    }

    Ok(())
}

// Função auxiliar para simplificar a main
fn carregar_ou_criar_blockchain(
    path: &str,
    wallet: &Wallet,
) -> Result<Blockchain, Box<dyn std::error::Error>> {
    if std::path::Path::new(path).exists() {
        Ok(Blockchain::load_from_file(path)?)
    } else {
        let genesis_tx = Transaction::coinbase(&wallet.address(), 50);
        let genesis_block = Block::genesis(genesis_tx);
        let bc = Blockchain::new(genesis_block);
        bc.save_to_file(path)?;
        Ok(bc)
    }
}
