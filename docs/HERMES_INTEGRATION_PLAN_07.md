# Pembaruan lanjutan dan integrasi Hermes — rancangan 07

Pembaruan 20 September malam: pengguna menunda model/provider Hermes, tetapi
meminta perbaikan inti dilanjutkan. P3 bukan syarat rilis inti tanpa Hermes.
[Investigasi dan patch 0.6.9](ACCOUNT_SNAPSHOT_DIAGNOSTICS_09.md), serta
STATUS/HANDOFF, menggantikan status deployment lama di bawah. Tidak ada
panggilan model/izin data keluar baru. Kontrak rancangan tetap sebagai riwayat.

Tanggal: 20 September 2026 WIB. Status awal: **rancangan**.
Baseline aplikasi: 0.6.6; angka 07 adalah nomor rancangan, bukan versi rilis.

Perkembangan sesudah pengguna meminta lanjut sampai siap live: **P1 dan P2
terdapat dalam kandidat software0.6.8, belum deploy**.
[P1](RELIABILITY_P1_07.md), [P2 exporter/validator lokal](EVALUATION_BRIDGE_P2_08.md),
[profil paket executable](EVALUATION_PACKET_V1.md). P3–P5 tetap rancangan; belum
ada pemasangan Hermes/model run. Provider/model, biaya dan izin data keluar
menunggu pilihan pengguna. P2/pilot arsip tidak memerlukan sesi live lebih dulu.
Kontrak dan hasil putaran desain di bawah dipertahankan sebagai riwayat.

## 1. Kontrak putaran desain

Permintaan pengguna: rancang pembaruan lanjutan dengan memasukkan Hermes.
Hermes di sini berarti **Nous Research Hermes Agent**, bukan nama test harness.
Test harness adalah perangkat uji Rust terpisah yang tetap diperlukan.

Lingkup putaran desain: satu rancangan arsitektur, kontrak data, prioritas patch,
pengujian, batas akses, dependency dan keputusan lanjut. Anggaran kerja desain
satu putaran maksimal 45 menit; nol percobaan model, nol instalasi, nol perubahan
kode produksi, konfigurasi akun atau sesi. Selesai ketika titik integrasi cocok
dengan kode/bukti, dokumen terkait konsisten, tautan dan diff diperiksa.
Ini bukan izin memasang Hermes, menggunakan layanan model berbayar, mengirim
data ke provider, deploy binary atau mengaktifkan transaksi.

Tujuan pembaruan: mengurangi kegagalan runtime yang bisa direproduksi dan
membuat evaluasi strategi dapat ditelusuri. Keuntungan dan kesiapan operasi
24 jam tetap harus dibuktikan terpisah; Hermes bukan prasyarat mesin berjalan.

## 2. Bukti awal dan batas kesimpulan

Acuan: [audit sesi 300 menit](../../../outputs/binance-session-review-300m-20260920-ip2XD9/LAPORAN.md),
sesi `b1a0bcf9e6d31249d`, strategi `ensemble-2-cost-aware`.

| Fakta audit | Implikasi rancangan |
|---|---|
| Sesi berjalan 194 menit 5,332 detik dari 300 menit | Keandalan saat posisi aktif menjadi prioritas pertama |
| Stop: `order_batch_age_ms=5069`, batas 5000 ms | Reproduksi penumpukan latency; jangan sekadar menaikkan batas |
| Tiga trade; gross +0.14079999, fee 0.48235739, funding 0, net −0.34155740 USDT | Pisahkan keberhasilan eksekusi dari ekonomi strategi |
| Settlement sempat menampilkan Watching walau Stop tersimpan | Perbaiki transisi/pesan tanpa membuka kembali izin entry |
| Trade 1–2 sekitar 30 menit; alasan exit terstruktur belum tersedia | Catat alasan permintaan exit dan bukti hasil exit secara terpisah |
| 38 candle terkonfirmasi; 21 keputusan pencarian entry, 3 entry | Jangan menganggap semua candle tanpa entry sebagai kegagalan input |

