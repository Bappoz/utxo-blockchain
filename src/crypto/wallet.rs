use ed25519_dalek::{VerifyingKey, SigningKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

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

    pub fn from_seed(seed_text: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(seed_text.as_bytes());
        let entropy = hasher.finalize();

        let secret = SigningKey::from_bytes(&entropy.into());
        let public = VerifyingKey::from(&secret);
        Wallet { secret, public }
    }
}