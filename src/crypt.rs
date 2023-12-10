use base64::{engine::general_purpose, Engine as _};
use openssl::symm::{decrypt, encrypt, Cipher};
use rand::{thread_rng, Rng};

use crate::error;

fn generate_iv(size: usize) -> Vec<u8> {
    let mut rng = thread_rng();
    let iv: Vec<u8> = (0..size).map(|_| rng.gen()).collect();
    iv
}

/// Repeat the key until it reaches the desired length.
fn match_key_length(key: &str, length: usize) -> String {
    let mut key = key.to_string();
    while key.len() < length {
        key = format!("{}{}", key, key);
    }
    key[..length].to_string()
}

pub fn encrypt_text(text: &str, key: &str) -> Result<String, error::CryptError> {
    let key_len = Cipher::aes_256_cbc().key_len();
    let iv = generate_iv(key_len);
    let cipher = Cipher::aes_256_cbc();

    let ciphertext = encrypt(
        cipher,
        match_key_length(key, key_len).as_bytes(),
        Some(&iv),
        text.as_bytes(),
    )?;

    let iv_and_ciphertext = [&iv[..], &ciphertext[..]].concat();
    let encoded: String = general_purpose::STANDARD_NO_PAD.encode(iv_and_ciphertext);
    Ok(encoded)
}

pub fn decrypt_text(text: &str, key: &str) -> Result<String, error::CryptError> {
    let key_len = Cipher::aes_256_cbc().key_len();
    let decoded = general_purpose::STANDARD_NO_PAD.decode(text)?;
    let iv = decoded[..key_len].to_vec();
    let data = &decoded[key_len..];

    let cipher = Cipher::aes_256_cbc();
    let decrypted = decrypt(
        cipher,
        match_key_length(key, key_len).as_bytes(),
        Some(&iv),
        data,
    )?;

    Ok(String::from_utf8(decrypted)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_key() {
        let key = "123";
        let text = "Hello, world!";
        let encrypted = encrypt_text(text, key);
        assert!(encrypted.is_ok());
    }

    #[test]
    fn test_unique_every_time() {
        let key = "0123456789abcdef0123456789abcdef";
        let text = "Hello, world!";
        let encrypted = encrypt_text(text, key).unwrap();
        let encrypted2 = encrypt_text(text, key).unwrap();
        assert_ne!(encrypted, encrypted2);
    }

    #[test]
    fn test_decrypt() {
        let key = "0123456789abcdef0123456789abcdef";
        let text = "Hello, world!";
        let encrypted = encrypt_text(text, key).unwrap();
        let decrypted = decrypt_text(&encrypted, key).unwrap();
        assert_eq!(text, decrypted);
    }
}
