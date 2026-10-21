use axum::{
    body::Body,
    http::{Request, StatusCode},
    response::IntoResponse,
};
use http_body_util::BodyExt;
use pawonee::{routes, AppError, AppState};
use sqlx::PgPool;
use tower::ServiceExt;

async fn json(resp: axum::response::Response) -> serde_json::Value {
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&b).unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn kesehatan_mengembalikan_baik_dan_v1(pool: PgPool) {
    let app = routes::router(AppState::baru(pool), 5);
    let resp = app
        .oneshot(Request::get("/v1/kesehatan").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let j = json(resp).await;
    assert_eq!(j["status"], "baik");
    assert_eq!(j["kontrakVersi"], "v1");
}

#[sqlx::test(migrations = "./migrations")]
async fn tabel_inti_ada_dan_cek_constraint_jalan(pool: PgPool) {
    for t in ["profil_keluarga", "riwayat_masak", "sinyal_pedaree"] {
        let n: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM information_schema.tables WHERE table_name = $1",
        )
        .bind(t)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(n, 1, "tabel {t} hilang");
    }
    let gagal = sqlx::query(
        "INSERT INTO profil_keluarga (id_keluarga, jumlah_jiwa) VALUES ('KEL-0001', 9)",
    )
    .execute(&pool)
    .await;
    assert!(gagal.is_err(), "jumlah_jiwa 9 harus ditolak");
    sqlx::query("INSERT INTO profil_keluarga (id_keluarga, jumlah_jiwa, alergi) VALUES ('KEL-0001', 4, ARRAY['udang'])")
        .execute(&pool)
        .await
        .unwrap();
}

#[test]
fn app_error_dipetakan_ke_status_baku() {
    let kasus = [
        (AppError::validasi("jiwa", "x"), 400, "VALIDASI_GAGAL"),
        (
            AppError::TidakTerotentikasi("x".into()),
            401,
            "TIDAK_TEROTENTIKASI",
        ),
        (AppError::ResepTidakAda("x".into()), 404, "RESEP_TIDAK_ADA"),
        (
            AppError::ProfilTidakAda("x".into()),
            404,
            "PROFIL_TIDAK_ADA",
        ),
        (AppError::Konflik("x".into()), 409, "KONFLIK"),
        (AppError::JiwaDiLuarRentang, 422, "JIWA_DI_LUAR_RENTANG"),
        (AppError::TakTerproses("x".into()), 422, "TAK_TERPROSES"),
        (AppError::BatasLaju, 429, "BATAS_LAJU"),
        (AppError::Server("rahasia".into()), 500, "KESALAHAN_SERVER"),
    ];
    for (e, status, kode) in kasus {
        assert_eq!(e.status().as_u16(), status);
        assert_eq!(e.kode(), kode);
    }
}

#[tokio::test]
async fn galat_server_tidak_membocorkan_detail_dan_429_memuat_retry() {
    let r = AppError::Server("password=hunter2".into()).into_response();
    assert_eq!(r.status(), 500);
    let j = json(r).await;
    assert!(!j.to_string().contains("hunter2"));
    let r = AppError::BatasLaju.into_response();
    assert_eq!(json(r).await["retrySetelahDetik"], 60);
}

#[sqlx::test(migrations = "./migrations")]
async fn migrasi_balik_menghapus_tabel_0002(pool: PgPool) {
    let down = include_str!("../db/down/0002_profil_riwayat_sinyal.down.sql");
    sqlx::raw_sql(down).execute(&pool).await.unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables WHERE table_name IN ('profil_keluarga','riwayat_masak','sinyal_pedaree')",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0);
}
