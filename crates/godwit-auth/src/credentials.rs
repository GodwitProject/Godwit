use aes_gcm::aead::{rand_core::RngCore, Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use godwit_core::PasteurError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecret {
    pub ciphertext: String,
    pub nonce: String,
}

pub fn encrypt_api_key(master_key: &[u8; 32], plaintext: &str) -> EncryptedSecret {
    let cipher = Aes256Gcm::new(master_key.into());
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .expect("AES-256-GCM encryption with a valid 12-byte nonce cannot fail");
    EncryptedSecret {
        ciphertext: STANDARD.encode(ciphertext),
        nonce: STANDARD.encode(nonce_bytes),
    }
}

pub fn decrypt_api_key(master_key: &[u8; 32], secret: &EncryptedSecret) -> Result<String, PasteurError> {
    let cipher = Aes256Gcm::new(master_key.into());
    let nonce_bytes = STANDARD
        .decode(&secret.nonce)
        .map_err(|e| PasteurError::Auth(format!("invalid credential nonce encoding: {e}")))?;
    let ciphertext = STANDARD
        .decode(&secret.ciphertext)
        .map_err(|e| PasteurError::Auth(format!("invalid credential ciphertext encoding: {e}")))?;
    if nonce_bytes.len() != 12 {
        return Err(PasteurError::Auth("invalid credential nonce length".to_string()));
    }
    let nonce = Nonce::from_slice(&nonce_bytes);
    let plaintext = cipher
        .decrypt(nonce, ciphertext.as_slice())
        .map_err(|_| PasteurError::Auth("failed to decrypt provider credential".to_string()))?;
    String::from_utf8(plaintext).map_err(|e| PasteurError::Auth(e.to_string()))
}

pub fn load_master_key_from_env(var: &str) -> Result<[u8; 32], PasteurError> {
    let encoded = std::env::var(var).map_err(|_| PasteurError::Config(format!("{var} is not set")))?;
    let bytes = STANDARD
        .decode(&encoded)
        .map_err(|e| PasteurError::Config(format!("{var} is not valid base64: {e}")))?;
    bytes
        .try_into()
        .map_err(|v: Vec<u8>| PasteurError::Config(format!("{var} must decode to exactly 32 bytes, got {}", v.len())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_key() -> [u8; 32] {
        [7u8; 32]
    }

    #[test]
    fn encrypt_then_decrypt_round_trips() {
        let key = test_key();
        let secret = encrypt_api_key(&key, "sk-real-provider-key");
        let plaintext = decrypt_api_key(&key, &secret).expect("decrypt");
        assert_eq!(plaintext, "sk-real-provider-key");
    }

    #[test]
    fn decrypt_fails_with_wrong_key() {
        let secret = encrypt_api_key(&test_key(), "sk-real-provider-key");
        let wrong_key = [9u8; 32];
        assert!(decrypt_api_key(&wrong_key, &secret).is_err());
    }

    #[test]
    fn decrypt_fails_with_tampered_ciphertext() {
        let key = test_key();
        let mut secret = encrypt_api_key(&key, "sk-real-provider-key");

        // Decode the base64 ciphertext
        let mut ciphertext_bytes = STANDARD
            .decode(&secret.ciphertext)
            .expect("valid base64 from encrypt_api_key");

        // Flip a byte in the ciphertext to tamper with it
        if !ciphertext_bytes.is_empty() {
            ciphertext_bytes[0] ^= 0xFF;
        }

        // Re-encode the tampered ciphertext back to base64
        secret.ciphertext = STANDARD.encode(&ciphertext_bytes);

        // Decryption should fail due to AEAD tag verification, not base64 decoding
        assert!(decrypt_api_key(&key, &secret).is_err());
    }

    #[test]
    fn load_master_key_from_env_decodes_base64() {
        std::env::set_var("TEST_CREDENTIAL_KEY", base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            [1u8; 32],
        ));
        let key = load_master_key_from_env("TEST_CREDENTIAL_KEY").expect("load key");
        assert_eq!(key, [1u8; 32]);
        std::env::remove_var("TEST_CREDENTIAL_KEY");
    }

    #[test]
    fn load_master_key_from_env_rejects_wrong_length() {
        std::env::set_var(
            "TEST_CREDENTIAL_KEY_SHORT",
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, [1u8; 16]),
        );
        assert!(load_master_key_from_env("TEST_CREDENTIAL_KEY_SHORT").is_err());
        std::env::remove_var("TEST_CREDENTIAL_KEY_SHORT");
    }
}