Tidak menyimpulkan keunggulan arah dari tiga gross positif. Penyebab eksklusif
exit trade 1–2 belum terbukti. Enam respons market sempat uncertain lalu pulih
melalui GET; field respons asli penyebab parser gagal tidak tersedia. Jangan
menulis diagnosis seolah raw response diketahui. Dua GET flat setelah audit
adalah bukti bertimestamp saat itu, bukan verifikasi akun pada waktu mendatang.

## 3. Arsitektur dan pemilik keputusan

```text
Binance ↔ Rust: input → strategi/planner → supervisor → order/SL/TP/ledger
                    │                           │
                    └──── peristiwa aman ────────┘
                                ↓
                 exporter Rust + validasi paket
                                ↓ hanya baca
                 Hermes terisolasi, setelah sesi
                                ↓
                 laporan + proposal eksperimen
                                ↓
                 validator + test harness Rust
                                ↓
                 review → rilis versi berikutnya
```

- Rust tetap satu-satunya pemilik eksekusi, risiko, Stop, deadline, outbox dan
  ledger. Native egui tetap UI utama; tidak membuat dashboard web pengganti.
- Hermes tidak dipanggil dari jalur `session_frame`, `tick_prepared`, pengiriman
  SL/TP, exit, maupun settlement. Tidak ada keputusan posisi menunggu LLM.
- Tahap pertama adalah pekerjaan sekali jalan **sesudah sesi selesai dan
  settlement final**. Sesi terputus boleh diekspor dengan label incomplete,
  tetapi tidak boleh dihitung sebagai hasil final atau memicu pemulihan akun.
- Pengamat live asinkron baru opsi fase lanjutan setelah non-interference
  terbukti. Tidak termasuk runtime pengamat 24 jam pada putaran pertama.
- Hermes menghasilkan hipotesis, bukan perintah shell, patch otomatis, order,
  atau konfigurasi yang dimuat aplikasi. Penilaian manusia tetap diperlukan.

## 4. Peta perubahan kode yang direncanakan

Semua nama modul baru di bawah adalah usulan, belum ada implementasinya.

| Area | Lokasi existing / calon baru | Perubahan dan ketergantungan |
|---|---|---|
| Akuisisi snapshot | `desktop-rs/src/futures/live.rs`, `session_input.rs`, `readiness.rs` | Pisahkan kerja history/reference lambat dari jendela bukti eksekusi segar |
| Guard dan lifecycle | `supervisor.rs`, `outbox.rs`, `flow.rs` | Pertahankan ownership, freshness, no-duplicate dan prioritas Stop/proteksi |
| Alasan exit | `supervisor.rs`, `session_store.rs`, `ledger.rs` | Alasan terstruktur, kompatibilitas state/arsip lama, hasil fill terverifikasi |
| Jejak terpilih | calon `futures/evaluation_events.rs` | Event bertipe, terbatas ukuran, tanpa raw private response |
| Ekspor | calon `futures/evaluation_export.rs`, argumen CLI di `main.rs` | Paket read-only, manifest, Decimal, checksum dan quality flags |
| Evaluator | calon utilitas Rust terpisah | Validasi paket/proposal, jalankan fixture/replay terbatas tanpa transport live |
| Hermes | runtime vendor terpisah dari kedua crate | Baca paket; tulis laporan/memori/usulan pada direktori sendiri |
| UI opsional | `desktop-rs/src/ui/live_ui.rs` | Tampilkan status evaluasi/laporan; tidak mengaktifkan kandidat strategi |

Evaluator memakai kembali perhitungan `planning`/`margin` melalui antarmuka
Rust yang diuji bila perlu diekstrak. Jangan menyalin formula ke prompt atau
menganggap simulator `engine-rs` sudah identik dengan ensemble Futures live.
Replay lengkap masih perlu dibangun dan diuji; jejak planner existing hanya
memadai untuk sebagian pemeriksaan ekonomi/sizing.

## 5. P1 — keandalan dan observabilitas sebelum Hermes

### P1a. Snapshot saat posisi aktif

