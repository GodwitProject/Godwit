use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub fn generate_refresh_token() -> (String, String) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let plaintext = bs58::encode(&bytes).into_string();
    let hash = hash_refresh_token(&plaintext);
    (plaintext, hash)
}

pub fn hash_refresh_token(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    hex::encode(hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_produces_hashable_token() {
        let (plaintext, hash) = generate_refresh_token();
        assert!(!plaintext.is_empty());
        assert_eq!(hash_refresh_token(&plaintext), hash);
    }

    #[test]
    fn different_tokens_hash_differently() {
        let (_, hash_a) = generate_refresh_token();
        let (_, hash_b) = generate_refresh_token();
        assert_ne!(hash_a, hash_b);
    }

    #[test]
    fn hash_is_deterministic() {
        let (plaintext, _) = generate_refresh_token();
        assert_eq!(hash_refresh_token(&plaintext), hash_refresh_token(&plaintext));
    }
}
