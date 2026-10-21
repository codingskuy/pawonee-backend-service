//! Klien baca ke Pedaree inventori (stok). Cache 60 detik; bila Pedaree mati, pakai data tersimpan
//! (usang). Pawonee tidak pernah mengubah stok.
//!
//! ASUMSI: Pedaree menyajikan adaptor `GET {PEDAREE_BASE_URL}/stok?idKeluarga=KEL-NNNN` yang
//! membalas `{"data":[StokItem]}`; `pedaree-openapi.yaml` belum memuat endpoint itu (dilaporkan).

use crate::{dto::StokItemDto, AppError, AppResult};
use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};

const TTL: Duration = Duration::from_secs(60);

pub struct Pedaree {
    basis: Option<String>,
    kunci_layanan: Option<String>,
    http: reqwest::Client,
    ttl: Duration,
    cache: Mutex<HashMap<String, (Instant, Vec<StokItemDto>)>>,
}

pub struct HasilStok {
    pub data: Vec<StokItemDto>,
    pub usang: bool,
    /// `true` bila data baru diambil dari Pedaree (bukan cache segar).
    pub segar_dari_sumber: bool,
}

#[derive(serde::Deserialize)]
struct Amplop {
    data: Vec<StokItemDto>,
}

impl Pedaree {
    pub fn baru(basis: Option<String>, kunci_layanan: Option<String>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .unwrap_or_default();
        Self {
            basis,
            kunci_layanan,
            http,
            ttl: TTL,
            cache: Mutex::new(HashMap::new()),
        }
    }

    /// Ubah umur cache (uji).
    pub fn dengan_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    fn dari_cache(&self, id: &str, boleh_usang: bool) -> Option<Vec<StokItemDto>> {
        let c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
        c.get(id)
            .filter(|(t, _)| boleh_usang || t.elapsed() < self.ttl)
            .map(|(_, d)| d.clone())
    }

    pub async fn stok(&self, id_keluarga: &str) -> AppResult<HasilStok> {
        if let Some(data) = self.dari_cache(id_keluarga, false) {
            return Ok(HasilStok {
                data,
                usang: false,
                segar_dari_sumber: false,
            });
        }
        match self.ambil(id_keluarga).await {
            Ok(data) => {
                let mut c = self.cache.lock().unwrap_or_else(|e| e.into_inner());
                c.insert(id_keluarga.to_string(), (Instant::now(), data.clone()));
                Ok(HasilStok {
                    data,
                    usang: false,
                    segar_dari_sumber: true,
                })
            }
            Err(e) => match self.dari_cache(id_keluarga, true) {
                Some(data) => {
                    tracing::warn!(error = %e, "Pedaree tidak terjangkau; memakai data tersimpan");
                    Ok(HasilStok {
                        data,
                        usang: true,
                        segar_dari_sumber: false,
                    })
                }
                None => Err(e),
            },
        }
    }

    async fn ambil(&self, id_keluarga: &str) -> AppResult<Vec<StokItemDto>> {
        let basis = self
            .basis
            .as_deref()
            .ok_or_else(|| AppError::JaringanGagal("Pedaree belum dikonfigurasi.".into()))?;
        let mut req = self
            .http
            .get(format!("{}/stok", basis.trim_end_matches('/')))
            .query(&[("idKeluarga", id_keluarga)])
            .header("X-Kontrak-Versi", crate::VERSI_KONTRAK);
        if let Some(k) = &self.kunci_layanan {
            req = req.header("X-Service-Key", k);
        }
        let gagal = |e: String| AppError::JaringanGagal(format!("Pedaree tidak terjangkau: {e}"));
        let resp = req.send().await.map_err(|e| gagal(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(gagal(format!("status {}", resp.status())));
        }
        let a: Amplop = resp.json().await.map_err(|e| gagal(e.to_string()))?;
        Ok(a.data)
    }
}
