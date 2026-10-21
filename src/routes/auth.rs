//! `POST /v1/auth/login` dan `POST /v1/auth/refresh`.

use crate::{
    auth::{refresh, sandi},
    ekstrak::JsonValid,
    AppError, AppResult, AppState,
};
use axum::{extract::State, Json};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Deserialize)]
pub struct Login {
    #[serde(rename = "idPengguna")]
    pub id_pengguna: String,
    #[serde(rename = "kataSandi")]
    pub kata_sandi: String,
    #[serde(rename = "idPerangkat")]
    pub id_perangkat: Option<String>,
}

#[derive(Deserialize)]
pub struct Refresh {
    #[serde(rename = "refreshToken")]
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct Data<T> {
    pub data: T,
}

/// Hash tiruan agar waktu respons pengguna tak dikenal setara dengan sandi salah.
fn hash_tiruan() -> &'static str {
    static H: OnceLock<String> = OnceLock::new();
    H.get_or_init(|| sandi::hash("tiruan-agar-waktu-sama").unwrap_or_default())
}

pub async fn login(
    State(s): State<AppState>,
    JsonValid(m): JsonValid<Login>,
) -> AppResult<Json<Data<refresh::TokenPasangan>>> {
    if m.id_pengguna.trim().is_empty() || m.id_pengguna.len() > 128 {
        return Err(AppError::validasi(
            "idPengguna",
            "wajib diisi (maks 128 karakter)",
        ));
    }
    if m.kata_sandi.is_empty() || m.kata_sandi.len() > 256 {
        return Err(AppError::validasi(
            "kataSandi",
            "wajib diisi (maks 256 karakter)",
        ));
    }
    if m.id_perangkat.as_deref().is_some_and(|p| p.len() > 128) {
        return Err(AppError::validasi("idPerangkat", "maksimal 128 karakter"));
    }
    if !s.pembatas.periksa(&format!("login:{}", m.id_pengguna)) {
        return Err(AppError::BatasLaju);
    }
    let baris: Option<(String, Option<String>)> =
        sqlx::query_as("SELECT hash_sandi, id_keluarga FROM pengguna WHERE id_pengguna = $1")
            .bind(&m.id_pengguna)
            .fetch_optional(&s.db)
            .await?;
    let (hash, id_keluarga) = match &baris {
        Some((h, k)) => (h.as_str(), k.as_deref()),
        None => (hash_tiruan(), None),
    };
    let cocok = sandi::cocok(&m.kata_sandi, hash);
    if !cocok || baris.is_none() {
        return Err(AppError::TidakTerotentikasi(
            "Kredensial tidak valid.".into(),
        ));
    }
    let pasangan = refresh::terbitkan_awal(
        &s.db,
        &s.jwt_rahasia,
        &m.id_pengguna,
        id_keluarga,
        Utc::now(),
    )
    .await?;
    Ok(Json(Data { data: pasangan }))
}

pub async fn refresh(
    State(s): State<AppState>,
    JsonValid(m): JsonValid<Refresh>,
) -> AppResult<Json<Data<refresh::TokenPasangan>>> {
    if m.refresh_token.is_empty() || m.refresh_token.len() > 256 {
        return Err(AppError::validasi("refreshToken", "wajib diisi"));
    }
    if !s.pembatas.periksa(&format!(
        "refresh:{}",
        crate::auth::hash_rahasia(&m.refresh_token)
    )) {
        return Err(AppError::BatasLaju);
    }
    let pasangan = refresh::tukar(&s.db, &s.jwt_rahasia, &m.refresh_token, Utc::now()).await?;
    Ok(Json(Data { data: pasangan }))
}
