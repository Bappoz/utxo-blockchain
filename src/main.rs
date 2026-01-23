use utxo_blockchain::models::{block::Block, transaction::{Input, Output, Transaction}};
use utxo_blockchain::crypto::wallet::Wallet; 
use ed25519_dalek::Verifier;

fn main() {
    println!("=== ⛓️  TESTE INTEGRADO: BLOCKCHAIN RUST (ETAPAS 1 & 2) ===\n");

    // 1. GERAR CARTEIRAS (IDENTIDADES)
    let alice_wallet = Wallet::new();
    let bob_wallet = Wallet::new();
    let miner_wallet = Wallet::new();

    println!("✅ Carteiras geradas:");
    println!("   Alice: {}", alice_wallet.address());
    println!("   Bob:   {}", bob_wallet.address());
    println!("   Miner: {}\n", miner_wallet.address());

    // 2. CRIAR O BLOCO GÉNESIS (COINBASE)
    // O minerador recebe 50 moedas "do nada" para começar a rede
    let genesis_tx = Transaction::coinbase(&miner_wallet.address(), 50);
    let genesis_block = Block::genesis(genesis_tx);
    let genesis_hash = genesis_block.header.calculate_hash();

    println!("📦 Bloco Génesis Criado!");
    println!("   Hash: {}", genesis_hash.to_hex());
    println!("   Merkle Root: {}\n", genesis_block.header.merkle_root.to_hex());

    // 3. CRIAR UMA TRANSAÇÃO ASSINADA (Miner -> Bob)
    println!("--- Criando Transação: Miner envia 10 para Bob ---");
    
    // O input aponta para a transação coinbase do génesis (índice 0)
    let mut tx = Transaction {
        inputs: vec![Input {
            prev_tx_hash: genesis_block.transactions[0].calculate_hash(),
            output_index: 0,
            signature: None, // Ainda não assinado
        }],
        outputs: vec![
            Output {
                value: 10,
                pubkey: bob_wallet.address(),
            },
            Output {
                value: 40, // Troco para o minerador
                pubkey: miner_wallet.address(),
            },
        ],
    };

    // ASSINATURA: O minerador assina com a sua chave privada
    tx.sign(&miner_wallet.secret);
    println!("✅ Transação assinada pelo Miner.");

    // 4. VERIFICAÇÃO DE SEGURANÇA
    let data_to_verify = tx.get_data_to_sign();
    let signature_bytes = tx.inputs[0].signature.as_ref().unwrap();
    let signature = ed25519_dalek::Signature::from_slice(signature_bytes).unwrap();

    // Qualquer nó pode verificar se a assinatura é válida usando a chave pública do minerador
    let is_valid = miner_wallet.public.verify(&data_to_verify, &signature).is_ok();
    println!("🛡️  Assinatura válida? {}\n", is_valid);

    // 5. TESTE DE ATAQUE (TAMPERING)
    println!("--- Simulação de Ataque (Tentativa de alterar valor) ---");
    let mut malicious_tx = tx.clone();
    malicious_tx.outputs[0].value = 100; // Alterando valor de 10 para 100

    let data_malicious = malicious_tx.get_data_to_sign();
    let is_malicious_valid = miner_wallet.public.verify(&data_malicious, &signature).is_ok();
    println!("🚨 Transação maliciosa válida? {} (Esperado: false)\n", is_malicious_valid);

    // 6. ADICIONAR AO BLOCO #1
    let block1 = Block::new(genesis_hash, vec![tx]);
    let block1_hash = block1.header.calculate_hash();

    println!("   Bloco #1 Minerado!");
    println!("   Hash Anterior: {}", block1.header.prev_block_hash.to_hex());
    println!("   Hash Atual:    {}", block1_hash.to_hex());
    println!("   Merkle Root:   {}", block1.header.merkle_root.to_hex());

    println!("\n=== TESTE CONCLUÍDO COM SUCESSO ===");
}