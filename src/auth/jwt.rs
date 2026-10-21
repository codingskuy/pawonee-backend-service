//! JWT akses: HS256, umur 15 menit, audiens `pawonee`.

use crate::AppError;
use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

pub const AUDIENS: &str = "pawonee";
pub const UMUR_AKSES_DETIK: i64 = 15 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Klaim {
    pub sub: String,
    pub aud: String,
    pub exp: i64,
    pub iat: i64,
    pub jti: String,
    /// Id keluarga pemilik akun (FR-301: hanya melihat datanya sendiri).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kel: Option<String>,
}

pub fn terbitkan(rahasia: &str, sub: &str, kel: Option<&str>) -> Result<String, AppError> {
    terbitkan_pada(rahasia, sub, kel, Utc::now().timestamp())
}

pub fn terbitkan_pada(
    rahasia: &str,
    sub: &str,
    kel: Option<&str>,
    iat: i64,
) -> Result<String, AppError> {
    let k = Klaim {
        sub: sub.into(),
        aud: AUDIENS.into(),
        exp: iat + UMUR_AKSES_DETIK,
        iat,
        jti: uuid::Uuid::now_v7().to_string(),
        kel: kel.map(Into::into),
    };
    encode(
        &Header::new(Algorithm::HS256),
        &k,
        &EncodingKey::from_secret(rahasia.as_bytes()),
    )
    .map_err(|e| AppError::Server(format!("jwt: {e}")))
}

pub fn verifikasi(rahasia: &str, token: &str) -> Result<Klaim, AppError> {
    let mut v = Validation::new(Algorithm::HS256);
    v.set_audience(&[AUDIENS]);
    v.set_required_spec_claims(&["exp", "aud", "sub"]);
    v.leeway = 0;
    decode::<Klaim>(token, &DecodingKey::from_secret(rahasia.as_bytes()), &v)
        .map(|d| d.claims)
        .map_err(|_| AppError::TidakTerotentikasi("Token tidak valid.".into()))
}

#[cfg(test)]
mod uji {
    use super::*;

    const R: &str = "rahasia-uji-yang-cukup-panjang-123456";

    #[test]
    fn putaran_penuh_dan_umur_15_menit() {
        let t = terbitkan(R, "pengguna-1", Some("KEL-0001")).unwrap();
        let k = verifikasi(R, &t).unwrap();
        assert_eq!(k.sub, "pengguna-1");
        assert_eq!(k.aud, "pawonee");
        assert_eq!(k.exp - k.iat, 900);
        assert_eq!(k.kel.as_deref(), Some("KEL-0001"));
    }

    #[test]
    fn kedaluwarsa_ditolak() {
        let t = terbitkan_pada(R, "p", None, Utc::now().timestamp() - 1000).unwrap();
        assert!(verifikasi(R, &t).is_err());
    }

    #[test]
    fn audiens_lain_ditolak() {
        let k = Klaim {
            sub: "p".into(),
            aud: "pedaree".into(),
            exp: Utc::now().timestamp() + 600,
            iat: 0,
            jti: "x".into(),
            kel: None,
        };
        let t = encode(
            &Header::default(),
            &k,
            &EncodingKey::from_secret(R.as_bytes()),
        )
        .unwrap();
        assert!(verifikasi(R, &t).is_err());
    }

    #[test]
    fn rahasia_salah_ditolak() {
        let t = terbitkan(R, "p", None).unwrap();
        assert!(verifikasi("rahasia-lain-yang-juga-panjang-9999999", &t).is_err());
    }
}
