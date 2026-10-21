//! Konfigurasi dari variabel lingkungan.

use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub alamat: SocketAddr,
    /// Batas waktu per permintaan (detik).
    pub timeout_detik: u64,
    /// Rahasia penandatangan JWT (minimal 32 karakter).
    pub jwt_rahasia: String,
    /// Basis URL adaptor stok Pedaree (opsional; tanpa ini `GET /stok` membalas 502).
    pub pedaree_basis: Option<String>,
    pub pedaree_kunci_layanan: Option<String>,
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
        let jwt_rahasia =
            std::env::var("JWT_RAHASIA").map_err(|_| "JWT_RAHASIA belum diatur".to_string())?;
        if jwt_rahasia.len() < 32 {
            return Err("JWT_RAHASIA minimal 32 karakter".into());
        }
        Ok(Self {
            database_url,
            alamat,
            timeout_detik,
            jwt_rahasia,
            pedaree_basis: std::env::var("PEDAREE_BASE_URL")
                .ok()
                .filter(|v| !v.is_empty()),
            pedaree_kunci_layanan: std::env::var("PEDAREE_SERVICE_KEY")
                .ok()
                .filter(|v| !v.is_empty()),
        })
    }
}
