use serde::{Serialize, Deserialize};
use crate::models::block::Block;
use crate::models::transaction::Transaction;


#[derive(Debug, Serialize, Deserialize)]
pub enum Message {
    DiscoverNodes,                  // Quem está online
    Version(usize),                 // Eu tenho x blocos. E voce ?
    Subscribe,                      // Me avise de novas transações
    NewTransaction(Transaction),    // Acabei de receber essa transação
    NewBlock(Block),                // Achei um novo bloco
    RequestChain,                   // me mande um blockchain completo
    FullChain(Vec<Block>),          //Aqui esta a minha chain
} 

