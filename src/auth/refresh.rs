//! Refresh token 7 hari, sekali pakai, dengan reuse-detection.
//!
//! Setiap login membuka satu "keluarga token". Tiap penukaran menandai token lama `dipakai`
//! dan menerbitkan token baru di keluarga yang sama. Bila token yang sudah `dipakai` muncul
//! lagi, seluruh keluarga dicabut sehingga pengguna wajib login ulang.

use super::{hash_rahasia, jwt, rahasia_acak};
use crate::{AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub const UMUR_REFRESH_HARI: i64 = 7;

#[derive(Debug, Serialize)]
pub struct TokenPasangan {
    #[serde(rename = "tokenAkses")]
    pub token_akses: String,
    #[serde(rename = "tokenRefresh")]
    pub token_refresh: String,
    #[serde(rename = "kedaluwarsaDetik")]
    pub kedaluwarsa_detik: i64,
    pub audiens: &'static str,
}

fn tak_valid() -> AppError {
    AppError::TidakTerotentikasi("Refresh token tidak valid.".into())
}

async fn terbitkan_di_keluarga(
    db: &PgPool,
    rahasia_jwt: &str,
    id_pengguna: &str,
    id_keluarga: Option<&str>,
    keluarga_token: Uuid,
    sekarang: DateTime<Utc>,
) -> AppResult<TokenPasangan> {
    let mentah = rahasia_acak();
    sqlx::query(
        "INSERT INTO refresh_token (id, id_pengguna, keluarga_token, hash_token, kedaluwarsa_pada) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::now_v7())
    .bind(id_pengguna)
    .bind(keluarga_token)
    .bind(hash_rahasia(&mentah))
    .bind(sekarang + Duration::days(UMUR_REFRESH_HARI))
    .execute(db)
    .await?;
    Ok(TokenPasangan {
        token_akses: jwt::terbitkan_pada(
            rahasia_jwt,
            id_pengguna,
            id_keluarga,
            sekarang.timestamp(),
        )?,
        token_refresh: mentah,
        kedaluwarsa_detik: jwt::UMUR_AKSES_DETIK,
        audiens: jwt::AUDIENS,
    })
}

/// Pasangan token baru untuk login (membuka keluarga token baru).
pub async fn terbitkan_awal(
    db: &PgPool,
    rahasia_jwt: &str,
    id_pengguna: &str,
    id_keluarga: Option<&str>,
    sekarang: DateTime<Utc>,
) -> AppResult<TokenPasangan> {
    terbitkan_di_keluarga(
        db,
        rahasia_jwt,
        id_pengguna,
        id_keluarga,
        Uuid::now_v7(),
        sekarang,
    )
    .await
}

/// Tukar refresh token. Token terpakai/kedaluwarsa/dicabut → 401; pakai-ulang mencabut seluruh keluarga.
pub async fn tukar(
    db: &PgPool,
    rahasia_jwt: &str,
    token_mentah: &str,
    sekarang: DateTime<Utc>,
) -> AppResult<TokenPasangan> {
    let hash = hash_rahasia(token_mentah);
    let baris: Option<(Uuid, String, Uuid, bool, bool, DateTime<Utc>)> = sqlx::query_as(
        "SELECT id, id_pengguna, keluarga_token, dipakai, dicabut, kedaluwarsa_pada FROM refresh_token WHERE hash_token = $1",
    )
    .bind(&hash)
    .fetch_optional(db)
    .await?;
    let Some((id, id_pengguna, keluarga, dipakai, dicabut, kedaluwarsa)) = baris else {
        return Err(tak_valid());
    };
    if dipakai {
        sqlx::query("UPDATE refresh_token SET dicabut = true WHERE keluarga_token = $1")
            .bind(keluarga)
            .execute(db)
            .await?;
        tracing::warn!(%id_pengguna, "refresh token dipakai ulang; keluarga token dicabut");
        return Err(tak_valid());
    }
    if dicabut || kedaluwarsa <= sekarang {
        return Err(tak_valid());
    }
    // Penandaan atomik: bila dua permintaan berlomba, hanya satu yang menang.
    let menang = sqlx::query("UPDATE refresh_token SET dipakai = true WHERE id = $1 AND dipakai = false AND dicabut = false")
        .bind(id)
        .execute(db)
        .await?
        .rows_affected();
    if menang == 0 {
        sqlx::query("UPDATE refresh_token SET dicabut = true WHERE keluarga_token = $1")
            .bind(keluarga)
            .execute(db)
            .await?;
        return Err(tak_valid());
    }
    let id_keluarga: Option<String> =
        sqlx::query_scalar("SELECT id_keluarga FROM pengguna WHERE id_pengguna = $1")
            .bind(&id_pengguna)
            .fetch_optional(db)
            .await?
            .flatten();
    terbitkan_di_keluarga(
        db,
        rahasia_jwt,
        &id_pengguna,
        id_keluarga.as_deref(),
        keluarga,
        sekarang,
    )
    .await
}
