//! Uji integrasi endpoint v1 (PWN-BE-004) di atas DB nyata (`#[sqlx::test]`).

use axum::{
    body::Body,
    http::{header, HeaderMap, Request, StatusCode},
    routing::get,
    Json, Router,
};
use http_body_util::BodyExt;
use pawonee::{auth::jwt, klien::Pedaree, routes, AppState};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::time::Duration;
use tower::ServiceExt;

const R: &str = "rahasia-uji-yang-cukup-panjang-123456";

fn app(pool: PgPool) -> Router {
    routes::router(AppState::baru(pool, R), 5)
}

fn token(kel: Option<&str>) -> String {
    jwt::terbitkan(R, "ibu-sari", kel).unwrap()
}

async fn kirim(app: &Router, req: Request<Body>) -> (StatusCode, HeaderMap, Value) {
    let resp = app.clone().oneshot(req).await.unwrap();
    let (st, h) = (resp.status(), resp.headers().clone());
    let b = resp.into_body().collect().await.unwrap().to_bytes();
    (st, h, serde_json::from_slice(&b).unwrap_or(Value::Null))
}

fn get_(uri: &str, tok: Option<&str>) -> Request<Body> {
    let mut r = Request::get(uri);
    if let Some(t) = tok {
        r = r.header(header::AUTHORIZATION, format!("Bearer {t}"));
    }
    r.body(Body::empty()).unwrap()
}

fn kirim_json(metode: &str, uri: &str, tok: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method(metode)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::AUTHORIZATION, format!("Bearer {tok}"))
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn profil(id: &str, versi: Option<i64>) -> Value {
    let mut p = json!({
        "idKeluarga": id, "jumlahJiwa": 4, "alergi": [], "pantangan": ["halal-selalu"],
        "budgetPerMakan": 35000, "levelPedas": 1, "alatMasak": ["kompor", "wajan", "panci", "kukusan"]
    });
    if let Some(v) = versi {
        p["versiProfil"] = json!(v);
    }
    p
}

async fn jumlah_event(pool: &PgPool, jenis: &str) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM event_log WHERE jenis = $1 AND versi_kontrak = 'v1' AND id_keluarga LIKE 'KEL-%'")
        .bind(jenis)
        .fetch_one(pool)
        .await
        .unwrap()
}

// ---------- resep ----------

#[sqlx::test(migrations = "./migrations")]
async fn daftar_resep_12_dengan_cache_7_hari_dan_header_versi(pool: PgPool) {
    let a = app(pool);
    let (st, h, j) = kirim(&a, get_("/v1/resep", Some(&token(None)))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(j["data"].as_array().unwrap().len(), 12);
    assert_eq!(h["cache-control"], "public, max-age=604800");
    assert_eq!(h["x-kontrak-versi"], "v1");
    assert_eq!(j["data"][0]["id"], "R-01");
    assert_eq!(j["data"][11]["id"], "R-12");
    let (st, _, j) = kirim(&a, get_("/v1/resep", None)).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("TIDAK_TEROTENTIKASI"))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn detail_resep_terskala_dan_galat(pool: PgPool) {
    let a = app(pool);
    let t = token(None);
    let (st, h, j) = kirim(&a, get_("/v1/resep/R-06?jiwa=2", Some(&t))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(h["cache-control"], "public, max-age=86400");
    let bahan = j["data"]["bahanTerskala"].as_array().unwrap();
    let kangkung = bahan.iter().find(|b| b["namaBahan"] == "kangkung").unwrap();
    assert_eq!(kangkung["gram"], 200); // 400 x 0,55 = 220 -> kelipatan 50
                                       // bawaan 4 jiwa = takaran dasar
    let (_, _, j) = kirim(&a, get_("/v1/resep/R-06", Some(&t))).await;
    let k = j["data"]["bahanTerskala"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["namaBahan"] == "kangkung")
        .unwrap()
        .clone();
    assert_eq!(k["gram"], 400);

    let (st, _, j) = kirim(&a, get_("/v1/resep/R-06?jiwa=9", Some(&t))).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("JIWA_DI_LUAR_RENTANG")
        )
    );
    let (st, _, _) = kirim(&a, get_("/v1/resep/R-06?jiwa=0", Some(&t))).await;
    assert_eq!(st, StatusCode::UNPROCESSABLE_ENTITY);
    let (st, _, j) = kirim(&a, get_("/v1/resep/R-99", Some(&t))).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::NOT_FOUND, Some("RESEP_TIDAK_ADA"))
    );
    let (st, _, j) = kirim(&a, get_("/v1/resep/R-06?jiwa=abc", Some(&t))).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::BAD_REQUEST, Some("VALIDASI_GAGAL"))
    );
    let (st, _, _) = kirim(&a, get_("/v1/resep/ngawur", Some(&t))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn header_kontrak_versi_selain_v1_ditolak(pool: PgPool) {
    let a = app(pool);
    let r = Request::get("/v1/kesehatan")
        .header("x-kontrak-versi", "v2")
        .body(Body::empty())
        .unwrap();
    let (st, _, j) = kirim(&a, r).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::BAD_REQUEST, Some("VALIDASI_GAGAL"))
    );
}

