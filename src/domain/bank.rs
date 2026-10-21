//! Bank 12 resep inti v1.0.0 (`R-01`..`R-12`), BRD 10.
//!
//! ASUMSI (ditandai di PR): BRD 10 hanya memberi daftar nama dan aturan; takaran gram dasar 4 porsi,
//! kelompok, alergen, alat, dan kisaran biaya di bawah ini disusun dari `bank_resep_grade2.json`
//! (pawonee-ai-models) dan dikonversi ke gram. Kisaran biaya dipatok ke batas kontrak
//! Rp18.000-Rp45.000 per porsi keluarga 4 jiwa. Perlu disahkan Tech Lead/Produk.

use std::sync::OnceLock;

#[derive(Debug, Clone)]
pub struct BahanDef {
    pub nama: &'static str,
    pub gram_dasar: i64,
    pub kode_grade: &'static str,
    pub opsional: bool,
    /// Bumbu memakai faktor skala yang lebih kecil dan dianggap tersedia di dapur (tidak masuk belanja).
    pub bumbu: bool,
}

#[derive(Debug, Clone)]
pub struct ResepDef {
    pub id: &'static str,
    pub slug: &'static str,
    pub nama: &'static str,
    pub kelompok: &'static str,
    pub menit: i64,
    pub biaya: [i64; 2],
    pub bahan: Vec<BahanDef>,
    pub langkah: Vec<&'static str>,
    pub alergen: Vec<&'static str>,
    pub alat: Vec<&'static str>,
    /// Tingkat pedas 0-3 (dipakai untuk filter keras `levelPedas`).
    pub pedas: i64,
}

impl ResepDef {
    pub fn hewani(&self) -> bool {
        self.bahan
            .iter()
            .any(|b| !b.opsional && (b.nama == "ayam" || b.nama == "telur"))
    }

    pub fn pakai_santan(&self) -> bool {
        self.bahan.iter().any(|b| !b.opsional && b.nama == "santan")
    }
}

fn b(nama: &'static str, gram: i64, grade: &'static str) -> BahanDef {
    BahanDef {
        nama,
        gram_dasar: gram,
        kode_grade: grade,
        opsional: false,
        bumbu: false,
    }
}
fn bumbu(nama: &'static str, gram: i64) -> BahanDef {
    BahanDef {
        nama,
        gram_dasar: gram,
        kode_grade: "NON-G2",
        opsional: false,
        bumbu: true,
    }
}
fn opsi(mut x: BahanDef) -> BahanDef {
    x.opsional = true;
    x
}

const V: &str = "G2-V";
const U: &str = "G2-U";
const S: &str = "G2-S";
const N: &str = "NON-G2";

pub fn bank() -> &'static [ResepDef] {
    static BANK: OnceLock<Vec<ResepDef>> = OnceLock::new();
    BANK.get_or_init(bangun)
}

pub fn cari(id: &str) -> Option<&'static ResepDef> {
    bank().iter().find(|r| r.id == id)
}

