//! Pawonee backend: API resep, rekomendasi, stok, dan profil di atas DB logis `pawonee`.
//! Selaras dengan modul `:shared:pantry-resep` milik Pawonee.

pub mod config;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

pub use error::{AppError, AppResult};
pub use state::AppState;

/// Versi kontrak publik (header `X-Kontrak-Versi`).
pub const VERSI_KONTRAK: &str = "v1";
