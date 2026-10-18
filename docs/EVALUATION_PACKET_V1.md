# Kontrak executable evaluasi v1 — profil arsip final

Implementasi: `desktop-rs/src/futures/evaluation/`; 0.6.8 dan ekstensi 0.6.9–0.6.14.
Dokumen ini mempersempit rancangan 07 menjadi profil yang benar-benar tersedia.
Tidak memuat transport/model runner, perubahan strategi, atau izin trading.

## CLI lokal

```text
binance-desk evaluation export --source /absolute/private/live --output /absolute/private/output/new-packet
binance-desk evaluation validate /absolute/private/output/new-packet
binance-desk evaluation validate-proposal /absolute/private/output/new-packet /absolute/private/proposal.json
```

Semua path absolut. Parent output harus sudah ada dan privat 0700; folder paket
harus baru. Tidak menimpa hasil lama. Source adalah folder state milik aplikasi,
bukan berkas key/KWallet; ekspor hanya sesi terakhir yang sudah final. Lock file
existing dibuka baca saja dengan shared try-lock, tanpa membuat/mengubah lock.
Writer live aktif, unfinished/current intent, arsip kurang atau total tidak
cocok menyebabkan penolakan. Tidak memulai recovery atau mengakses jaringan.

Subcommand tidak dapat digabungkan dengan flag vault, probe, paper atau sesi;
ditangani sebelum startup GUI/feed. Export berjalan terpisah, tidak dipanggil
di loop pengelolaan posisi. Lock lepas sebelum publikasi dan evaluasi model.

## Paket

Hanya lima file diizinkan; file tambahan, symlink, hardlink, izin terbuka dan
path traversal ditolak. File 0600, direktori 0700. Batas manifest 16 KiB,
masing-masing payload 2 MiB, maksimal 100 trade. Manifest dipublikasikan lewat
rename setelah payload + manifest sementara di-fsync; kegagalan sebelum itu
meninggalkan folder incomplete yang ditolak validator, bukan hasil sukses.

- `manifest.json`: schema 1, ID ekspor berdasar hash konten, `session-001`, versi
  exporter dan fingerprint source exporter + decoder + lockfile, provenance,
  UTC ms, config digest, daftar file/hash/ukuran/jumlah record, missing-data.
- `summary.json`: durasi diminta/aktual, waktu final state, early-stop, total
  gross/fee/funding aktual/net dan batas ekonomi terpilih. Tidak ada saldo.
  Mulai 0.6.9, `diagnostics` opsional berisi versi pembuat sesi, 11 counter
  bertipe, cabang terakhir dan flag saturation; bukan timeline penuh.
  Mulai 0.6.10, `recovery` opsional berisi jumlah episode (0–2) dan flag
  active yang wajib false pada paket final. Bukan timeline outage/health.
- `trades.jsonl`: `trade-001` dst, side/pattern, waktu intent/candle/last fill,
  qty/SL/TP/worst-entry rencana, qty fill aktual, hasil ledger, jumlah fill dan
  funding record, metadata exit opsional. Tidak ada ID exchange/client/fill.
- `decisions.jsonl` dan `health.jsonl`: kosong dalam profil arsip ini, wajib
  memiliki missing flags. Bukan bukti bahwa tidak ada keputusan/gangguan.

JSON paket adalah representasi kanonik serializer Rust: key order tetap,
tanpa pretty-print/newline pada JSON tunggal; tiap trade JSONL berakhir newline.
Validator reserialize dan mencocokkan byte sehingga duplicate key/encoding
alternatif/field tak dikenal ditolak. Jangan mengedit/pretty-print paket untuk
input; buat salinan tampilan di luar paket jika diperlukan.

Semua uang/harga/qty berbentuk string Decimal kanonik; tidak menerima JSON
number, exponent, NaN, trailing zero atau skala >12. Contoh `"-0.3415574"`
persis sama dengan −0.34155740, bukan pembulatan melalui float. Rust menghitung
`net = gross - fees + funding_actual`, mencocokkan setiap ledger dan total state.
Funding reserve adalah batas rencana, bukan biaya yang telah dibayar.

Fill/funding ID asli diperiksa keunikannya sebelum dibuang. Ledger diperiksa
session/strategy/config/prefix-nya. Kebenaran hasil adalah konsistensi arsip
lokal yang sebelumnya diselesaikan mesin; tidak melakukan query exchange ulang,
tidak memverifikasi ulang setiap raw fill yang memang tidak disimpan dalam
arsip, dan checksum bukan signature autentik dari exchange.

## Data yang sengaja tidak dikarang

