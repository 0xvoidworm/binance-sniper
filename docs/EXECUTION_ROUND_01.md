# Putaran implementasi 01 — inti eksekusi Rust

Mulai 16 September 2026, atas permintaan pengguna melanjutkan pembangunan mesin Binance.

## Pertanyaan dan batas

Bisakah satu siklus order/posisi dijalankan secara deterministik, dengan sizing,
TP/SL, penghentian risiko, serta jurnal yang pulih setelah restart?

- Implementasi: komponen `engine-rs` dalam proyek existing; tanpa perubahan engine
  strategi, UI atau layanan Python yang sedang ada.
- Produk acuan teknis: USDⓈ-M linear one-way, sesuai proyek existing. Pilihan pasar
  pengguna masih diminta; ini tidak menetapkan Futures untuk akun live.
- Satu posisi aktif per mesin, order entry market, long/short, TP/SL close-all,
  laporan fill kumulatif dan laporan rekonsiliasi eksplisit. Belum portfolio banyak posisi.
- Input dan fill adalah skenario lokal sintetis, bukan transaksi Binance.
- Nol kandidat strategi baru, nol optimasi/backtest historis, nol order/API privat.
- Anggaran implementasi putaran: satu komponen dan satu fixture demonstrasi utama;
  maksimal 90 menit kerja aktif sebelum evaluasi batas. Jika belum lolos, laporkan
  hambatan dan pekerjaan yang tersisa; jangan memperluas scope diam-diam.
- Angka modal, biaya dan batas risiko fixture hanya untuk pengujian. Tidak menjadi
  konfigurasi akun pengguna atau janji profit.

## Keluaran dan penerimaan

- Library dan CLI Rust, konfigurasi eksplisit, jurnal append-only dengan checksum
  dan penguncian satu penulis; tidak ada pembacaan kredensial atau transport order.
- Sizing desimal serta pembulatan lot/filter; penjaga quote usang/spread; batas
  risiko per transaksi, notional dan kerugian total/harian pada simulasi.
- Uji long/short, partial fill, duplicate/conflicting event, timeout/reconcile,
  reject, protection failure, cancel, restart, corruption, dan risiko/quote.
- Demo entry → partial/full fill → TP/SL → exit → cleanup menghasilkan laporan.
- Format, Clippy, test, doc, release, audit Cargo lulus; bukti output disimpan
  di `~/Workspace/outputs` dan status/handoff diperbarui.

Selesai dengan LANJUT jika perilaku perangkat lunak memenuhi pemeriksaan;
TOLAK jika invariant inti tidak dapat dipenuhi; TERHAMBAT jika prasyarat nyata
menghalangi. LANJUT tidak berarti strategi profit atau adapter live siap.

## Referensi

- [Binance order dan algo/conditional](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade)
- [Filter instrumen](https://developers.binance.com/en/docs/products/derivatives-trading-usds-futures/common-definition)

Detail order live, rekonsiliasi akun aktual, funding, margin/likuidasi, hedge mode,
likuiditas/antrian order dan batas loss saat gap belum dibuktikan oleh simulator.

## Hasil — LANJUT, 16 September 2026

Selesai dalam batas satu komponen/fixture dan anggaran waktu putaran. `engine-rs`
berisi library dan CLI Rust 1.98.1; seluruh keluaran lingkup di atas tersedia.
36 tes lulus; format/Clippy/doc/release/audit Cargo dan doctor lingkungan lulus.
Demo release mencapai posisi kosong/cleanup; proses status terpisah memulihkan
state dan head hash identik. Uji juga mengulang setiap prefix demo setelah restart
dan menghasilkan jurnal byte-identik untuk dua run.

Pipeline gabungan pertama gagal 2 dari 35 tes: descriptor proses anak sempat
mempertahankan lock sesudah writer ditutup. Pelepasan lock eksplisit pada Drop
dan satu tes regresi menyelesaikannya. Log awal dipertahankan, bukan dibuang.
Koreksi compile awal: format hex SHA-256 mengikuti tipe digest versi terpin;
dua peringatan Clippy nested-if dirapikan tanpa menonaktifkan lint.

**LANJUT hanya berarti fondasi software memenuhi penerimaan putaran.** Tidak ada
strategi baru, backtest, edge/profit proof, koneksi/order Binance atau mutasi akun.
Belum ada modal/risiko live yang disepakati. Hasil negatif sebelumnya tidak berubah.
Satu demo sengaja memakai TP agar menguji cleanup; uji lain memeriksa stop/rugi,
gap melewati loss limit dan exit ditolak dengan eksposur tetap tercatat.

Keterbatasan implementasi yang ditemukan: caller wajib memberi Tick dan berhenti
pada input ditolak; rekonsiliasi laporan masih dipercaya dari simulator; koreksi
terminal, protection fill/cancel race nyata dan residual-exit retry belum tersedia.
Jurnal menolak tail rusak dan punya batas ukuran tanpa recovery/rollover otomatis.

- [Panduan dan kontrak lengkap](../engine-rs/README.md)
- [Paket keputusan/bukti](../../../outputs/binance-execution-round01-20260916/DECISION.md)
- [Log final](../../../outputs/binance-execution-round01-20260916/check-final.log)
- [Demo sintetis](../../../outputs/binance-execution-round01-20260916/demo/report.md)

Usulan pekerjaan berikut: adapter feed/account read-only dan startup reconciliation
dengan protokol tersendiri; keputusan pasar/alokasi tetap menunggu pengguna.
Putaran ini tidak otomatis membuka riset strategi atau aktivasi live.