Alur saat ini: persiapkan reference → rekonsiliasi order → fetch akun/quote →
history bila diperlukan → validasi frame dan bukti. `session_input::poll`
mengambil akun sebelum history; kerja sesudah rekonsiliasi menua-kan bukti.
Sebab terperinci tiap latency produksi belum seluruhnya terukur.

Rancangan pilihan: siapkan reference dan kandidat history di luar jendela
pengambilan bukti eksekusi; ambil/validasi pasangan order–akun/quote dengan
budget bersama dan waktu asli. Manajemen posisi/Stop tidak menunggu finalisasi
candle. Evaluasi sinyal, termasuk exit karena arah berlawanan, tetap hanya
memakai candle terkonfirmasi dengan aturan versi yang sama; pemisahan jalur
tidak boleh diam-diam menghilangkan aturan exit tersebut.

Tambahkan correlation ID per siklus dan rentang waktu persiapan/reference,
order, akun, quote, history serta dispatch. Gunakan jam monotonik untuk durasi
dan waktu exchange untuk umur quote. REST tetap tidak atomik; snapshot silang
yang tidak konsisten harus ditolak, bukan dibuat seolah atomik oleh timestamp.

Maksimal satu reacquire pasangan bukti sebagai calon solusi, hanya sebelum
keputusan/mutasi, dalam deadline dan cooldown existing. Tidak mengulang POST,
men-stamp ulang bukti lama, menghapus Stop, atau menunggu tanpa batas. Jika
tetap stale, jalur pause saat flat / stop-close-only saat intent aktif tetap.
Jumlah request tambahan dan rate-limit harus diukur sebelum menerima desain.

### P1b. State dan alasan keluar

- Setelah settlement ketika Stop aktif, tampilkan finalisasi/Finished sesuai
  syarat aktual; jangan menampilkan Watching atau membuka entry baru.
- Catat `exit_requested_at`, `exit_primary_reason`, `exit_causes` (semua sebab
  yang benar pada evaluasi itu), serta versi aturan. Enum mencakup user-stop,
  deadline, loss-limit, stale-data, protection-failure, hold-timeout,
  funding-window, opposite-signal dan liquidation-buffer.
- Bedakan alasan permintaan dengan `exit_observed_outcome`: fill SL/TP,
  reduce-only, campuran atau unknown; simpulkan outcome hanya dari order/fill.
  Simpan pemicu Stop sesi pertama terpisah dari penyebab exit trade.
- Tetapkan prioritas sebab saat beberapa guard aktif dan uji deterministik.
  Catatan lama menggunakan unknown/missing, bukan rekonstruksi asumsi timeout.
- Perubahan serialisasi wajib kompatibel dengan checksum/envelope existing.
  Field opsional absent tidak mengubah byte/hash lama; fixture restart,
  settlement dua kali dan crash di sela commit wajib lulus sebelum migrasi.

## 6. P2 — kontrak paket evaluasi v1

Exporter berjalan lokal di Rust, tidak membuka KWallet/transport jaringan.
Ambil snapshot konsisten dari arsip final di bawah mekanisme lock yang benar,
lepaskan lock sebelum pekerjaan evaluasi. Jangan membaca/mengekspor state
aktif separuh tertulis. Hasil ke subfolder baru berizin 0700 dalam
`~/Workspace/outputs`; file 0600. Arsip sumber tidak berubah.

| File usulan | Isi yang diizinkan |
|---|---|
| `manifest.json` | schema_version=1, export_id, ID sesi pseudonim, build/strategy/config digest, rentang UTC, waktu ekspor, hash/ukuran file, provenance, complete/incomplete |
| `summary.json` | durasi diminta/aktual, trade count, gross/fee/funding/net, stop code, konfigurasi ekonomi terpilih |
| `trades.jsonl` | ID trade lokal, sinyal/side, qty, harga, SL/TP rencana, hasil Decimal, alasan exit + kualitas bukti |
| `decisions.jsonl` | ID event/candle, confirmed/pending/skip, pola/penolakan, review planner terpilih, strategy version |
| `health.jsonl` | ID siklus, endpoint enum, latency/umur, kode error, recovery; bukan URL atau body |
| `observations.jsonl` | opsional: bid/ask/mark/OHLCV publik bertimestamp, cakupan sampling, gap dan provenance |

