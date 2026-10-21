//! `GET /v1/stok?idKeluarga=` — proksi baca stok Pedaree (cache 60 detik, header `X-Data-Usang`).

use crate::{
    auth::{Jenis, Pemanggil},
    dto::{id_keluarga_valid, Data},
    event, AppError, AppResult, AppState,
};
use axum::{
    extract::{Query, State},
    http::{header::HeaderName, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use uuid::Uuid;

pub async fn baca(
    State(s): State<AppState>,
    p: Pemanggil,
    Query(q): Query<HashMap<String, String>>,
) -> AppResult<Response> {
    let p = p.wajib(&[Jenis::Bearer, Jenis::Perangkat, Jenis::Layanan])?;
    let id = q
        .get("idKeluarga")
        .filter(|v| id_keluarga_valid(v))
        .ok_or_else(|| AppError::validasi("idKeluarga", "wajib, format KEL-NNNN"))?
        .clone();
    p.boleh_keluarga(&id, "stok:baca")?;

    let ada: bool =
        sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM profil_keluarga WHERE id_keluarga = $1)")
            .bind(&id)
            .fetch_one(&s.db)
            .await?;
    if !ada {
        return Err(AppError::ProfilTidakAda(format!("Profil {id} tidak ada.")));
    }

    let hasil = s.pedaree.stok(&id).await?;
    if hasil.segar_dari_sumber {
        let payload = json!({ "jumlahItem": hasil.data.len() });
        let _ =
            sqlx::query("INSERT INTO sinyal_pedaree (id, id_keluarga, payload) VALUES ($1,$2,$3)")
                .bind(Uuid::now_v7())
                .bind(&id)
                .bind(&payload)
                .execute(&s.db)
                .await
                .map_err(|e| tracing::error!(error = %e, "gagal menyimpan sinyal_pedaree"));
        event::catat(&s.db, event::SINYAL_DITERBITKAN, &id, payload).await;
    }
    let mut r = Json(Data { data: hasil.data }).into_response();
    if hasil.usang {
        r.headers_mut().insert(
            HeaderName::from_static("x-data-usang"),
            HeaderValue::from_static("true"),
        );
    }
    Ok(r)
}
