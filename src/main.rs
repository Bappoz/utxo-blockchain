use utxo_blockchain::models::{block::Block, blockchain::Blockchain, transaction::{Input, Output, Transaction}};
use utxo_blockchain::crypto::wallet::Wallet; 


fn main() {
    println!("=== 🧪 TESTE DA ETAPA 3: UTXO & CONTABILIDADE ===\n");

    // 1. Criar identidades
    let miner_wallet = Wallet::new();
    let alice_wallet = Wallet::new();

    // 2. Criar Bloco Gênesis (O Minerador ganha 50 moedas)
    let genesis_tx = Transaction::coinbase(&miner_wallet.address(), 50);
    let mut genesis_block = Block::genesis(genesis_tx);

    genesis_block.header.difficulty = 2;
    genesis_block.mine();

    // Inicializar a Blockchain com o Genesis
    let mut blockchain = Blockchain::new(genesis_block);
    
    println!("💰 Saldo Inicial Minerador: {} moedas", blockchain.get_balance(&miner_wallet.address()));
    println!("💰 Saldo Inicial Alice: {} moedas\n", blockchain.get_balance(&alice_wallet.address()));

    let mut tx1 = Transaction {
        inputs: vec![Input {
            prev_tx_hash: blockchain.chain[0].transactions[0].calculate_hash(),
            output_index: 0,
            signature: None,
        }],
        outputs: vec![
            Output { value: 10, pubkey: alice_wallet.address() }, // Envio
            Output { value: 40, pubkey: miner_wallet.address() }, // Troco
        ],
    };

    // Assinar a transação
    tx1.sign(&miner_wallet.secret);

    // Criar bloco 1
    let prev_hash = blockchain.chain.last().unwrap().header.calculate_hash();

    // O bloco #1 contem a transacao de envio + a recompensa do minerador por este novo bloco
    let coinbase_reward = Transaction::coinbase(&miner_wallet.address(), 50);

    let mut block1 = Block::new(prev_hash, vec![coinbase_reward, tx1]);
    block1.header.difficulty = 4;

    block1.mine();

    if blockchain.add_block(block1) {
        println!("Bloco 1 aceito pela rede");
    }

    println!("Saldo Alice: {}", blockchain.get_balance(&alice_wallet.address()));
    println!("Saldo Miner: {}", blockchain.get_balance(&miner_wallet.address()));
}