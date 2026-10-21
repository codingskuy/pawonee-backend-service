//! Hash kredensial dengan argon2id.

use crate::AppError;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash(sandi: &str) -> Result<String, AppError> {
    let garam = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(sandi.as_bytes(), &garam)
        .map(|h| h.to_string())
        .map_err(|e| AppError::Server(format!("argon2: {e}")))
}

pub fn cocok(sandi: &str, hash_tersimpan: &str) -> bool {
    PasswordHash::new(hash_tersimpan)
        .map(|h| {
            Argon2::default()
                .verify_password(sandi.as_bytes(), &h)
                .is_ok()
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn hash_argon2_dan_verifikasi() {
        let h = hash("sandi-rahasia").unwrap();
        assert!(h.starts_with("$argon2id$"));
        assert!(cocok("sandi-rahasia", &h));
        assert!(!cocok("salah", &h));
        assert!(!cocok("x", "bukan-hash"));
    }
}