Aturan kontrak:

1. Uang, qty, harga, fee dan rasio berbentuk string Decimal; tidak melalui
   float. Angka ringkasan dihitung Rust dari ledger dan dibandingkan total sesi.
2. Timestamp UTC ms eksplisit; WIB hanya tampilan. Missing tidak menjadi nol.
   Funding aktual dibedakan dari reserve. Waktu intent bukan waktu fill.
3. Ekspor memakai allowlist field, bukan dump lalu sensor regex. Dilarang:
   key/secret/cookie/signature, wallet/balance mentah, binding akun, alamat
   pribadi, response privat mentah, file `.env`, URL bertanda tangan dan
   client/exchange order ID asli. Relasi order/fill memakai ID lokal konsisten.
4. Teks bebas dibatasi dan disanitasi; reason utama enum/template. Validator
   menolak schema tidak dikenal, path traversal/symlink keluar paket, file
   hilang, hash salah, ID ganda dan relasi lintas sesi yang tidak sah.
5. Manifest dipublikasikan atomik setelah isi lengkap; runner hanya menerima
   paket valid. Hash memeriksa integritas, bukan membuktikan laba atau asal
   exchange. Tidak mengklaim signature autentik tanpa mekanisme signing.
6. Batasi jumlah/ukuran record. Kelebihan/gap diberi flag, tidak dipotong diam-
   diam. Untuk telemetry tambahan, gunakan antrean bounded non-blocking;
   export gagal/antrean penuh tidak menghentikan pengelolaan posisi. Ledger
   dan outbox wajib tetap durabel; aturan best-effort hanya untuk telemetry.
7. Jejak lama dapat tidak lengkap. Jangan membuat MFE/MAE atau waktu sentuh
   target dari harga entry/exit saja. Sampel terbatas hanya menghasilkan
   estimasi berlabel; OHLCV tidak membuktikan urutan SL dan TP dalam satu bar.

Paket audit existing menjadi sumber bootstrap lokal, **belum otomatis layak
dikirim ke provider**. Allowlist/validator baru harus memeriksa dan membentuk
ulang paket, tanpa memasukkan laporan jawaban acuan ke input uji Hermes.

## 7. P3 — runtime Hermes dan batas kepercayaan

