//! Autentikasi: JWT akses 15 menit (audiens `pawonee`), refresh 7 hari dengan reuse-detection,
//! API key perangkat, service key (rotasi 90 hari), PIN, dan pembatas laju.

pub mod jwt;
pub mod kunci;
pub mod limiter;
pub mod pemanggil;
pub mod pin;
pub mod refresh;
pub mod sandi;

pub use pemanggil::{Jenis, Pemanggil};

use sha2::{Digest, Sha256};

/// Hash SHA-256 heksadesimal untuk rahasia berentropi tinggi (token/kunci acak).
pub fn hash_rahasia(nilai: &str) -> String {
    let mut h = Sha256::new();
    h.update(nilai.as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// Rahasia acak 244 bit (dua UUID v4) dalam heksadesimal.
pub fn rahasia_acak() -> String {
    format!(
        "{}{}",
        uuid::Uuid::new_v4().simple(),
        uuid::Uuid::new_v4().simple()
    )
}
