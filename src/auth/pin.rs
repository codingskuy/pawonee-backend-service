//! PIN 6 digit: diganti tiap 90 hari; 3 salah beruntun mengunci 15 menit.

use crate::{auth::sandi, AppError, AppResult};
use chrono::{DateTime, Duration, Utc};
use sqlx::PgPool;

/// (hash_pin, pin_diganti_pada, pin_gagal, pin_terkunci_hingga)
type BarisPin = (
    Option<String>,
    Option<DateTime<Utc>>,
    i16,
    Option<DateTime<Utc>>,
);

pub const MAKS_SALAH: i16 = 3;
pub const KUNCI_MENIT: i64 = 15;
pub const GANTI_HARI: i64 = 90;

pub fn format_valid(pin: &str) -> bool {
    pin.len() == 6 && pin.bytes().all(|b| b.is_ascii_digit())
}

pub fn perlu_ganti(diganti_pada: Option<DateTime<Utc>>, sekarang: DateTime<Utc>) -> bool {
    match diganti_pada {
        Some(t) => sekarang - t >= Duration::days(GANTI_HARI),
        None => true,
    }
}

/// Hasil pencatatan salah: (jumlah salah baru, kunci hingga).
pub fn catat_salah(salah: i16, sekarang: DateTime<Utc>) -> (i16, Option<DateTime<Utc>>) {
    let baru = salah + 1;
    if baru >= MAKS_SALAH {
        (0, Some(sekarang + Duration::minutes(KUNCI_MENIT)))
    } else {
        (baru, None)
    }
}

pub async fn atur_pin(db: &PgPool, id_pengguna: &str, pin: &str) -> AppResult<()> {
    if !format_valid(pin) {
        return Err(AppError::validasi("pin", "harus 6 digit angka"));
    }
    let h = sandi::hash(pin)?;
    sqlx::query(
        "UPDATE pengguna SET hash_pin = $2, pin_diganti_pada = now(), pin_gagal = 0, pin_terkunci_hingga = NULL WHERE id_pengguna = $1",
    )
    .bind(id_pengguna)
    .bind(h)
    .execute(db)
    .await?;
    Ok(())
}

/// Verifikasi PIN. Terkunci → 429 `BATAS_LAJU`; salah → 401; PIN kedaluwarsa (90 hari) → 422.
pub async fn verifikasi(
    db: &PgPool,
    id_pengguna: &str,
    pin: &str,
    sekarang: DateTime<Utc>,
) -> AppResult<()> {
    let baris: Option<BarisPin> = sqlx::query_as(
        "SELECT hash_pin, pin_diganti_pada, pin_gagal, pin_terkunci_hingga FROM pengguna WHERE id_pengguna = $1",
    )
    .bind(id_pengguna)
    .fetch_optional(db)
    .await?;
    let Some((hash_pin, diganti, salah, kunci)) = baris else {
        return Err(AppError::TidakTerotentikasi("PIN tidak valid.".into()));
    };
    if kunci.is_some_and(|k| k > sekarang) {
        return Err(AppError::BatasLaju);
    }
    let Some(hash_pin) = hash_pin else {
        return Err(AppError::TidakTerotentikasi("PIN tidak valid.".into()));
    };
    if !format_valid(pin) || !sandi::cocok(pin, &hash_pin) {
        let (baru, kunci_baru) = catat_salah(salah, sekarang);
        sqlx::query(
            "UPDATE pengguna SET pin_gagal = $2, pin_terkunci_hingga = $3 WHERE id_pengguna = $1",
        )
        .bind(id_pengguna)
        .bind(baru)
        .bind(kunci_baru)
        .execute(db)
        .await?;
        return Err(AppError::TidakTerotentikasi("PIN tidak valid.".into()));
    }
    sqlx::query(
        "UPDATE pengguna SET pin_gagal = 0, pin_terkunci_hingga = NULL WHERE id_pengguna = $1",
    )
    .bind(id_pengguna)
    .execute(db)
    .await?;
    if perlu_ganti(diganti, sekarang) {
        return Err(AppError::TakTerproses(
            "PIN kedaluwarsa, wajib diganti (90 hari).".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn format_pin() {
        assert!(format_valid("123456"));
        assert!(!format_valid("12345"));
        assert!(!format_valid("12345a"));
        assert!(!format_valid("1234567"));
    }

    #[test]
    fn kunci_pada_salah_ketiga() {
        let t = Utc::now();
        assert_eq!(catat_salah(0, t), (1, None));
        assert_eq!(catat_salah(1, t), (2, None));
        let (n, k) = catat_salah(2, t);
        assert_eq!(n, 0);
        assert_eq!(k, Some(t + Duration::minutes(15)));
    }

    #[test]
    fn ganti_setelah_90_hari() {
        let t = Utc::now();
        assert!(!perlu_ganti(Some(t - Duration::days(89)), t));
        assert!(perlu_ganti(Some(t - Duration::days(90)), t));
        assert!(perlu_ganti(None, t));
    }
}
