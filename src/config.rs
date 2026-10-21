//! Konfigurasi dari variabel lingkungan.

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub alamat: SocketAddr,
    /// Batas waktu per permintaan (detik).
    pub timeout_detik: u64,
}

impl Config {
    pub fn dari_env() -> Result<Self, String> {
        let database_url =
            std::env::var("DATABASE_URL").map_err(|_| "DATABASE_URL belum diatur".to_string())?;
        let alamat = std::env::var("ALAMAT")
            .unwrap_or_else(|_| "0.0.0.0:8080".into())
            .parse()
            .map_err(|e| format!("ALAMAT tidak valid: {e}"))?;
        let timeout_detik = std::env::var("TIMEOUT_DETIK")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(15);
        Ok(Self {
            database_url,
            alamat,
            timeout_detik,
        })
    }
}
