use utxo_blockchain::models::{block::Block, transaction::Transaction};
fn main() {
    println!("=== Testando Blockchain Etapa 1 ===");

    // 1. Criar a Transação Coinbase para o Bloco Gênesis
    // Simulando que o minerador "Alice" recebeu 50 moedas
    let genesis_tx = Transaction::coinbase("Alice", 50);
    println!("1. Transação Genesis criada para Alice.");

    // 2. Criar o Bloco Gênesis
    let genesis_block = Block::genesis(genesis_tx);
    let genesis_hash = genesis_block.header.calculate_hash();
    
    println!("   Hash do Bloco Gênesis: {}", genesis_hash.to_hex());
    println!("   Merkle Root: {}", genesis_block.header.merkle_root.to_hex());

    println!("\n--- Criando um novo bloco ---");

    // 3. Criar transações para o Bloco #1
    let tx1 = Transaction::coinbase("Bob", 25); // Recompensa do novo bloco
    
    // Simulando uma transação comum (na Etapa 1 ainda não validamos assinaturas)
    let tx2 = Transaction {
        inputs: vec![], // No futuro, isso apontará para a saída da Alice
        outputs: vec![
            utxo_blockchain::models::transaction::Output {
                value: 10,
                pubkey: "Charlie".to_string(),
            }
        ],
    };

    let block1_transactions = vec![tx1, tx2];

    // 4. Criar o Bloco #1 apontando para o Hash do Gênesis
    let block1 = Block::new(genesis_hash, block1_transactions);
    let block1_hash = block1.header.calculate_hash();

    println!("2. Bloco #1 criado.");
    println!("   Hash anterior (Gênesis): {}", block1.header.prev_block_hash.to_hex());
    println!("   Hash atual do Bloco #1: {}", block1_hash.to_hex());
    println!("   Merkle Root do Bloco #1: {}", block1.header.merkle_root.to_hex());

    // 5. Verificação de Integridade Simples
    println!("\n=== Verificação de Integridade ===");
    if block1.header.prev_block_hash == genesis_hash {
        println!("Sucesso: O Bloco #1 está corretamente encadeado ao Gênesis.");
    } else {
        println!("Erro: O encadeamento falhou!");
    }
}