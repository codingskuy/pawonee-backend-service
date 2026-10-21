use axum::{
    body::Body,
    http::{header, Request, StatusCode},
    routing::get,
    Json, Router,
};
use chrono::{Duration, Utc};
use http_body_util::BodyExt;
use pawonee::{
    auth::{self, jwt, kunci, pin, sandi, Jenis, Pemanggil},
    routes, AppState,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

const R: &str = "rahasia-uji-yang-cukup-panjang-123456";

async fn kirim(app: Router, req: Request<Body>) -> (StatusCode, Value) {
    let resp = app.oneshot(req).await.unwrap();
    let st = resp.status();
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (st, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::post(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn buat_pengguna(pool: &PgPool, id: &str, sandi_teks: &str, kel: Option<&str>) {
    sqlx::query("INSERT INTO pengguna (id_pengguna, id_keluarga, hash_sandi) VALUES ($1,$2,$3)")
        .bind(id)
        .bind(kel)
        .bind(sandi::hash(sandi_teks).unwrap())
        .execute(pool)
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn login_sukses_memberi_pasangan_audiens_pawonee(pool: PgPool) {
    buat_pengguna(&pool, "ibu-sari", "sandi-kuat-1", Some("KEL-0001")).await;
    let app = routes::router(AppState::baru(pool, R), 5);
    let (st, j) = kirim(
        app,
        post(
            "/v1/auth/login",
            json!({"idPengguna":"ibu-sari","kataSandi":"sandi-kuat-1"}),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let d = &j["data"];
    assert_eq!(d["audiens"], "pawonee");
    assert_eq!(d["kedaluwarsaDetik"], 900);
    let k = jwt::verifikasi(R, d["tokenAkses"].as_str().unwrap()).unwrap();
    assert_eq!(k.kel.as_deref(), Some("KEL-0001"));
    assert!(d["tokenRefresh"].as_str().unwrap().len() >= 64);
}

#[sqlx::test(migrations = "./migrations")]
async fn login_gagal_sandi_salah_pengguna_tak_ada_dan_validasi(pool: PgPool) {
    buat_pengguna(&pool, "ibu-sari", "sandi-kuat-1", None).await;
    let app = routes::router(AppState::baru(pool, R), 5);
    let (st, j) = kirim(
        app.clone(),
        post(
            "/v1/auth/login",
            json!({"idPengguna":"ibu-sari","kataSandi":"salah"}),
        ),
    )
    .await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("TIDAK_TEROTENTIKASI"))
    );
    let (st, _) = kirim(
        app.clone(),
        post(
            "/v1/auth/login",
            json!({"idPengguna":"hantu","kataSandi":"x"}),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, j) = kirim(
        app,
        post("/v1/auth/login", json!({"idPengguna":"","kataSandi":"x"})),
    )
    .await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::BAD_REQUEST, Some("VALIDASI_GAGAL"))
    );
}

/// Temuan Schemathesis: karakter NUL pada `idPengguna` membuat Postgres menolak (500). Harus 400.
#[sqlx::test(migrations = "./migrations")]
async fn login_menolak_karakter_nul_dengan_400_bukan_500(pool: PgPool) {
    let app = routes::router(AppState::baru(pool, R), 5);
    for badan in [
        json!({"idPengguna":"\u{0}\u{9f}\u{9f}","kataSandi":"x"}),
        json!({"idPengguna":"ibu-sari","kataSandi":"x","idPerangkat":"hp\u{0}"}),
    ] {
        let (st, j) = kirim(app.clone(), post("/v1/auth/login", badan)).await;
        assert_eq!(
            (st, j["kode"].as_str()),
            (StatusCode::BAD_REQUEST, Some("VALIDASI_GAGAL"))
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_sekali_pakai_dan_reuse_mencabut_seluruh_keluarga(pool: PgPool) {
    buat_pengguna(&pool, "ibu-sari", "sandi-kuat-1", None).await;
    let app = routes::router(AppState::baru(pool, R), 5);
    let (_, j) = kirim(
        app.clone(),
        post(
            "/v1/auth/login",
            json!({"idPengguna":"ibu-sari","kataSandi":"sandi-kuat-1"}),
        ),
    )
    .await;
    let r1 = j["data"]["tokenRefresh"].as_str().unwrap().to_string();

    let (st, j2) = kirim(
        app.clone(),
        post("/v1/auth/refresh", json!({"refreshToken": r1})),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let r2 = j2["data"]["tokenRefresh"].as_str().unwrap().to_string();
    assert_ne!(r1, r2);

    // r1 dipakai ulang -> 401 dan seluruh keluarga (termasuk r2) dicabut.
    let (st, _) = kirim(
        app.clone(),
        post("/v1/auth/refresh", json!({"refreshToken": r1})),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = kirim(
        app.clone(),
        post("/v1/auth/refresh", json!({"refreshToken": r2})),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED, "r2 harus ikut dicabut");
    let (st, _) = kirim(
        app,
        post("/v1/auth/refresh", json!({"refreshToken": "ngawur"})),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_kedaluwarsa_7_hari_ditolak(pool: PgPool) {
    buat_pengguna(&pool, "ibu-sari", "sandi-kuat-1", None).await;
    let lampau = Utc::now() - Duration::days(8);
    let p = auth::refresh::terbitkan_awal(&pool, R, "ibu-sari", None, lampau)
        .await
        .unwrap();
    let hasil = auth::refresh::tukar(&pool, R, &p.token_refresh, Utc::now()).await;
    assert!(hasil.is_err());
    // Tepat di dalam 7 hari masih sah.
    let p =
        auth::refresh::terbitkan_awal(&pool, R, "ibu-sari", None, Utc::now() - Duration::days(6))
            .await
            .unwrap();
    assert!(auth::refresh::tukar(&pool, R, &p.token_refresh, Utc::now())
        .await
        .is_ok());
}

async fn saya(p: Pemanggil) -> Result<Json<Value>, pawonee::AppError> {
    let p = p.wajib(&[Jenis::Bearer, Jenis::Perangkat])?;
    Ok(Json(json!({"jenis": format!("{:?}", p.jenis())})))
}

fn app_uji(pool: PgPool) -> Router {
    Router::new()
        .route("/saya", get(saya))
        .with_state(AppState::baru(pool, R))
}

#[sqlx::test(migrations = "./migrations")]
async fn extractor_menerima_bearer_dan_device_menolak_service_dan_tanpa_kredensial(pool: PgPool) {
    let app = app_uji(pool.clone());
    let tok = jwt::terbitkan(R, "ibu-sari", Some("KEL-0001")).unwrap();
    let (st, j) = kirim(
        app.clone(),
        Request::get("/saya")
            .header("authorization", format!("Bearer {tok}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!((st, j["jenis"].as_str()), (StatusCode::OK, Some("Bearer")));

    let dev = kunci::daftarkan_perangkat(&pool, "hp-1", Some("KEL-0001"))
        .await
        .unwrap();
    let (st, j) = kirim(
        app.clone(),
        Request::get("/saya")
            .header("x-device-key", &dev)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(
        (st, j["jenis"].as_str()),
        (StatusCode::OK, Some("Perangkat"))
    );

    let svc = kunci::terbitkan_service_key(&pool, "pedaree", &["stok:baca"], Utc::now())
        .await
        .unwrap();
    let (st, _) = kirim(
        app.clone(),
        Request::get("/saya")
            .header("x-service-key", &svc)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::UNAUTHORIZED,
        "service key tidak boleh di endpoint ini"
    );

    let (st, _) = kirim(
        app.clone(),
        Request::get("/saya").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = kirim(
        app.clone(),
        Request::get("/saya")
            .header("authorization", "Bearer sampah")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = kirim(
        app,
        Request::get("/saya")
            .header("x-device-key", "salah")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn batas_30_per_menit_per_token_lalu_429_dengan_retry_60(pool: PgPool) {
    let app = app_uji(pool);
    let tok = jwt::terbitkan(R, "ibu-sari", None).unwrap();
    let req = || {
        Request::get("/saya")
            .header("authorization", format!("Bearer {tok}"))
            .body(Body::empty())
            .unwrap()
    };
    for _ in 0..30 {
        assert_eq!(kirim(app.clone(), req()).await.0, StatusCode::OK);
    }
    let (st, j) = kirim(app.clone(), req()).await;
    assert_eq!(st, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(j["kode"], "BATAS_LAJU");
    assert_eq!(j["retrySetelahDetik"], 60);
    // token lain tidak terdampak
    let lain = jwt::terbitkan(R, "bapak", None).unwrap();
    let (st, _) = kirim(
        app,
        Request::get("/saya")
            .header("authorization", format!("Bearer {lain}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn service_key_kedaluwarsa_setelah_90_hari_dan_dicabut(pool: PgPool) {
    let sekarang = Utc::now();
    let k = kunci::terbitkan_service_key(&pool, "pedaree", &["stok:baca"], sekarang)
        .await
        .unwrap();
    assert!(
        kunci::cari_service_key(&pool, &k, sekarang + Duration::days(89))
            .await
            .unwrap()
            .is_some()
    );
    assert!(
        kunci::cari_service_key(&pool, &k, sekarang + Duration::days(91))
            .await
            .unwrap()
            .is_none()
    );
    sqlx::query("UPDATE service_key SET dicabut = true")
        .execute(&pool)
        .await
        .unwrap();
    assert!(kunci::cari_service_key(&pool, &k, sekarang)
        .await
        .unwrap()
        .is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn pin_tiga_salah_mengunci_15_menit_lalu_terbuka(pool: PgPool) {
    buat_pengguna(&pool, "ibu-sari", "sandi-kuat-1", None).await;
    pin::atur_pin(&pool, "ibu-sari", "123456").await.unwrap();
    let t = Utc::now();
    for _ in 0..3 {
        assert!(pin::verifikasi(&pool, "ibu-sari", "000000", t)
            .await
            .is_err());
    }
    // Terkunci: PIN benar pun ditolak dengan BATAS_LAJU.
    let e = pin::verifikasi(&pool, "ibu-sari", "123456", t + Duration::minutes(5))
        .await
        .unwrap_err();
    assert_eq!(e.kode(), "BATAS_LAJU");
    // Setelah 15 menit terbuka.
    assert!(
        pin::verifikasi(&pool, "ibu-sari", "123456", t + Duration::minutes(16))
            .await
            .is_ok()
    );
    // PIN lewat 90 hari wajib diganti.
    let e = pin::verifikasi(&pool, "ibu-sari", "123456", t + Duration::days(91))
        .await
        .unwrap_err();
    assert_eq!(e.kode(), "TAK_TERPROSES");
}

#[sqlx::test(migrations = "./migrations")]
async fn migrasi_balik_0003(pool: PgPool) {
    sqlx::raw_sql(include_str!("../db/down/0003_auth.down.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM information_schema.tables WHERE table_name IN ('pengguna','refresh_token','api_key_perangkat','service_key')")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(n, 0);
}

#[test]
fn boleh_keluarga_membatasi_ke_keluarga_sendiri() {
    let p = Pemanggil::Pengguna {
        id: "a".into(),
        id_keluarga: Some("KEL-0001".into()),
    };
    assert!(p.boleh_keluarga("KEL-0001", "stok:baca").is_ok());
    assert!(p.boleh_keluarga("KEL-0002", "stok:baca").is_err());
    let l = Pemanggil::Layanan(kunci::Layanan {
        id: "x".into(),
        cakupan: vec!["stok:baca".into()],
    });
    assert!(l.boleh_keluarga("KEL-0002", "stok:baca").is_ok());
    assert!(l.boleh_keluarga("KEL-0002", "profil:tulis").is_err());
}