`source_build_version=null` untuk sesi lama: versi binary pembuat sesi belum
ada di state, bukan versi exporter. Tidak menebak dari binary yang terpasang
sekarang. Sesi yang dibuat 0.6.9 memakai build enum dalam `diagnostics`, harus
identik pada manifest/summary. Absence field opsional menjaga byte kanonik
lama. Validator 0.6.14 menerima exporter 0.6.8/0.6.9/0.6.10/0.6.11/0.6.12/0.6.13/0.6.14; exporter 0.6.8
tidak boleh mengklaim diagnostics baru. Exporter sebelum 0.6.10 tidak boleh
mengklaim recovery atau build sumber 0.6.10–0.6.14. Exporter lama juga tidak
boleh mengklaim build sumber yang lebih baru darinya. Recovery hanya cocok
dengan diagnostics build0.6.10–0.6.14, tidak dikarang untuk sesi lama.
`planned_worst_entry` bukan harga fill rata-rata. `intent_at_ms` bukan waktu
fill entry. `finished_at_ms` berasal dari update final state, bukan last fill.

Missing flags dasar legacy: source build, full decision history, health time series,
intratrade price path, actual fill prices dan raw fill details. Exit reason/
outcome absent tetap null dengan flag. Tidak menghitung MFE/MAE/hit-target atau
menyamakan waktu holding kira-kira 30 menit dengan bukti alasan exit.

Counter 0.6.9 menghilangkan missing source build saja. Full decision/health
history tetap missing dan dua JSONL tetap kosong. Urutan counts adalah
no_pattern, late_signal, volatility, conflicting_signal, signal, funding,
economics, intent, recovery_skip, data_failure, safety_failure. Lima pertama
saling eksklusif per candle yang dievaluasi, bukan per polling. Counter
sinyal harus mencakup funding + economics + intent; intent sekurangnya jumlah
trade selesai. Cabang terakhir harus mempunyai counter positif. Sinyal dan
intent tidak membuktikan fill. Semua ini deskriptif, tanpa auto-apply.

Alasan bebas tidak diekspor. Satu template legacy yang seluruhnya cocok untuk
`order_batch_age_ms=N` dipetakan ke diagnosis `reconciliation_stale`, umur dan
batas 5000 ms, dengan evidence `legacy_exact_template`; flag stop terstruktur
tetap missing. Alasan lain menjadi missing, tanpa meneruskan teks mentah.
Metadata P1 mempertahankan waktu keputusan/primary/causes/outcome apa adanya
setelah validasi; pengurutan priority tidak ditafsir ulang sebagai fill.

## Proposal v1

Field wajib: `schema_version`, `proposal_id`, `export_digest` (SHA256 byte
manifest), `evaluator_version`, `model`, `prompt_sha256`, `question`, `facts`,
`inferences`, `missing_data`, `hypothesis`, `candidate_change`, `invariants`,
`test_protocol`, `pass_fail_criteria`, `usage`, `status`.

Fact memiliki `file` enum `summary`/`trades`, `record` null untuk summary atau
indeks trade mulai 1, `field` nama field scalar langsung, dan `value` persis
sama termasuk tipe string Decimal. Referensi ganda/fiktif/field tidak ada
ditolak. Maksimal 64 fakta, 8 inference, teks 2048 byte/field, file 64 KiB.
Pretty-print diperbolehkan untuk proposal, bukan paket.

Candidate hanya enum `study_target_and_holding_horizon_offline`,
`collect_missing_observations`, atau `no_change`. Tidak ada payload executable,
path runtime, parameter leverage/risk, atau patch. Status hanya `proposed`,
`insufficient_data`, `rejected`; tidak ada `accepted` dari agent. Missing flags
harus persis daftar manifest. Empat invariant wajib: `no_live_orders`,
`no_risk_increase`, `no_auto_apply`, `no_profit_or24h_claim`.

`usage` memuat calls/input_tokens/output_tokens dengan batas 6/40000/8000.
Ini pemeriksaan **deklarasi**, bukan meter biaya yang tepercaya. Runner P3 harus
mengukur seluruh subpanggilan dan mereservasi biaya sebelum request. Validasi
proposal tidak mengizinkan panggilan atau biaya, dan tidak membuktikan kebenaran
narasi/inference. Sukses CLI berarti **reviewable, human_review_required**, bukan
approved. Proposal tidak diimpor ke konfigurasi mesin dan tidak dieksekusi.

## Batas profil

Sesi lama selain sesi terakhir, sesi incomplete, parsing seluruh journal,
raw fill/price reconstruction dan schema telemetry umum belum didukung. Tidak
ada auto-export live, memory agent, evaluator runtime, container/proxy, scheduler
atau model run. P3 memerlukan pin vendor, isolasi proses/egress, meter budget,
provider/model yang dipilih dan izin data keluar sebelum pilot.
