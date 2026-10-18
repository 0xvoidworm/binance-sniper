# Diagnosis aktivasi sesi — 17 September 2026

Pengguna melaporkan tombol aktivasi tidak menjalankan sesi. Layar terakhir
menampilkan `Tidak ada sesi untuk dipulihkan`; pesan aktivasi awal sudah tidak
tersedia. Proses desktop masih berjalan. Folder live hanya berisi session.lock.

Lingkup: diagnosis baca saja terhadap syarat aktivasi, menggunakan konektor Rust
dan KWallet existing. Budget pembanding 100 USDT berasal dari form yang terlihat,
bukan persetujuan baru atas modal atau risiko. Tidak mengaktifkan sesi, mengubah
akun, mengirim order, atau mengubah aplikasi yang sedang berjalan.

Batas putaran: satu utilitas diagnostik, maksimum dua snapshot akun dalam satu
eksekusi dengan timeout 60 detik. Probe strategi existing telah lolos dua snapshot.
Tidak memasang dependency atau mengubah toolchain/lockfile. Utilitas hanya
mencetak hasil pemeriksaan boolean, tanpa kredensial, saldo atau ID order.

Selesai ketika blocker aktivasi ditemukan atau seluruh syarat yang diperiksa
lolos dan keterbatasan diagnosis dinyatakan.

## Hasil

- `binance-desk --probe-auto`: dua snapshot lolos; usia maksimum 1758 ms.
- `cargo run --locked --offline --example diagnose_activation -- 100`: dua
  snapshot konsisten. `fee_bnb_off=false`; sembilan pemeriksaan lain lolos:
  canTrade, One-way, Single-asset, auto-add margin off, tanpa posisi, tanpa
  order/algo BTCUSDT, budget mencukupi dan batas notional akun positif.
- `live::run` secara eksplisit menolak `feeBurn=true` sebelum `Engine::begin`,
  perubahan isolated/leverage atau entry. Ini menjelaskan mengapa tidak ada
  session.json yang dapat dipulihkan. Tidak membuktikan tidak pernah ada
  error tambahan pada percobaan sebelumnya, karena UI hanya menyimpan error terakhir.
- Keputusan: penyebab penghalang aktivasi saat diperiksa adalah pembayaran fee
  BNB yang belum didukung versi 0.6.0. Opsi akun tidak diubah. Pengguna dapat
  mematikannya untuk USD-M Futures; tindakan oleh agen memerlukan instruksi
  eksplisit karena merupakan perubahan akun. Diskon BNB tidak berlaku saat off.
- Utilitas memakai KWallet/konektor existing, akses `ReadOnly`, tanpa supervisor
  eksekusi atau session store, tidak mencetak saldo/ID order/kredensial.
- Binary desktop terpasang dan proses UI tidak diubah atau diaktifkan.
- Validasi utilitas: build/run offline dengan lockfile, Clippy example dengan
  `-D warnings`, serta `cargo fmt --all -- --check` lulus. Tidak ada dependency,
  toolchain atau lockfile yang berubah.
