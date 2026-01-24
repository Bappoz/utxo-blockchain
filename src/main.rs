use utxo_blockchain::models::{block::Block, blockchain::Blockchain, transaction::{Input, Output, Transaction}};
use utxo_blockchain::crypto::wallet::Wallet; 


fn main() {
    println!("=== 🧪 TESTE DA ETAPA 3: UTXO & CONTABILIDADE ===\n");

    // 1. Criar identidades
    let miner_wallet = Wallet::new();
    let alice_wallet = Wallet::new();
    let bob_wallet = Wallet::new();

    // 2. Criar Bloco Gênesis (O Minerador ganha 50 moedas)
    let genesis_tx = Transaction::coinbase(&miner_wallet.address(), 50);
    let genesis_block = Block::genesis(genesis_tx);
    
    // Inicializar a Blockchain com o Genesis
    let mut blockchain = Blockchain::new(genesis_block);
    
    println!("💰 Saldo Inicial Minerador: {} moedas", blockchain.get_balance(&miner_wallet.address()));
    println!("💰 Saldo Inicial Alice: {} moedas\n", blockchain.get_balance(&alice_wallet.address()));

    // --- CENÁRIO 1: Transação Válida (Minerador -> Alice) ---
    println!("🛒 Minerador enviando 20 moedas para Alice...");
    
    let mut tx1 = Transaction {
        inputs: vec![Input {
            prev_tx_hash: blockchain.chain[0].transactions[0].calculate_hash(),
            output_index: 0,
            signature: None,
        }],
        outputs: vec![
            Output { value: 20, pubkey: alice_wallet.address() }, // Envio
            Output { value: 30, pubkey: miner_wallet.address() }, // Troco
        ],
    };

    // Assinar a transação
    tx1.sign(&miner_wallet.secret);

    // Validar e Adicionar ao Bloco
    if blockchain.validate_transaction(tx1.clone()) {
        let prev_hash = blockchain.chain.last().unwrap().header.calculate_hash();
        let block1 = Block::new(prev_hash, vec![tx1]);
        blockchain.add_block(block1);
        println!("✅ Bloco #1 adicionado com sucesso!");
    }

    println!("💰 Novo Saldo Minerador: {}", blockchain.get_balance(&miner_wallet.address()));
    println!("💰 Novo Saldo Alice: {}\n", blockchain.get_balance(&alice_wallet.address()));

    // --- CENÁRIO 2: Tentativa de Gasto Duplo (Double Spending) ---
    println!("🚨 TENTATIVA DE FRAUDE: Minerador tentando usar o mesmo Input novamente...");
    
    let mut tx_fraud = Transaction {
        inputs: vec![Input {
            prev_tx_hash: blockchain.chain[0].transactions[0].calculate_hash(), // O mesmo do genesis!
            output_index: 0,
            signature: None,
        }],
        outputs: vec![Output { value: 50, pubkey: bob_wallet.address() }],
    };
    tx_fraud.sign(&miner_wallet.secret);

    if !blockchain.validate_transaction(tx_fraud) {
        println!("🛡️  Bloqueado: O sistema detectou que essas moedas já foram gastas!\n");
    }

    // --- CENÁRIO 3: Saldo Insuficiente ---
    println!("🚨 TENTATIVA DE FRAUDE: Alice tentando enviar mais do que tem (100 moedas)...");
    
    let mut tx_broke = Transaction {
        inputs: vec![Input {
            prev_tx_hash: blockchain.chain[1].transactions[0].calculate_hash(), // Refere-se ao bloco 1
            output_index: 0,
            signature: None,
        }],
        outputs: vec![Output { value: 100, pubkey: bob_wallet.address() }],
    };
    tx_broke.sign(&alice_wallet.secret);

    if !blockchain.validate_transaction(tx_broke) {
        println!("🛡️  Bloqueado: Saldo insuficiente detectado!");
    }

    println!("\n=== ✅ FIM DOS TESTES DA ETAPA 3 ===");
}