Pilihan desain: seluruh proses Hermes dalam container non-root terisolasi;
utamakan runtime rootless yang kompatibel setelah inventaris Fedora diperiksa.
Container terminal saja tidak mengisolasi seluruh agent; dokumentasi upstream
membedakan [Hermes di dalam Docker](https://hermes-agent.nousresearch.com/docs/user-guide/docker)
dari backend terminal Docker. Dukungan runtime lokal belum diuji dalam putaran ini.

- Mount hanya satu paket tervalidasi read-only; direktori report/memori/scratch
  terpisah writable. Tidak mount home host, repo trading, live state/outbox,
  KWallet, D-Bus/session bus, SSH agent, socket container atau device desktop.
- Root filesystem/runtime read-only, capability dibuang, no-new-privileges,
  SELinux tetap aktif. Tidak memasukkan agen ke grup/otoritas kontrol host.
- Tidak ada koneksi Binance, web browser, messaging, MCP host, terminal host,
  delegasi subagent atau pemasangan skill/tool otomatis. Kredensial model
  terpisah dari trading; hanya akses ke provider yang disetujui melalui
  pembatasan egress/proxy. Uji koneksi terlarang; flag konfigurasi saja tidak
  cukup membuktikan isolasi. Lihat [batas keamanan upstream](https://hermes-agent.nousresearch.com/docs/user-guide/security).
  Secret model disuplai dari penyimpanan kredensial terpisah, tidak disimpan
  pada paket/proyek/laporan atau dicetak lewat dump environment.
- Prompt/data/log diperlakukan sebagai input tidak tepercaya. Instruksi dalam
  record tidak dapat memperluas tools/akses. Output juga tidak tepercaya:
  tidak dieksekusi dan tidak diimpor langsung sebagai setting atau source.
- Proposal batas awal (belum persetujuan biaya): satu worker, maksimum dua run
  pada satu sesi acuan, 10 menit/run, enam panggilan model/run, total input
  maksimal 40.000 token dan output 8.000 token/run, tanpa retry job otomatis.
  Budget mencakup subpanggilan internal; fitur latar belakang di luar meter
  dinonaktifkan. Runner luar menegakkan timeout/cap, bukan berharap prompt.
  Sebelum panggilan, reservasi budget untuk input dan output maksimum menurut
  tarif provider yang diverifikasi saat pemilihan; jangan hanya memeriksa
  biaya setelah panggilan selesai. Tidak ada tarif yang diasumsikan di sini.
- CPU/RAM/disk dibatasi setelah kapasitas host diperiksa. Uji pertama hanya
  saat mesin trading idle. Provider, model, batas rupiah/USD dan izin data
  keluar belum ditetapkan; run model tetap **disabled** sampai itu jelas.
  Jika accounting biaya gagal/tidak tersedia, tidak memulai panggilan baru.
- Gateway/cron tidak diperlukan untuk percobaan pertama. Penjadwalan setelah
  sesi baru dipertimbangkan sesudah audit satu-shot lolos; keyed oleh export
  digest + evaluator/model/prompt version agar event ulang tidak menggandakan
  job. Timeout menghasilkan status gagal/unknown, bukan pengulangan tak terbatas.

### Memori dan keluaran

Hermes memiliki [memori persisten yang dapat diperbarui](https://hermes-agent.nousresearch.com/docs/user-guide/features/memory).
Itu cache kerja, bukan sumber kebenaran. Arsip eksperimen append-only dikelola
runner Rust di luar hak tulis Hermes. Setiap hasil punya bukti, versi dan status
accepted/rejected/insufficient-data; hasil negatif tidak boleh dihapus. Satu
profil/home Hermes khusus proyek, satu penulis; tidak mengimpor memori pribadi.
Catat digest prompt, skill dan snapshot memori pada tiap run. Uji pengulangan
pilot memakai snapshot awal yang sama; perubahan memori/skill menjadi versi
eksperimen baru, bukan perubahan tersembunyi di tengah pembandingan.

Kontrak `proposal.json` v1: `proposal_id`, `export_digest`, evaluator/model/
prompt version, `question`, `facts[]` (nilai + file/record/field bukti),
`inferences[]`, `missing_data[]`, `hypothesis`, `candidate_change`,
`invariants[]`, `test_protocol`, `pass_fail_criteria`, `usage`, `status`.
Maksimal satu hipotesis utama. Kandidat tidak boleh memuat executable, path
arbitrer atau field pengubah risiko. Validator menolak output tidak sesuai;
hasil yang benar secara schema tetap memerlukan review kebenaran fakta.

## 8. Dependency dan pemasangan yang direncanakan

- Tidak ada dependency/toolchain diubah pada putaran desain. P1/P2 utamakan
  crate existing: serde, serde_json, rust_decimal, sha2, tokio, tempfile.
- Semua kode penghubung, validator, runner dan test baru memakai Rust dengan
  pin/lockfile existing. Utilitas baru dibuat melalui `rust-workbench new`
  setelah membaca README/inventaris workbench; validasi `rust-workbench check`.
- Hermes upstream adalah software Python: [manifest resmi yang diperiksa](https://raw.githubusercontent.com/NousResearch/hermes-agent/main/pyproject.toml)
  mensyaratkan Python >=3.11,<3.14. Python 3.12 terkelola uv dipilih sebagai
  pengecualian teknis untuk menjalankan vendor, bukan menulis ulang engine.
  Gunakan environment terpisah dari proyek Binance dan Python sistem.
- Sebelum install: periksa tool/inventaris dan doctor baseline; pilih commit/
  rilis upstream immutable, catat digest source/image, lockfile dan dependency
  resolved. Pin versi provider adapter dan audit lisensi/kerentanan. Link main
  di atas adalah referensi saat desain, bukan pin pemasangan.
- Hindari installer pipe-to-shell, image latest, auto-update, dependency lazy
  install saat run dan tambahan all-extras tanpa kebutuhan. Jika pin tidak
  kompatibel Python/runtime terpilih, laporkan dahulu; tidak mengganti pin
  Rust atau memakai Python sistem diam-diam.
- Tidak menambah vector database, broker antrean atau API server untuk satu
  evaluator lokal. Artefak file terversi mencukupi tahap pertama.

## 9. Urutan kerja dan keputusan tiap putaran

Plafon berikut adalah usulan teknis, bukan persetujuan biaya atau janji selesai.
Jika habis, simpan bukti dan keputusan; jangan otomatis memperluas scope.

| Tahap | Batas percobaan | Syarat selesai / keputusan |
|---|---|---|
| P1 reliability | Satu rancangan snapshot; maksimal 2 jam kerja aktif sebelum laporan checkpoint | Regresi lama gagal sebelum patch, lolos sesudahnya; seluruh guard/lifecycle tetap |
| P2 exporter | Satu schema v1, satu sesi acuan + fixture negatif; 2 jam/checkpoint | Paket aman, hitungan eksak, kompatibilitas arsip dan source state tidak berubah |
| P3 Hermes pilot | Satu pin/model, satu sesi acuan, maksimal dua run sesuai cap bagian 7 | Fakta benar, batas akses lolos, satu usulan terukur dan biaya tercatat |
| P4 eksperimen strategi | Satu kandidat terhadap baseline; protokol/data/budget dibekukan sebelum run | Accepted/rejected/insufficient-data; tanpa menyetel ulang sampai menang |
| P5 UI/scheduler | Opsional setelah P3 berguna | Laporan terhubung native UI, job dedup, kegagalan tidak memengaruhi live |

P1/P2 dapat dilanjutkan tanpa model/provider Hermes. P3 memerlukan pilihan
provider/model, izin instalasi serta data keluar/biaya. Itu gerbang pelaksanaan,
bukan penghalang menyelesaikan rancangan ini. Tidak memulai sesi paper/live baru
sebagai jalan pintas; fixture dan replay offline tetap boleh untuk pengujian.

Pilot dinilai terhadap laporan audit yang ditahan sebagai jawaban acuan:
angka ketiga ledger dan total tepat; early-stop/funding/missing exit reason
diklasifikasikan benar; fakta memiliki referensi valid; tidak mencampur market
setelah sesi dengan kondisi entry; tidak mengklaim profit/24 jam. Satu kesalahan
material atau pelanggaran akses menggagalkan pilot. Catat biaya, waktu review
manusia, kesalahan dan manfaat tambahan dibanding laporan Rust tanpa Hermes.
Kelulusan satu kasus hanya menerima integrasi pilot, bukan akurasi umum.

Hipotesis awal P4: apakah target cost-aware 3–6 ATR selaras dengan horizon
keluar 30 menit dan biaya aktual? Belum memilih TP lebih dekat atau holding
lebih lama. Sebelum pengujian tentukan cakupan data, jumlah sampel memadai,
periode pengembangan/holdout kronologis, satu perubahan kandidat, model fill/
slippage/funding, metrik net/drawdown/exposure dan syarat terima. Data yang sudah
dipakai memilih kandidat tidak menjadi holdout. Data kurang → insufficient-data,
bukan menurunkan standar. Tiga trade saat ini hanya diagnosis eksploratif.

## 10. Matriks pengujian wajib

| Kelompok | Kasus minimum dan hasil yang diharapkan |
|---|---|
| Snapshot | DNS/clock lambat + pergantian candle; bukti 4999/5000/5001/5069 ms; akun/order/quote basi bergantian → guard sesuai, tanpa restamping |
| Koherensi | Intent berubah, bukti dipakai ulang, Stop/deadline saat GET, order terisi di sela snapshot → tolak bukti salah, tidak ada reentry/POST ganda |
| Proteksi | Fill parsial, SL ditolak/hilang, uncertain response, cancel/fill race, residual/restart → jalur proteksi/close-only tetap benar |
| Settlement | Stop + ledger stabil, crash arsip sebelum ringkasan, replay commit → tidak Watching saat Stop, PnL tidak ganda |
| State lama | State/envelope/arsip 0.6.6 tanpa field baru → terbaca dan hash valid, alasan missing tidak dikarang |
| Ekspor | Decimal, fee/funding, sesi incomplete, duplicate fill, gap, disk penuh, crash tulis → hasil benar atau penolakan eksplisit, live tidak terganggu |
| Data aman | Canary kredensial sintetis, field asing/free-text, symlink/traversal, hash rusak, paket terlalu besar → tidak bocor/tidak diproses diam-diam |
| Isolasi | Coba baca/tulis dummy live-state, akses D-Bus/socket/host, koneksi Binance dan tujuan non-allowlist → gagal di batas OS/jaringan |
| Output agent | Angka fiktif, bukti tidak ada, instruksi di log, usulan leverage/risk, executable → ditolak; tidak ada jalur eksekusi output |
| Non-interference | Kill/hang evaluator, overload terbatasi, antrean penuh, provider timeout/budget habis → lifecycle fixture identik dengan Hermes mati |
| Reproduksibilitas | Paket sama + versi sama; run berulang, event/job duplikat → metrik Rust identik, variasi model tercatat, job tidak ganda |

Sesudah implementasi: fmt, Clippy, test desktop/engine, doc, build release dan
audit dependency sesuai pipeline proyek. Tes jaringan tidak dianggap lulus
karena ignored. Probe read-only maksimal 10 menit dan dua setup GET hanya pada
putaran verifikasi yang mencantumkannya; bukan bukti jalur posisi aktif/24 jam.
Uji sintetis 24 jam memakai waktu virtual untuk deadline, funding, pergantian
hari UTC dan restart; tetap bukan pengganti pengamatan produksi pengguna.

## 11. Aktivasi, rollback dan handoff

- Aktivasi evaluator terpisah dari izin sesi trading. Default off; hentikan
  evaluator tanpa menyentuh worker live. Laporan/proposal tidak auto-apply.
- Deployment binary hanya ketika idle/flat dan arsip final terverifikasi,
  dengan lock serta backup dan persetujuan yang sesuai. Jangan mengganti proses
  saat posisi aktif. Versi strategi/config tetap sepanjang satu sesi.
- Rollback evaluator: nonaktifkan job, pertahankan bukti. Rollback binary:
  cocokkan schema terbaru terlebih dahulu; jangan memulihkan state/outbox lama
  di atas aktivitas exchange yang lebih baru atau menghapus transaksi gagal.
- Hermes tidak memberikan izin transaksi. Pengguna tetap mengaktifkan sesi
  live lewat UI dan batas eksplisit; tidak ada janji entry atau keuntungan.

## 12. Hasil putaran desain

Rancangan mengikat patch keandalan, ekspor, evaluator dan review tanpa akses
order Hermes. Kode target dan batas bukti dicocokkan dengan baseline 0.6.6 dan
audit sesi 300 menit. Verifikasi dokumentasi: 98 tautan lokal diperiksa tanpa
target hilang; pemeriksaan whitespace lolos. Hash 56 file source Rust,
konfigurasi, toolchain dan lockfile sebelum/sesudah identik. Instruksi proyek,
scope, STATUS, KNOWN_GAPS dan HANDOFF diselaraskan tanpa menghapus riwayat.

Keputusan: putaran rancangan selesai; titik awal implementasi berikutnya P1,
bukan instalasi Hermes dahulu. Tes/build tidak dijalankan ulang karena tidak
ada perubahan kode. Belum ada patch, schema executable, runtime Hermes, model
terpilih, pengujian implementasi, instalasi atau perubahan akun/sesi di sini.
