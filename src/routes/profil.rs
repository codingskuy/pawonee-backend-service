//! `PUT /v1/profil/{idKeluarga}` — idempoten berdasarkan `versiProfil`.

use crate::{
    auth::{Jenis, Pemanggil},
    dto::{id_keluarga_valid, Data, Komposisi, ProfilDto},
    ekstrak::JsonValid,
    event, AppError, AppResult, AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

type Baris = (
    i32,
    i16,
    Option<Value>,
    Vec<String>,
    Vec<String>,
    i64,
    i16,
    Vec<String>,
    String,
);

const KOLOM: &str = "versi_profil, jumlah_jiwa, komposisi, alergi, pantangan, budget_per_makan, level_pedas, alat_masak, sumber_belanja";

fn dari_baris(id: &str, b: Baris) -> (i64, ProfilDto) {
    let komposisi: Option<Komposisi> = b.2.and_then(|v| serde_json::from_value(v).ok());
    (
        b.0 as i64,
        ProfilDto {
            id_keluarga: id.into(),
            versi_profil: Some(b.0 as i64),
            jumlah_jiwa: b.1 as i64,
            komposisi,
            alergi: b.3,
            pantangan: b.4,
            budget_per_makan: b.5,
            level_pedas: b.6 as i64,
            alat_masak: b.7,
            sumber_belanja: Some(b.8),
        }
        .normal(),
    )
}

/// Isi profil tanpa versi, supaya perbandingan idempotensi hanya menilai isi.
fn isi(mut p: ProfilDto) -> ProfilDto {
    p.versi_profil = None;
    if p.sumber_belanja.is_none() {
        p.sumber_belanja = Some("campuran".into());
    }
    p.normal()
}

pub async fn simpan(
    State(s): State<AppState>,
    p: Pemanggil,
    Path(id): Path<String>,
    JsonValid(masuk): JsonValid<ProfilDto>,
) -> AppResult<Json<Data<Value>>> {
    let p = p.wajib(&[Jenis::Bearer])?;
    if !id_keluarga_valid(&id) {
        return Err(AppError::validasi("idKeluarga", "format KEL-NNNN"));
    }
    if masuk.id_keluarga != id {
        return Err(AppError::validasi(
            "idKeluarga",
            "harus sama dengan parameter path",
        ));
    }
    masuk.periksa()?;

    let mut tx = s.db.begin().await?;
    let ada: Option<Baris> = sqlx::query_as(&format!(
        "SELECT {KOLOM} FROM profil_keluarga WHERE id_keluarga = $1 FOR UPDATE"
    ))
    .bind(&id)
    .fetch_optional(&mut *tx)
    .await?;

    let baru_versi: i64;
    match ada {
        None => {
            // Pembuatan pertama: akun belum terikat keluarga boleh mengikatnya (FR-301: 1 akun 1 keluarga).
            match &p {
                Pemanggil::Pengguna {
                    id_keluarga: Some(k),
                    ..
                } if *k != id => {
                    return Err(AppError::TidakTerotentikasi(
                        "Tidak berhak atas data keluarga ini.".into(),
                    ));
                }
                _ => {}
            }
            baru_versi = masuk.versi_profil.unwrap_or(1);
            let m = masuk.clone().normal();
            sqlx::query(
                "INSERT INTO profil_keluarga (id_keluarga, versi_profil, jumlah_jiwa, komposisi, alergi, pantangan, budget_per_makan, level_pedas, alat_masak, sumber_belanja) \
                 VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)",
            )
            .bind(&id)
            .bind(baru_versi as i32)
            .bind(m.jumlah_jiwa as i16)
            .bind(m.komposisi.as_ref().map(|k| serde_json::to_value(k).unwrap_or(Value::Null)))
            .bind(&m.alergi)
            .bind(&m.pantangan)
            .bind(m.budget_per_makan)
            .bind(m.level_pedas as i16)
            .bind(&m.alat_masak)
            .bind(m.sumber_belanja.as_deref().unwrap_or("campuran"))
            .execute(&mut *tx)
            .await?;
            if let Pemanggil::Pengguna {
                id: uid,
                id_keluarga: None,
            } = &p
            {
                let r = sqlx::query("UPDATE pengguna SET id_keluarga = $2 WHERE id_pengguna = $1 AND id_keluarga IS NULL")
                    .bind(uid)
                    .bind(&id)
                    .execute(&mut *tx)
                    .await?;
                if r.rows_affected() == 0 {
                    tracing::debug!(pengguna = %uid, "akun tidak ditemukan di tabel pengguna; keluarga tidak diikat");
                }
            }
        }
        Some(baris) => {
            p.boleh_keluarga(&id, "profil:tulis")?;
            let (versi_tersimpan, tersimpan) = dari_baris(&id, baris);
            let sama = isi(tersimpan) == isi(masuk.clone());
            match masuk.versi_profil {
                Some(v) if v < versi_tersimpan => {
                    return Err(AppError::Konflik(
                        "Versi profil lebih lama dari yang tersimpan.".into(),
                    ));
                }
                Some(v) if v == versi_tersimpan => {
                    if sama {
                        // Idempoten: kirim ulang yang sama → 200 tanpa perubahan/duplikat.
                        return Ok(Json(Data {
                            data: json!({ "tersimpan": true }),
                        }));
                    }
                    return Err(AppError::Konflik(
                        "Versi sama tetapi isi berbeda; naikkan versiProfil.".into(),
                    ));
                }
                None if sama => {
                    return Ok(Json(Data {
                        data: json!({ "tersimpan": true }),
                    }))
                }
                _ => {}
            }
            baru_versi = masuk.versi_profil.unwrap_or(versi_tersimpan + 1);
            let m = masuk.clone().normal();
            sqlx::query(
                "UPDATE profil_keluarga SET versi_profil=$2, jumlah_jiwa=$3, komposisi=$4, alergi=$5, pantangan=$6, \
                 budget_per_makan=$7, level_pedas=$8, alat_masak=$9, sumber_belanja=$10, diperbarui_pada=now() WHERE id_keluarga=$1",
            )
            .bind(&id)
            .bind(baru_versi as i32)
            .bind(m.jumlah_jiwa as i16)
            .bind(m.komposisi.as_ref().map(|k| serde_json::to_value(k).unwrap_or(Value::Null)))
            .bind(&m.alergi)
            .bind(&m.pantangan)
            .bind(m.budget_per_makan)
            .bind(m.level_pedas as i16)
            .bind(&m.alat_masak)
            .bind(m.sumber_belanja.as_deref().unwrap_or("campuran"))
            .execute(&mut *tx)
            .await?;
        }
    }
    tx.commit().await?;
    event::catat(
        &s.db,
        event::PROFIL_DISIMPAN,
        &id,
        json!({ "versiProfil": baru_versi }),
    )
    .await;
    Ok(Json(Data {
        data: json!({ "tersimpan": true }),
    }))
}