// ---------- rekomendasi ----------

fn req_rekom(stok: Value, jiwa: i64, waktu: i64) -> Value {
    json!({ "profil": profil("KEL-0001", None), "stok": stok, "jiwa": jiwa, "waktuMaksimalMenit": waktu })
}

#[sqlx::test(migrations = "./migrations")]
async fn rekomendasi_stok_kosong_mode_hemat_dan_event(pool: PgPool) {
    let a = app(pool.clone());
    let t = token(Some("KEL-0001"));
    let (st, _, j) = kirim(
        &a,
        kirim_json("POST", "/v1/rekomendasi", &t, req_rekom(json!([]), 4, 30)),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(j["modeTanpaStok"], true);
    let data = j["data"].as_array().unwrap();
    assert!(!data.is_empty() && data.len() <= 5);
    assert_eq!(jumlah_event(&pool, "rekomendasi.dihitung").await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn rekomendasi_dengan_stok_maks_5_belanja_dalam_budget(pool: PgPool) {
    let a = app(pool);
    let t = token(Some("KEL-0001"));
    let stok = json!([
        {"namaBahan":"kentang mini","gram":600,"kedaluwarsaHari":5,"kodeGrade":"G2-U"},
        {"namaBahan":"wortel bengkok","gram":400,"kedaluwarsaHari":4,"kodeGrade":"G2-V"},
        {"namaBahan":"kangkung","gram":500,"kedaluwarsaHari":2,"kodeGrade":"G2-S"}
    ]);
    let (st, _, j) = kirim(
        &a,
        kirim_json("POST", "/v1/rekomendasi", &t, req_rekom(stok, 4, 30)),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(j["modeTanpaStok"], false);
    let data = j["data"].as_array().unwrap();
    assert!(!data.is_empty() && data.len() <= 5);
    for d in data {
        let total: i64 = d["belanjaTambahan"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["perkiraanRp"].as_i64().unwrap())
            .sum();
        assert!(total <= 35000, "belanja {total} melebihi budget");
        assert!(d["skorTotal"].as_f64().unwrap() <= 100.0);
        assert!(!d["alasan"].as_str().unwrap().is_empty());
    }
    let skor: Vec<f64> = data
        .iter()
        .map(|d| d["skorTotal"].as_f64().unwrap())
        .collect();
    assert!(skor.windows(2).all(|w| w[0] >= w[1]));
}

#[sqlx::test(migrations = "./migrations")]
async fn rekomendasi_validasi_galat(pool: PgPool) {
    let a = app(pool);
    let t = token(Some("KEL-0001"));
    let (st, _, j) = kirim(
        &a,
        kirim_json("POST", "/v1/rekomendasi", &t, req_rekom(json!([]), 4, 5)),
    )
    .await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::BAD_REQUEST, Some("VALIDASI_GAGAL"))
    );
    assert_eq!(j["rinci"][0]["bidang"], "waktuMaksimalMenit");
    let (st, _, j) = kirim(
        &a,
        kirim_json("POST", "/v1/rekomendasi", &t, req_rekom(json!([]), 9, 30)),
    )
    .await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("JIWA_DI_LUAR_RENTANG")
        )
    );
    let stok = json!([{"namaBahan":"x","gram":-1,"kedaluwarsaHari":1,"kodeGrade":"G2-X"}]);
    let (st, _, j) = kirim(
        &a,
        kirim_json("POST", "/v1/rekomendasi", &t, req_rekom(stok, 4, 30)),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    assert_eq!(j["rinci"].as_array().unwrap().len(), 2);
    let (st, _, _) = kirim(
        &a,
        kirim_json("POST", "/v1/rekomendasi", &t, json!({"ngawur": 1})),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    // FR-301: keluarga lain ditolak
    let lain = token(Some("KEL-0002"));
    let (st, _, _) = kirim(
        &a,
        kirim_json(
            "POST",
            "/v1/rekomendasi",
            &lain,
            req_rekom(json!([]), 4, 30),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

// ---------- profil ----------

#[sqlx::test(migrations = "./migrations")]
async fn put_profil_idempoten_tanpa_duplikat(pool: PgPool) {
    let a = app(pool.clone());
    let t = token(Some("KEL-0001"));
    let uri = "/v1/profil/KEL-0001";
    let (st, _, j) = kirim(&a, kirim_json("PUT", uri, &t, profil("KEL-0001", Some(1)))).await;
    assert_eq!(
        (st, &j["data"]["tersimpan"]),
        (StatusCode::OK, &json!(true))
    );
    // kirim ulang sama → 200 tanpa duplikat/event baru
    let (st, _, _) = kirim(&a, kirim_json("PUT", uri, &t, profil("KEL-0001", Some(1)))).await;
    assert_eq!(st, StatusCode::OK);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM profil_keluarga")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(jumlah_event(&pool, "profil.disimpan").await, 1);
    // versi sama isi beda → 409
    let mut beda = profil("KEL-0001", Some(1));
    beda["budgetPerMakan"] = json!(40000);
    let (st, _, j) = kirim(&a, kirim_json("PUT", uri, &t, beda.clone())).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::CONFLICT, Some("KONFLIK"))
    );
    // versi baru → terupdate
    beda["versiProfil"] = json!(2);
    let (st, _, _) = kirim(&a, kirim_json("PUT", uri, &t, beda)).await;
    assert_eq!(st, StatusCode::OK);
    let (b, v): (i64, i32) =
        sqlx::query_as("SELECT budget_per_makan, versi_profil FROM profil_keluarga")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!((b, v), (40000, 2));
    assert_eq!(jumlah_event(&pool, "profil.disimpan").await, 2);
    // versi lebih lama → 409
    let (st, _, _) = kirim(&a, kirim_json("PUT", uri, &t, profil("KEL-0001", Some(1)))).await;
    assert_eq!(st, StatusCode::CONFLICT);
}

#[sqlx::test(migrations = "./migrations")]
async fn put_profil_validasi_dan_hak_akses(pool: PgPool) {
    let a = app(pool);
    let t = token(Some("KEL-0001"));
    let (st, _, _) = kirim(
        &a,
        kirim_json("PUT", "/v1/profil/KEL-0001", &t, profil("KEL-0009", None)),
    )
    .await;
    assert_eq!(
        st,
        StatusCode::BAD_REQUEST,
        "id badan harus sama dengan path"
    );
    let mut p = profil("KEL-0001", None);
    p["alergi"] = json!(["durian"]);
    let (st, _, j) = kirim(&a, kirim_json("PUT", "/v1/profil/KEL-0001", &t, p)).await;
    assert_eq!(
        (st, j["rinci"][0]["bidang"].as_str()),
        (StatusCode::BAD_REQUEST, Some("alergi"))
    );
    let mut p = profil("KEL-0001", None);
    p["jumlahJiwa"] = json!(9);
    let (st, _, j) = kirim(&a, kirim_json("PUT", "/v1/profil/KEL-0001", &t, p)).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("JIWA_DI_LUAR_RENTANG")
        )
    );
    // keluarga lain tidak boleh menulis profil ini
    let lain = token(Some("KEL-0002"));
    let (st, _, _) = kirim(
        &a,
        kirim_json(
            "PUT",
            "/v1/profil/KEL-0001",
            &lain,
            profil("KEL-0001", None),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn put_profil_pertama_mengikat_akun_ke_keluarga(pool: PgPool) {
    sqlx::query("INSERT INTO pengguna (id_pengguna, hash_sandi) VALUES ('ibu-sari','x')")
        .execute(&pool)
        .await
        .unwrap();
    let a = app(pool.clone());
    let t = token(None);
    let (st, _, _) = kirim(
        &a,
        kirim_json("PUT", "/v1/profil/KEL-0007", &t, profil("KEL-0007", None)),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    let k: Option<String> =
        sqlx::query_scalar("SELECT id_keluarga FROM pengguna WHERE id_pengguna='ibu-sari'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(k.as_deref(), Some("KEL-0007"));
}

// ---------- stok (Pedaree palsu) ----------

async fn pedaree_palsu(gagal_setelah: usize) -> (String, tokio::task::JoinHandle<()>) {
    // Permintaan ke-(gagal_setelah+1) dan seterusnya dibalas 503 (Pedaree "mati").
    let n = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let r = Router::new().route(
        "/stok",
        get(move || {
            let n = n.clone();
            async move {
                if n.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= gagal_setelah {
                    return Err(StatusCode::SERVICE_UNAVAILABLE);
                }
                Ok(Json(json!({"data":[{"namaBahan":"kentang mini","gram":600,"kedaluwarsaHari":5,"kodeGrade":"G2-U"}]})))
            }
        }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    (
        url,
        tokio::spawn(async move { axum::serve(l, r).await.unwrap() }),
    )
}

async fn buat_profil(a: &Router, id: &str) {
    let t = token(Some(id));
    let (st, _, _) = kirim(
        a,
        kirim_json("PUT", &format!("/v1/profil/{id}"), &t, profil(id, None)),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
}

#[sqlx::test(migrations = "./migrations")]
async fn stok_proksi_pedaree_cache_usang_dan_galat(pool: PgPool) {
    let (url, _server) = pedaree_palsu(1).await;
    let st_app = AppState::baru(pool.clone(), R)
        .dengan_pedaree(Pedaree::baru(Some(url), None).dengan_ttl(Duration::from_millis(0)));
    let a = routes::router(st_app, 5);
    let t = token(Some("KEL-0001"));

    // profil belum ada → 404
    let (st, _, j) = kirim(&a, get_("/v1/stok?idKeluarga=KEL-0001", Some(&t))).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::NOT_FOUND, Some("PROFIL_TIDAK_ADA"))
    );
    // id tak valid → 400
    let (st, _, _) = kirim(&a, get_("/v1/stok?idKeluarga=abc", Some(&t))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    buat_profil(&a, "KEL-0001").await;
    let (st, h, j) = kirim(&a, get_("/v1/stok?idKeluarga=KEL-0001", Some(&t))).await;
    assert_eq!(st, StatusCode::OK);
    assert!(h.get("x-data-usang").is_none());
    assert_eq!(j["data"][0]["namaBahan"], "kentang mini");
    assert_eq!(jumlah_event(&pool, "sinyal.diterbitkan").await, 1);

    // Pedaree mati → data tersimpan dengan X-Data-Usang: true
    let (st, h, j) = kirim(&a, get_("/v1/stok?idKeluarga=KEL-0001", Some(&t))).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(h["x-data-usang"], "true");
    assert_eq!(j["data"][0]["gram"], 600);
    assert_eq!(
        jumlah_event(&pool, "sinyal.diterbitkan").await,
        1,
        "data usang tidak menerbitkan sinyal baru"
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn stok_tanpa_pedaree_dan_tanpa_cache_502_jaringan_gagal(pool: PgPool) {
    let a = app(pool);
    buat_profil(&a, "KEL-0001").await;
    let (st, _, j) = kirim(
        &a,
        get_(
            "/v1/stok?idKeluarga=KEL-0001",
            Some(&token(Some("KEL-0001"))),
        ),
    )
    .await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (StatusCode::BAD_GATEWAY, Some("JARINGAN_GAGAL"))
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn stok_service_key_perlu_cakupan_dan_pengguna_lain_ditolak(pool: PgPool) {
    use pawonee::auth::kunci;
    let (url, _s) = pedaree_palsu(usize::MAX).await;
    let st_app = AppState::baru(pool.clone(), R).dengan_pedaree(Pedaree::baru(Some(url), None));
    let a = routes::router(st_app, 5);
    buat_profil(&a, "KEL-0001").await;

    let lain = token(Some("KEL-0002"));
    let (st, _, _) = kirim(&a, get_("/v1/stok?idKeluarga=KEL-0001", Some(&lain))).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);

    let ok = kunci::terbitkan_service_key(&pool, "pedaree", &["stok:baca"], chrono::Utc::now())
        .await
        .unwrap();
    let r = Request::get("/v1/stok?idKeluarga=KEL-0001")
        .header("x-service-key", ok)
        .body(Body::empty())
        .unwrap();
    assert_eq!(kirim(&a, r).await.0, StatusCode::OK);
    let sempit = kunci::terbitkan_service_key(&pool, "lain", &["profil:tulis"], chrono::Utc::now())
        .await
        .unwrap();
    let r = Request::get("/v1/stok?idKeluarga=KEL-0001")
        .header("x-service-key", sempit)
        .body(Body::empty())
        .unwrap();
    assert_eq!(kirim(&a, r).await.0, StatusCode::UNAUTHORIZED);
}

// ---------- riwayat ----------

#[sqlx::test(migrations = "./migrations")]
async fn riwayat_tandai_dimasak_idempoten_dan_ringkasan(pool: PgPool) {
    let a = app(pool.clone());
    let t = token(Some("KEL-0001"));
    let body = json!({"idKeluarga":"KEL-0001","resepId":"R-06","jiwa":4,"tanggal":"2026-10-20"});

    let (st, _, _) = kirim(
        &a,
        kirim_json("POST", "/v1/riwayat-masak", &t, body.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND, "profil belum ada");

    buat_profil(&a, "KEL-0001").await;
    let (st, _, j) = kirim(
        &a,
        kirim_json("POST", "/v1/riwayat-masak", &t, body.clone()),
    )
    .await;
    assert_eq!(st, StatusCode::CREATED);
    assert_eq!(j["data"]["gramG2Terserap"], 420);
    assert_eq!(jumlah_event(&pool, "resep.dimasak").await, 1);
    assert_eq!(jumlah_event(&pool, "serapan.dihitung").await, 1);

    // tanda ganda hari sama dihitung sekali
    let (st, _, j) = kirim(&a, kirim_json("POST", "/v1/riwayat-masak", &t, body)).await;
    assert_eq!((st, &j["duplikat"]), (StatusCode::OK, &json!(true)));
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM riwayat_masak")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(jumlah_event(&pool, "resep.dimasak").await, 1);

    // klaim gram dari klien diabaikan
    let curang = json!({"idKeluarga":"KEL-0001","resepId":"R-01","jiwa":4,"tanggal":"2026-10-20","gramG2Terserap":99999});
    let (_, _, j) = kirim(&a, kirim_json("POST", "/v1/riwayat-masak", &t, curang)).await;
    assert_eq!(j["data"]["gramG2Terserap"], 60);

    let bad = json!({"idKeluarga":"KEL-0001","resepId":"R-06","jiwa":9,"tanggal":"2026-10-21"});
    let (st, _, j) = kirim(&a, kirim_json("POST", "/v1/riwayat-masak", &t, bad)).await;
    assert_eq!(
        (st, j["kode"].as_str()),
        (
            StatusCode::UNPROCESSABLE_ENTITY,
            Some("JIWA_DI_LUAR_RENTANG")
        )
    );
    let bad = json!({"idKeluarga":"KEL-0001","resepId":"R-77","jiwa":4,"tanggal":"2026-10-21"});
    let (st, _, _) = kirim(&a, kirim_json("POST", "/v1/riwayat-masak", &t, bad)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);

    let (st, _, j) = kirim(
        &a,
        get_(
            "/v1/riwayat-masak?idKeluarga=KEL-0001&mingguTerakhir=104",
            Some(&t),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(j["data"]["riwayat"].as_array().unwrap().len(), 2);
    let mingguan = j["data"]["serapanMingguan"].as_array().unwrap();
    assert_eq!(mingguan.len(), 1);
    assert_eq!(mingguan[0]["mingguMulai"], "2026-10-19");
    assert_eq!(mingguan[0]["serapanGram"], 480);

    let (st, _, _) = kirim(
        &a,
        get_(
            "/v1/riwayat-masak?idKeluarga=KEL-0001&mingguTerakhir=0",
            Some(&t),
        ),
    )
    .await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let lain = token(Some("KEL-0002"));
    let (st, _, _) = kirim(
        &a,
        get_("/v1/riwayat-masak?idKeluarga=KEL-0001", Some(&lain)),
    )
    .await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
}

#[sqlx::test(migrations = "./migrations")]
async fn migrasi_balik_0004(pool: PgPool) {
    sqlx::raw_sql(include_str!("../db/down/0004_event_log.down.sql"))
        .execute(&pool)
        .await
        .unwrap();
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM information_schema.tables WHERE table_name='event_log'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(n, 0);
}
