use utxo_blockchain::models::{block::Block, blockchain::Blockchain, transaction::{Input, Output, Transaction}};
use utxo_blockchain::crypto::wallet::Wallet; 

fn main() {
    let db_path = "blockchain.json";
    
    // 1. CARTEIRAS COM SEEDS FIXAS
    // Agora miner_wallet e alice_wallet terão sempre o mesmo endereço em cada execução
    let miner_wallet = Wallet::from_seed("minerador_secreto_123");
    let alice_wallet = Wallet::from_seed("alice_secreta_456");

    let mut blockchain = if std::path::Path::new(db_path).exists() {
        Blockchain::load_from_file(db_path).expect("Erro ao carregar")
    } else {
        println!("🌱 Criando novo Gênesis para o Minerador Fixo...");
        let genesis_tx = Transaction::coinbase(&miner_wallet.address(), 50);
        let genesis_block = Block::genesis(genesis_tx);
        let bc = Blockchain::new(genesis_block);
        bc.save_to_file(db_path).unwrap();
        bc
    };


    println!("\n--- 📥 TESTANDO MEMPOOL ---");
    println!("Saldo Inicial Minerador: {}", blockchain.get_balance(&miner_wallet.address()));

    // 2. CRIAR UMA TRANSAÇÃO MANUAL
    // Procuramos um UTXO que realmente pertença à chave do minerador_wallet
    let miner_addr = miner_wallet.address();
    let utxo_do_minerador = blockchain.utxos.iter().find(|(_, output)| {
        output.pubkey == miner_addr && output.value >= 10 
    });

    if let Some((key, output)) = utxo_do_minerador {
        let mut tx = Transaction {
            inputs: vec![Input {
                prev_tx_hash: key.tx_hash,
                output_index: key.output_index,
                signature: None,
            }],
            outputs: vec![
                Output { value: 10, pubkey: alice_wallet.address() },
                Output { value: output.value - 10, pubkey: miner_wallet.address() }, // Troco para o minerador
            ],
        };

        // Agora a assinatura vai bater, pois a miner_wallet é a dona real do UTXO
        tx.sign(&miner_wallet.secret);

        if blockchain.submit_transaction(tx) {
            println!("✅ Transação aceita na Mempool!");
        }
    } else {
        println!("❌ Minerador não possui UTXOs suficientes.");
    }

    println!("Saldo Alice (antes da mineração): {}", blockchain.get_balance(&alice_wallet.address()));

    // 3. MINERAR TUDO QUE ESTÁ NA MEMPOOL
    if !blockchain.mempool.is_empty() {
        // O minerador do bloco recebe a recompensa de 50 (coinbase)
        let mut novo_bloco = blockchain.create_next_block(&miner_wallet.address(), 16);
        
        println!("⛏️  Minerando bloco com {} transações...", novo_bloco.transactions.len());
        novo_bloco.mine();

        if blockchain.add_block(novo_bloco) {
            blockchain.save_to_file(db_path).unwrap();
            println!("✨ Bloco adicionado com sucesso!");
        }
    }

    println!("Saldo Alice (depois da mineração): {}", blockchain.get_balance(&alice_wallet.address()));
    println!("Saldo Minerador (depois da mineração): {}", blockchain.get_balance(&miner_wallet.address()));
}