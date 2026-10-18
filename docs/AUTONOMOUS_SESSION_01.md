# Sesi Futures otomatis 01 — 16 September 2026

## Lingkup sebelum implementasi

Permintaan pengguna: langsung membangun pemilihan arah otomatis dan menghubungkan
strategi campuran → SL/TP/sizing → pengelola order nyata, bukan perbaikan label.

- Satu implementasi Rust, anggaran maksimal 180 menit aktif; tanpa pencarian
  parameter, optimasi, strategi profit-claim atau sesi paper baru.
- BTCUSDT USD-M standar, Single-asset USDT, One-way, Isolated; satu posisi bot.
- Aturan tetap untuk breakout/trend-pullback/range-reversal, resolver konflik,
  candle selesai, keputusan Long/Short/menunggu, SL/TP berbasis volatilitas/tick.
- Supervisor menghubungkan signal, margin, flow, outbox, proteksi, cancel/query,
  penutupan, fill/fee/funding accounting, batas rugi serta persistensi/recovery.
- UI native memberi batas modal/risiko/leverage/durasi, satu aktivasi sesi live
  oleh pengguna, status alasan/posisi dan stop. Tidak ada arah/SL/TP manual sesi.
- Agen tidak mengaktifkan sesi uang nyata atau melakukan POST/DELETE Binance.
  Uji lifecycle otomatis memakai exchange lokal; data pasar/account aktual GET-only.
- Lindungi credential; data sesi privat disimpan user-local 0600, bukan log audit.
  Startup biasa tidak mengaktifkan transaksi atau resume dana nyata diam-diam.
- Penerimaan: uji long→flat→short, konflik/no-trade, biaya/risk, crash/duplicate,
  stop/protection failure/reconciliation, UI dan pipeline; install dan bukti native.
- Bukti `Workspace/outputs/binance-autonomous-session01-20260916`.

Selesai berarti alur software terhubung dan teruji sesuai kontrak, bukan bukti
profit atau acceptance order Binance production. Catat batas yang ditemukan
secara eksplisit; jangan menyamakan perubahan tampilan dengan mesin selesai.

## Hasil implementasi

**DITERIMA untuk integrasi software 0.6.0.** Rust menghubungkan ensemble tetap,
margin/sizing, supervisor, outbox/flow/transport, ledger serta session store.
Panel **Sesi otomatis** tersedia pada aplikasi native yang terpasang; parameter
arah/SL/TP tidak diminta dari pengguna. Satu aktivasi sesi melalui UI mengizinkan
program mengelola order selanjutnya. Tidak ada aktivasi live atau POST/DELETE
Binance oleh agen, tidak ada sesi paper baru, optimasi atau klaim profit.

### Bukti

- **84 tes desktop dan 36 engine lulus**, 1 tes jaringan publik terpisah ignored.
  Pipeline fmt/Clippy/doc/release/audit489 lulus. Setelah koreksi indikator sidebar
  menjadi status sesi live, 14 tes UI, fmt/Clippy serta build release diverifikasi
  lagi. Engine0.1 cocok checksum sumber baseline dan tidak diubah.
- Uji supervisor memakai implementasi yang sama dengan runtime, dengan exchange
  tiruan tanpa credential/jaringan: Long→flat→Short dalam satu sesi, SL/TP,
  child fill, fee, wallet, stop/deadline/limit, partial/reject, response hilang,
  restart dan residual. GET guard serta caller mencegah resend entry tidak pasti.
- Ledger diuji dengan2002 fill berhalaman, missing/duplicate/foreign fill,
  komisi non-USDT, funding tertunda, wallet mismatch dan arsip crash sebelum
  ringkasan state. Snapshot riwayat startup tidak mengklaim posisi saat ini.
- Uji UI memakai pointer pada widget nyata dengan worker fixture. Start hanya
  masuk setelah acknowledgment/budget, ganda ditolak, Stop diterima, dan mode
  offline/capture tidak bisa mengaktifkan transaksi.
- Probe Binance aktual dari binary terpasang **PASS dua snapshot GET-only**,
  termasuk feeBurn. Versi/probe/hash, screenshot native1400×900 dan1024×720
  tersimpan pada folder bukti. Nilai akun dan credential disamarkan.
- `binance-desk-auto-ui` membuka aplikasi0.6.0 halaman Futures. Jendela lama
  dipertahankan. Tidak menambahkan autostart atau layanan trading permanen.

### Keputusan dan batas

Rantai software yang diminta kini tersedia dan teruji secara deterministik.
Penerimaan POST/DELETE Binance, izin TRADE key dan performa dana nyata masih
belum diuji. Strategi tiga aturan belum memiliki bukti keuntungan. Batas awal:
BTCUSDT USDT standar, One-way/Single-asset/Isolated, satu posisi, fee USDT,
REST berurutan, akun didedikasikan, state satu API key. Tidak ada HFT/WebSocket
atau migrasi key. Gap/disconnect dapat melewati model rugi; crash sebelum
penerimaan SL dapat menyisakan posisi tanpa proteksi. Pemulihan tidak membuka
entry baru dan ketidakpastian permanen perlu pemeriksaan manual.

Kegagalan awal pengujian tetap ada pada log. Perbaikannya: assertion kalkulator
setelah collapse, fixture biaya short, presisi fixture EMA, kapitalisasi JSON
Side, digit grouping Clippy, dan status sidebar yang masih memakai paper.
Tidak melonggarkan biaya/validasi produksi agar fixture lolos.

[Panduan](../desktop-rs/README.md), [kontrak lengkap](../desktop-rs/src/futures/README.md),
[bukti dan hash](../../../outputs/binance-autonomous-session01-20260916/DECISION.md).
