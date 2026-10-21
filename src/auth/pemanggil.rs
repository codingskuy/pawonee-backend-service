//! Extractor `Pemanggil`: mengenali JWT bearer, `X-Device-Key`, atau `X-Service-Key`,
//! lalu menerapkan batas 30 permintaan/menit per token.

use super::{hash_rahasia, jwt, kunci};
use crate::{AppError, AppState};
use axum::{extract::FromRequestParts, http::request::Parts};
use chrono::Utc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pemanggil {
    Pengguna {
        id: String,
        id_keluarga: Option<String>,
    },
    Perangkat(kunci::Perangkat),
    Layanan(kunci::Layanan),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Jenis {
    Bearer,
    Perangkat,
    Layanan,
}

impl Pemanggil {
    pub fn jenis(&self) -> Jenis {
        match self {
            Self::Pengguna { .. } => Jenis::Bearer,
            Self::Perangkat(_) => Jenis::Perangkat,
            Self::Layanan(_) => Jenis::Layanan,
        }
    }

    /// Tolak (401) bila jenis kredensial tidak termasuk yang diizinkan endpoint.
    pub fn wajib(self, diizinkan: &[Jenis]) -> Result<Self, AppError> {
        if diizinkan.contains(&self.jenis()) {
            Ok(self)
        } else {
            Err(AppError::TidakTerotentikasi(
                "Jenis kredensial tidak diizinkan untuk endpoint ini.".into(),
            ))
        }
    }

    /// FR-301: pengguna/perangkat hanya boleh mengakses keluarganya sendiri; service key lintas keluarga
    /// hanya dengan cakupan `<cakupan>`. Akun tanpa keluarga terikat ditolak.
    pub fn boleh_keluarga(&self, id_keluarga: &str, cakupan: &str) -> Result<(), AppError> {
        let ok = match self {
            Self::Pengguna { id_keluarga: k, .. } => k.as_deref() == Some(id_keluarga),
            Self::Perangkat(p) => p.id_keluarga.as_deref() == Some(id_keluarga),
            Self::Layanan(l) => l.cakupan.iter().any(|c| c == cakupan),
        };
        if ok {
            Ok(())
        } else {
            Err(AppError::TidakTerotentikasi(
                "Tidak berhak atas data keluarga ini.".into(),
            ))
        }
    }
}

impl FromRequestParts<AppState> for Pemanggil {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let kepala = |nama: &str| {
            parts
                .headers
                .get(nama)
                .and_then(|v| v.to_str().ok())
                .map(str::to_owned)
        };
        let sekarang = Utc::now();

        let (kunci_laju, pemanggil) = if let Some(a) = kepala("authorization") {
            let token = a.strip_prefix("Bearer ").ok_or_else(|| {
                AppError::TidakTerotentikasi("Skema otorisasi harus Bearer.".into())
            })?;
            let k = jwt::verifikasi(&state.jwt_rahasia, token)?;
            (
                format!("jwt:{}", hash_rahasia(token)),
                Self::Pengguna {
                    id: k.sub,
                    id_keluarga: k.kel,
                },
            )
        } else if let Some(k) = kepala("x-device-key") {
            let p = kunci::cari_perangkat(&state.db, &k).await?.ok_or_else(|| {
                AppError::TidakTerotentikasi("Kunci perangkat tidak valid.".into())
            })?;
            (format!("dev:{}", hash_rahasia(&k)), Self::Perangkat(p))
        } else if let Some(k) = kepala("x-service-key") {
            let l = kunci::cari_service_key(&state.db, &k, sekarang)
                .await?
                .ok_or_else(|| {
                    AppError::TidakTerotentikasi("Service key tidak valid atau kedaluwarsa.".into())
                })?;
            (format!("svc:{}", hash_rahasia(&k)), Self::Layanan(l))
        } else {
            return Err(AppError::TidakTerotentikasi("Kredensial hilang.".into()));
        };

        if !state.pembatas.periksa(&kunci_laju) {
            return Err(AppError::BatasLaju);
        }
        Ok(pemanggil)
    }
}
