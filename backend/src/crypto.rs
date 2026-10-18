use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, Key, OsRng, rand_core::RngCore},
};

#[derive(Debug)]
pub enum TokenCryptoError {
    InvalidKeyLength,
    EncryptionFailed,
    DecryptionFailed,
}

impl std::fmt::Display for TokenCryptoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenCryptoError::InvalidKeyLength => write!(f, "invalid token encryption key length"),
            TokenCryptoError::EncryptionFailed => write!(f, "token encryption failed"),
            TokenCryptoError::DecryptionFailed => write!(f, "token decryption failed"),
        }
    }
}

impl std::error::Error for TokenCryptoError {}

pub fn encrypt_token(key: &[u8], plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>), TokenCryptoError> {
    if key.len() != 32 {
        return Err(TokenCryptoError::InvalidKeyLength);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let mut nonce_bytes = [0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| TokenCryptoError::EncryptionFailed)?;
    Ok((ciphertext, nonce_bytes.to_vec()))
}

pub fn decrypt_token(
    key: &[u8],
    ciphertext: &[u8],
    nonce: &[u8],
) -> Result<Vec<u8>, TokenCryptoError> {
    if key.len() != 32 {
        return Err(TokenCryptoError::InvalidKeyLength);
    }
    if nonce.len() != 12 {
        return Err(TokenCryptoError::DecryptionFailed);
    }
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(key));
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| TokenCryptoError::DecryptionFailed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key() -> Vec<u8> {
        vec![7u8; 32]
    }

    #[test]
    fn round_trip() {
        let key = key();
        let plaintext = b"secret-token";
        let (ciphertext, nonce) = encrypt_token(&key, plaintext).unwrap();
        assert_ne!(ciphertext, plaintext);
        assert_eq!(nonce.len(), 12);
        let decrypted = decrypt_token(&key, &ciphertext, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn tampering_fails() {
        let key = key();
        let (mut ciphertext, nonce) = encrypt_token(&key, b"token").unwrap();
        if let Some(byte) = ciphertext.first_mut() {
            *byte ^= 1;
        }
        assert!(decrypt_token(&key, &ciphertext, &nonce).is_err());
    }

    #[test]
    fn wrong_key_fails() {
        let key = key();
        let (ciphertext, nonce) = encrypt_token(&key, b"token").unwrap();
        let wrong = vec![9u8; 32];
        assert!(decrypt_token(&wrong, &ciphertext, &nonce).is_err());
    }

    #[test]
    fn invalid_key_length() {
        assert!(matches!(
            encrypt_token(&[1u8; 16], b"token"),
            Err(TokenCryptoError::InvalidKeyLength)
        ));
        assert!(matches!(
            decrypt_token(&[1u8; 16], b"ciphertext", &[0u8; 12]),
            Err(TokenCryptoError::InvalidKeyLength)
        ));
    }

    #[test]
    fn modified_nonce_fails() {
        let key = key();
        let (ciphertext, mut nonce) = encrypt_token(&key, b"token").unwrap();
        if let Some(byte) = nonce.first_mut() {
            *byte ^= 1;
        }
        assert!(decrypt_token(&key, &ciphertext, &nonce).is_err());
    }

    #[test]
    fn wrong_nonce_length_fails() {
        let key = key();
        assert!(matches!(
            decrypt_token(&key, b"ciphertext", &[0u8; 11]),
            Err(TokenCryptoError::DecryptionFailed)
        ));
    }
}