fn bangun() -> Vec<ResepDef> {
    vec![
        ResepDef {
            id: "R-01",
            slug: "tempe-tumis-bawang",
            nama: "Tempe Tumis Bawang",
            kelompok: "tumis",
            menit: 15,
            biaya: [18000, 24000],
            bahan: vec![
                b("tempe", 300, N),
                b("bawang merah", 60, U),
                bumbu("kecap manis", 30),
                bumbu("garam", 5),
            ],
            langkah: vec![
                "Iris tempe tipis dan bawang merah.",
                "Tumis bawang hingga harum, masukkan tempe.",
                "Tambah kecap dan garam, masak 8 menit hingga kecokelatan.",
            ],
            alergen: vec!["kedelai"],
            alat: vec!["kompor", "wajan"],
            pedas: 1,
        },
        ResepDef {
            id: "R-02",
            slug: "sayur-bening-bayam",
            nama: "Sayur Bening Bayam",
            kelompok: "kuah",
            menit: 15,
            biaya: [18000, 22000],
            bahan: vec![
                b("bayam", 300, S),
                b("jagung manis", 200, V),
                b("bawang merah", 30, U),
                bumbu("garam", 5),
                bumbu("gula", 5),
            ],
            langkah: vec![
                "Didihkan 600 ml air bersama irisan bawang.",
                "Masukkan jagung, masak 4 menit.",
                "Masukkan bayam, bumbui garam dan gula, masak 3 menit.",
            ],
            alergen: vec![],
            alat: vec!["kompor", "panci"],
            pedas: 0,
        },
        ResepDef {
            id: "R-03",
            slug: "telur-balado",
            nama: "Telur Balado",
            kelompok: "lauk",
            menit: 25,
            biaya: [20000, 28000],
            bahan: vec![
                b("telur", 240, N),
                b("cabai merah", 100, V),
                b("bawang merah", 50, U),
                b("tomat", 100, V),
            ],
            langkah: vec![
                "Rebus telur 10 menit, kupas dan goreng hingga berkulit.",
                "Haluskan cabai, bawang, dan tomat lalu tumis hingga tanak.",
                "Masukkan telur, bumbui, masak 5 menit.",
            ],
            alergen: vec!["telur"],
            alat: vec!["kompor", "wajan", "panci"],
            pedas: 3,
        },
        ResepDef {
            id: "R-04",
            slug: "nasi-goreng-kampung",
            nama: "Nasi Goreng Kampung",
            kelompok: "tumis",
            menit: 15,
            biaya: [18000, 26000],
            bahan: vec![
                b("nasi", 400, N),
                b("telur", 120, N),
                b("bawang putih", 20, U),
                bumbu("kecap manis", 45),
                opsi(b("kangkung", 50, S)),
            ],
            langkah: vec![
                "Orak-arik telur, sisihkan.",
                "Tumis bawang putih hingga harum.",
                "Masukkan nasi dan kecap, aduk 5 menit, campur telur.",
            ],
            alergen: vec!["telur", "kedelai"],
            alat: vec!["kompor", "wajan"],
            pedas: 2,
        },
        ResepDef {
            id: "R-05",
            slug: "sop-ayam-sederhana",
            nama: "Sop Ayam Sederhana",
            kelompok: "kuah",
            menit: 30,
            biaya: [28000, 40000],
            bahan: vec![
                b("ayam", 250, N),
                b("wortel", 200, V),
                b("kentang", 200, U),
                b("bawang putih", 20, U),
                bumbu("garam", 5),
                bumbu("merica", 5),
            ],
            langkah: vec![
                "Rebus ayam 10 menit, buang busanya.",
                "Masukkan bawang putih, wortel, dan kentang.",
                "Bumbui merica dan garam, masak 15 menit.",
            ],
            alergen: vec![],
            alat: vec!["kompor", "panci"],
            pedas: 0,
        },
        ResepDef {
            id: "R-06",
            slug: "tumis-kangkung",
            nama: "Tumis Kangkung",
            kelompok: "tumis",
            menit: 10,
            biaya: [18000, 22000],
            bahan: vec![
                b("kangkung", 400, S),
                b("bawang putih", 20, U),
                opsi(b("cabai rawit", 15, V)),
                bumbu("garam", 5),
            ],
            langkah: vec![
                "Siangi dan cuci kangkung.",
                "Tumis bawang dan cabai hingga harum.",
                "Masukkan kangkung, masak 4 menit dengan api besar.",
            ],
            alergen: vec![],
            alat: vec!["kompor", "wajan"],
            pedas: 3,
        },
        ResepDef {
            id: "R-07",
            slug: "semur-tahu",
            nama: "Semur Tahu",
            kelompok: "lauk",
            menit: 20,
            biaya: [18000, 26000],
            bahan: vec![
                b("tahu", 400, N),
                bumbu("kecap manis", 60),
                b("bawang merah", 50, U),
                b("bawang putih", 10, U),
            ],
            langkah: vec![
                "Goreng tahu setengah kering.",
                "Tumis bawang hingga harum.",
                "Masukkan tahu dan kecap, tambah air, masak 10 menit.",
            ],
            alergen: vec!["kedelai"],
            alat: vec!["kompor", "wajan"],
            pedas: 1,
        },
        ResepDef {
            id: "R-08",
            slug: "pecel-sayur",
            nama: "Pecel Sayur",
            kelompok: "kukus",
            menit: 15,
            biaya: [18000, 24000],
            bahan: vec![
                b("bayam", 250, S),
                b("taoge", 100, V),
                b("bumbu pecel", 80, N),
            ],
            langkah: vec![
                "Rebus bayam dan taoge, tiriskan.",
                "Encerkan bumbu pecel dengan air hangat.",
                "Tata sayur dan siram bumbu.",
            ],
            alergen: vec!["kacang-tanah"],
            alat: vec!["kompor", "panci"],
            pedas: 3,
        },
        ResepDef {
            id: "R-09",
            slug: "ayam-kecap-rumahan",
            nama: "Ayam Kecap Rumahan",
            kelompok: "lauk",
            menit: 30,
            biaya: [32000, 45000],
            bahan: vec![
                b("ayam", 500, N),
                bumbu("kecap manis", 75),
                b("bawang bombai", 100, U),
                b("bawang putih", 20, U),
            ],
            langkah: vec![
                "Tumis bawang bombai hingga layu.",
                "Masukkan ayam, aduk hingga berubah warna.",
                "Tambah kecap dan 200 ml air, masak 20 menit.",
            ],
            alergen: vec!["kedelai"],
            alat: vec!["kompor", "wajan"],
            pedas: 1,
        },
        ResepDef {
            id: "R-10",
            slug: "sop-jagung-telur",
            nama: "Sop Jagung Telur",
            kelompok: "kuah",
            menit: 20,
            biaya: [18000, 26000],
            bahan: vec![
                b("jagung manis", 300, V),
                b("telur", 120, N),
                b("wortel", 100, V),
                bumbu("garam", 5),
                bumbu("merica", 5),
            ],
            langkah: vec![
                "Sisir jagung, rebus bersama wortel 10 menit.",
                "Kocok telur, tuang perlahan sambil diaduk.",
                "Bumbui garam dan merica, masak 3 menit.",
            ],
            alergen: vec!["telur"],
            alat: vec!["kompor", "panci"],
            pedas: 0,
        },
        ResepDef {
            id: "R-11",
            slug: "sambal-goreng-tempe-kentang",
            nama: "Sambal Goreng Tempe Kentang",
            kelompok: "sambal",
            menit: 30,
            biaya: [22000, 32000],
            bahan: vec![
                b("tempe", 300, N),
                b("kentang", 300, U),
                b("cabai merah", 100, V),
                b("santan", 200, N),
                b("bawang merah", 40, U),
            ],
            langkah: vec![
                "Potong dadu tempe dan kentang, goreng hingga matang.",
                "Tumis bumbu halus hingga tanak, tuang santan.",
                "Masukkan tempe dan kentang, masak hingga meresap.",
            ],
            alergen: vec!["kedelai"],
            alat: vec!["kompor", "wajan"],
            pedas: 3,
        },
        ResepDef {
            id: "R-12",
            slug: "capcay-kuah",
            nama: "Capcay Kuah",
            kelompok: "kuah",
            menit: 25,
            biaya: [22000, 34000],
            // Resep sapu stok untuk G2-S kedaluwarsa <=2 hari (KONTEKS 5).
            bahan: vec![
                b("kol", 200, S),
                b("wortel", 150, V),
                b("sawi putih", 150, S),
                b("tomat", 100, V),
                b("bawang putih", 20, U),
                opsi(b("ayam", 100, N)),
            ],
            langkah: vec![
                "Iris semua sayur, tumis bawang putih hingga harum.",
                "Masukkan wortel, kol, dan sawi, tambah 400 ml air.",
                "Masak 10 menit, bumbui garam dan merica.",
            ],
            alergen: vec![],
            alat: vec!["kompor", "wajan"],
            pedas: 0,
        },
    ]
}

#[cfg(test)]
mod uji {
    use super::*;

    #[test]
    fn tepat_12_resep_id_berurutan_dan_batas_kontrak() {
        let b = bank();
        assert_eq!(b.len(), 12);
        for (i, r) in b.iter().enumerate() {
            assert_eq!(r.id, format!("R-{:02}", i + 1));
            assert!(r.menit >= 1 && r.menit <= 45, "{}", r.id);
            assert!(
                r.biaya[0] >= 18000 && r.biaya[1] <= 45000 && r.biaya[0] <= r.biaya[1],
                "{}",
                r.id
            );
            assert!(r.pedas <= 3);
            assert!(r
                .bahan
                .iter()
                .all(|x| ["G2-V", "G2-U", "G2-S", "NON-G2"].contains(&x.kode_grade)));
        }
        assert_eq!(b[11].slug, "capcay-kuah");
    }
}
