use argon2::{
    password_hash::{rand_core::RngCore, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand::rngs::OsRng;

const PREFIX: &str = "sk-pasteur-";
const PREFIX_LEN: usize = 16;

pub fn generate_api_key() -> (String, String, String) {
    let mut bytes = [0u8; 24];
    OsRng.fill_bytes(&mut bytes);
    let plaintext = format!("{}{}", PREFIX, bs58::encode(&bytes).into_string());
    let prefix = extract_prefix(&plaintext);
    let hash = hash_key(&plaintext);
    (plaintext, hash, prefix)
}

pub fn extract_prefix(key: &str) -> String {
    let start = PREFIX.len();
    let end = (start + PREFIX_LEN).min(key.len());
    key[start..end].to_string()
}

pub fn hash_key(key: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(key.as_bytes(), &salt)
        .expect("hash key")
        .to_string()
}

pub fn verify_key(key: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(key.as_bytes(), &parsed)
        .is_ok()
}

pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash password")
        .to_string()
}

pub fn verify_password(password: &str, hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(p) => p,
        Err(_) => return false,
    };
    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_verify_key() {
        let (plaintext, hash, prefix) = generate_api_key();
        assert!(plaintext.starts_with(PREFIX));
        assert!(verify_key(&plaintext, &hash));
        assert_eq!(extract_prefix(&plaintext), prefix);
    }

    #[test]
    fn verify_rejects_wrong_key() {
        let (_, hash, _) = generate_api_key();
        assert!(!verify_key("sk-pasteur-wrong", &hash));
    }

    #[test]
    fn hash_and_verify_password() {
        let hash = hash_password("hunter2");
        assert!(verify_password("hunter2", &hash));
        assert!(!verify_password("wrong", &hash));
    }
}
