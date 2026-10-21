use pawonee::{config::Config, db, routes, AppState};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();

    let cfg = match Config::dari_env() {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("{e}");
            std::process::exit(1);
        }
    };
    let pool = db::sambung(&cfg.database_url)
        .await
        .expect("koneksi database pawonee gagal");
    db::migrasi(&pool).await.expect("migrasi gagal");

    let app = routes::router(
        AppState::baru(pool, &cfg.jwt_rahasia).dengan_pedaree(pawonee::klien::Pedaree::baru(
            cfg.pedaree_basis.clone(),
            cfg.pedaree_kunci_layanan.clone(),
        )),
        cfg.timeout_detik,
    );
    let pendengar = tokio::net::TcpListener::bind(cfg.alamat)
        .await
        .expect("bind gagal");
    tracing::info!(alamat = %cfg.alamat, "pawonee berjalan");
    axum::serve(pendengar, app).await.expect("server berhenti");
}
