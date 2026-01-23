use ed25519_dalek::{VerifyingKey, SigningKey};
use rand::rngs::OsRng;

#[derive(Debug)]
pub struct Wallet {
    pub secret: SigningKey,
    pub public: VerifyingKey,
}

impl Wallet {
    pub fn new() -> Self {
        let mut csprng = OsRng;
        let secret: SigningKey = SigningKey::generate(&mut csprng);
        let public: VerifyingKey = VerifyingKey::from(&secret);
        Wallet { secret, public }
    }

    /// Retorna a chave pública formatada como string hex (para o endereço)
    pub fn address(&self) -> String {
        hex::encode(self.public.as_bytes())
    }
}