//! Satu `AppError` untuk seluruh galat; dipetakan ke respons baku kontrak.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

pub type AppResult<T> = Result<T, AppError>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Rinci {
    pub bidang: String,
    pub pesan: String,
}

#[derive(Debug)]
pub enum AppError {
    Validasi { pesan: String, rinci: Vec<Rinci> },
    TidakTerotentikasi(String),
    ResepTidakAda(String),
    ProfilTidakAda(String),
    Konflik(String),
    TakTerproses(String),
    JiwaDiLuarRentang,
    BatasLaju,
    JaringanGagal(String),
    Server(String),
}

impl AppError {
    pub fn validasi(bidang: &str, pesan: &str) -> Self {
        Self::Validasi {
            pesan: "Parameter tidak valid.".into(),
            rinci: vec![Rinci {
                bidang: bidang.into(),
                pesan: pesan.into(),
            }],
        }
    }

    pub fn validasi_banyak(rinci: Vec<Rinci>) -> Self {
        Self::Validasi {
            pesan: "Parameter tidak valid.".into(),
            rinci,
        }
    }

    pub fn kode(&self) -> &'static str {
        match self {
            Self::Validasi { .. } => "VALIDASI_GAGAL",
            Self::TidakTerotentikasi(_) => "TIDAK_TEROTENTIKASI",
            Self::ResepTidakAda(_) => "RESEP_TIDAK_ADA",
            Self::ProfilTidakAda(_) => "PROFIL_TIDAK_ADA",
            Self::Konflik(_) => "KONFLIK",
            Self::TakTerproses(_) => "TAK_TERPROSES",
            Self::JiwaDiLuarRentang => "JIWA_DI_LUAR_RENTANG",
            Self::BatasLaju => "BATAS_LAJU",
            Self::JaringanGagal(_) => "JARINGAN_GAGAL",
            Self::Server(_) => "KESALAHAN_SERVER",
        }
    }

    pub fn status(&self) -> StatusCode {
        match self {
            Self::Validasi { .. } => StatusCode::BAD_REQUEST,
            Self::TidakTerotentikasi(_) => StatusCode::UNAUTHORIZED,
            Self::ResepTidakAda(_) | Self::ProfilTidakAda(_) => StatusCode::NOT_FOUND,
            Self::Konflik(_) => StatusCode::CONFLICT,
            Self::TakTerproses(_) | Self::JiwaDiLuarRentang => StatusCode::UNPROCESSABLE_ENTITY,
            Self::BatasLaju => StatusCode::TOO_MANY_REQUESTS,
            Self::JaringanGagal(_) => StatusCode::BAD_GATEWAY,
            Self::Server(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn pesan(&self) -> String {
        match self {
            Self::Validasi { pesan, .. } => pesan.clone(),
            Self::TidakTerotentikasi(p)
            | Self::ResepTidakAda(p)
            | Self::ProfilTidakAda(p)
            | Self::Konflik(p)
            | Self::TakTerproses(p)
            | Self::JaringanGagal(p) => p.clone(),
            Self::JiwaDiLuarRentang => "Jumlah jiwa harus 1 sampai 8.".into(),
            Self::BatasLaju => "Terlalu banyak permintaan.".into(),
            // Rincian galat internal tidak pernah dibocorkan ke klien.
            Self::Server(_) => "Terjadi kesalahan pada server.".into(),
        }
    }
}

#[derive(Serialize)]
struct Badan {
    kode: &'static str,
    pesan: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    rinci: Vec<Rinci>,
    #[serde(rename = "retrySetelahDetik", skip_serializing_if = "Option::is_none")]
    retry_setelah_detik: Option<u32>,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        if let Self::Server(detail) = &self {
            tracing::error!(%detail, "kesalahan server");
        }
        let rinci = match &self {
            Self::Validasi { rinci, .. } => rinci.clone(),
            _ => vec![],
        };
        let badan = Badan {
            kode: self.kode(),
            pesan: self.pesan(),
            rinci,
            retry_setelah_detik: matches!(self, Self::BatasLaju).then_some(60),
        };
        (self.status(), Json(badan)).into_response()
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        Self::Server(format!("database: {e}"))
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kode(), self.pesan())
    }
}

impl std::error::Error for AppError {}
