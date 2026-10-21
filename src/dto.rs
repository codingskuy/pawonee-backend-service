//! Tipe permintaan/respons kontrak v1 (camelCase) beserta validasinya.

use crate::{
    domain::{
        bank::ResepDef,
        rekomendasi::{Belanja, Filter, Hasil, Stok},
    },
    error::Rinci,
    AppError, AppResult,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const KODE_GRADE: [&str; 4] = ["G2-V", "G2-U", "G2-S", "NON-G2"];
pub const ALERGI: [&str; 7] = [
    "udang",
    "telur",
    "kacang-tanah",
    "kedelai",
    "susu-sapi",
    "ikan",
    "gluten",
];
pub const PANTANGAN: [&str; 6] = [
    "halal-selalu",
    "vegetarian-nabati",
    "tanpa-santan",
    "rendah-garam",
    "rendah-gula",
    "lunak-lansia",
];
pub const ALAT: [&str; 7] = [
    "kompor",
    "wajan",
    "panci",
    "kukusan",
    "blender",
    "oven",
    "air-fryer",
];
pub const SUMBER_BELANJA: [&str; 3] = ["pedaree-stok", "lumbung-g2", "campuran"];

pub fn id_keluarga_valid(s: &str) -> bool {
    s.len() == 8 && s.starts_with("KEL-") && s[4..].bytes().all(|b| b.is_ascii_digit())
}

pub fn resep_id_valid(s: &str) -> bool {
    matches!(s.strip_prefix("R-"), Some(n) if n.len() == 2 && n.bytes().all(|b| b.is_ascii_digit())
        && (1..=12).contains(&n.parse::<u8>().unwrap_or(0)))
}

fn dalam(nilai: &[String], sah: &[&str], bidang: &str, r: &mut Vec<Rinci>) {
    for v in nilai {
        if !sah.contains(&v.as_str()) {
            r.push(Rinci {
                bidang: bidang.into(),
                pesan: format!("nilai '{v}' tidak dikenal"),
            });
        }
    }
}

pub fn periksa_jiwa(jiwa: i64) -> AppResult<()> {
    if (1..=8).contains(&jiwa) {
        Ok(())
    } else {
        Err(AppError::JiwaDiLuarRentang)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Komposisi {
    #[serde(default)]
    pub dewasa: i64,
    #[serde(default)]
    pub anak: i64,
    #[serde(default)]
    pub balita: i64,
    #[serde(default)]
    pub lansia: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfilDto {
    #[serde(rename = "idKeluarga")]
    pub id_keluarga: String,
    #[serde(
        rename = "versiProfil",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub versi_profil: Option<i64>,
    #[serde(rename = "jumlahJiwa")]
    pub jumlah_jiwa: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub komposisi: Option<Komposisi>,
    pub alergi: Vec<String>,
    pub pantangan: Vec<String>,
    #[serde(rename = "budgetPerMakan")]
    pub budget_per_makan: i64,
    #[serde(rename = "levelPedas")]
    pub level_pedas: i64,
    #[serde(rename = "alatMasak")]
    pub alat_masak: Vec<String>,
    #[serde(
        rename = "sumberBelanja",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sumber_belanja: Option<String>,
}

impl ProfilDto {
    /// Validasi: 400 `VALIDASI_GAGAL` untuk bidang salah; 422 `JIWA_DI_LUAR_RENTANG` untuk jiwa.
    pub fn periksa(&self) -> AppResult<()> {
        let mut r = Vec::new();
        if !id_keluarga_valid(&self.id_keluarga) {
            r.push(Rinci {
                bidang: "idKeluarga".into(),
                pesan: "format KEL-NNNN".into(),
            });
        }
        if self.versi_profil.is_some_and(|v| v < 1) {
            r.push(Rinci {
                bidang: "versiProfil".into(),
                pesan: "minimal 1".into(),
            });
        }
        if let Some(k) = &self.komposisi {
            if [k.dewasa, k.anak, k.balita, k.lansia]
                .iter()
                .any(|x| *x < 0)
            {
                r.push(Rinci {
                    bidang: "komposisi".into(),
                    pesan: "tidak boleh negatif".into(),
                });
            }
        }
        dalam(&self.alergi, &ALERGI, "alergi", &mut r);
        dalam(&self.pantangan, &PANTANGAN, "pantangan", &mut r);
        dalam(&self.alat_masak, &ALAT, "alatMasak", &mut r);
        if let Some(s) = &self.sumber_belanja {
            dalam(
                std::slice::from_ref(s),
                &SUMBER_BELANJA,
                "sumberBelanja",
                &mut r,
            );
        }
        if self.budget_per_makan < 0 {
            r.push(Rinci {
                bidang: "budgetPerMakan".into(),
                pesan: "minimal 0".into(),
            });
        }
        if !(0..=3).contains(&self.level_pedas) {
            r.push(Rinci {
                bidang: "levelPedas".into(),
                pesan: "harus 0 sampai 3".into(),
            });
        }
        if !r.is_empty() {
            return Err(AppError::validasi_banyak(r));
        }
        periksa_jiwa(self.jumlah_jiwa)
    }

    pub fn normal(mut self) -> Self {
        self.alergi.sort();
        self.alergi.dedup();
        self.pantangan.sort();
        self.pantangan.dedup();
        self.alat_masak.sort();
        self.alat_masak.dedup();
        self
    }

    pub fn filter(&self) -> Filter {
        Filter {
            alergi: self.alergi.clone(),
            pantangan: self.pantangan.clone(),
            alat: self.alat_masak.clone(),
            level_pedas: self.level_pedas,
            budget: self.budget_per_makan,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StokItemDto {
    #[serde(rename = "namaBahan")]
    pub nama_bahan: String,
    pub gram: i64,
    #[serde(rename = "kedaluwarsaHari")]
    pub kedaluwarsa_hari: i64,
    #[serde(rename = "kodeGrade")]
    pub kode_grade: String,
}

impl StokItemDto {
    pub fn periksa_daftar(daftar: &[StokItemDto]) -> AppResult<()> {
        let mut r = Vec::new();
        for (i, s) in daftar.iter().enumerate() {
            if s.nama_bahan.trim().is_empty() {
                r.push(Rinci {
                    bidang: format!("stok[{i}].namaBahan"),
                    pesan: "wajib diisi".into(),
                });
            }
            if s.gram < 0 {
                r.push(Rinci {
                    bidang: format!("stok[{i}].gram"),
                    pesan: "minimal 0".into(),
                });
            }
            if !KODE_GRADE.contains(&s.kode_grade.as_str()) {
                r.push(Rinci {
                    bidang: format!("stok[{i}].kodeGrade"),
                    pesan: "salah satu dari G2-V, G2-U, G2-S, NON-G2".into(),
                });
            }
        }
        if r.is_empty() {
            Ok(())
        } else {
            Err(AppError::validasi_banyak(r))
        }
    }

    pub fn ke_domain(&self) -> Stok {
        Stok {
            nama: self.nama_bahan.clone(),
            gram: self.gram,
            kedaluwarsa_hari: self.kedaluwarsa_hari,
            kode_grade: self.kode_grade.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RekomendasiRequest {
    pub profil: ProfilDto,
    pub stok: Vec<StokItemDto>,
    pub jiwa: i64,
    #[serde(rename = "waktuMaksimalMenit", default = "waktu_bawaan")]
    pub waktu_maksimal_menit: i64,
}

fn waktu_bawaan() -> i64 {
    30
}

impl RekomendasiRequest {
    pub fn periksa(&self) -> AppResult<()> {
        let mut r = Vec::new();
        if !(10..=45).contains(&self.waktu_maksimal_menit) {
            r.push(Rinci {
                bidang: "waktuMaksimalMenit".into(),
                pesan: "harus 10 sampai 45".into(),
            });
        }
        if let Err(AppError::Validasi { rinci, .. }) = self.profil.periksa() {
            r.extend(rinci);
        }
        if let Err(AppError::Validasi { rinci, .. }) = StokItemDto::periksa_daftar(&self.stok) {
            r.extend(rinci);
        }
        if !r.is_empty() {
            return Err(AppError::validasi_banyak(r));
        }
        periksa_jiwa(self.jiwa)?;
        self.profil.periksa()
    }
}

#[derive(Serialize)]
pub struct BahanResepDto {
    #[serde(rename = "namaBahan")]
    pub nama_bahan: &'static str,
    #[serde(rename = "gramDasar4Porsi")]
    pub gram_dasar_4_porsi: i64,
    #[serde(rename = "kodeGrade")]
    pub kode_grade: &'static str,
    pub opsional: bool,
}

#[derive(Serialize)]
pub struct ResepDto {
    pub id: &'static str,
    pub nama: &'static str,
    pub kelompok: &'static str,
    #[serde(rename = "waktuMenit")]
    pub waktu_menit: i64,
    #[serde(rename = "kisaranBiayaRp")]
    pub kisaran_biaya_rp: [i64; 2],
    pub bahan: Vec<BahanResepDto>,
    pub langkah: Vec<&'static str>,
    pub alergen: Vec<&'static str>,
    #[serde(rename = "alatWajib")]
    pub alat_wajib: Vec<&'static str>,
}

impl From<&'static ResepDef> for ResepDto {
    fn from(r: &'static ResepDef) -> Self {
        Self {
            id: r.id,
            nama: r.nama,
            kelompok: r.kelompok,
            waktu_menit: r.menit,
            kisaran_biaya_rp: r.biaya,
            bahan: r
                .bahan
                .iter()
                .map(|b| BahanResepDto {
                    nama_bahan: b.nama,
                    gram_dasar_4_porsi: b.gram_dasar,
                    kode_grade: b.kode_grade,
                    opsional: b.opsional,
                })
                .collect(),
            langkah: r.langkah.clone(),
            alergen: r.alergen.clone(),
            alat_wajib: r.alat.clone(),
        }
    }
}

#[derive(Serialize)]
pub struct BahanTerskalaDto {
    #[serde(rename = "namaBahan")]
    pub nama_bahan: &'static str,
    pub gram: i64,
}

#[derive(Serialize)]
pub struct BelanjaDto {
    #[serde(rename = "namaBahan")]
    pub nama_bahan: &'static str,
    pub gram: i64,
    #[serde(rename = "perkiraanRp")]
    pub perkiraan_rp: i64,
}

impl From<&Belanja> for BelanjaDto {
    fn from(b: &Belanja) -> Self {
        Self {
            nama_bahan: b.nama,
            gram: b.gram,
            perkiraan_rp: b.rupiah,
        }
    }
}

#[derive(Serialize)]
pub struct RekomendasiDto {
    pub resep: ResepDto,
    #[serde(rename = "skorTotal")]
    pub skor_total: f64,
    pub alasan: String,
    #[serde(rename = "bahanTerskala")]
    pub bahan_terskala: Vec<BahanTerskalaDto>,
    #[serde(rename = "belanjaTambahan")]
    pub belanja_tambahan: Vec<BelanjaDto>,
}

impl From<&Hasil> for RekomendasiDto {
    fn from(h: &Hasil) -> Self {
        Self {
            resep: h.resep.into(),
            skor_total: h.skor,
            alasan: h.alasan.clone(),
            bahan_terskala: h
                .bahan
                .iter()
                .map(|(n, g)| BahanTerskalaDto {
                    nama_bahan: n,
                    gram: *g,
                })
                .collect(),
            belanja_tambahan: h.belanja.iter().map(Into::into).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct Data<T> {
    pub data: T,
}

pub fn json_kosong() -> Value {
    Value::Object(Default::default())
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn pola_id() {
        assert!(id_keluarga_valid("KEL-0001"));
        assert!(!id_keluarga_valid("KEL-001"));
        assert!(!id_keluarga_valid("KEL-00a1"));
        assert!(resep_id_valid("R-01") && resep_id_valid("R-12"));
        assert!(!resep_id_valid("R-00") && !resep_id_valid("R-13") && !resep_id_valid("R-1"));
    }
}
