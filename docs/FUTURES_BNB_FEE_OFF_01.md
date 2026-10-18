# Matikan pembayaran fee BNB Futures — 17 September 2026

Otorisasi: pengguna menjawab `YA boleh.. gwe mau live segera` atas pertanyaan
eksplisit untuk mengubah fee BNB khusus USD-M Futures dari ON ke OFF.

Lingkup: GET status awal, paling banyak satu POST `/fapi/v1/feeBurn` dengan
`feeBurn=false`, lalu konfirmasi melalui dua GET. Jika hasil POST tidak pasti,
hanya baca ulang; jangan mengulang POST otomatis. Kredensial tetap melalui
KWallet dan signer existing, tanpa nilai key/secret di output atau file.

Perubahan ini berlaku untuk semua simbol USD-M Futures, bukan opsi fee Spot,
sesuai kontrak Binance. Diskon BNB Futures tidak berlaku ketika OFF. Tidak ada
order, transfer, pengaturan leverage/margin atau aktivasi sesi lewat utilitas ini.

Batas kerja: satu utilitas Rust memakai dependency/toolchain/lockfile proyek,
satu percobaan perubahan akun, dua GET verifikasi dan satu probe prasyarat
aktivasi. Selesai jika fee OFF terkonfirmasi atau kegagalan spesifik dilaporkan.

Sumber protokol:
https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/account

## Hasil terverifikasi

- Status awal `feeBurn=true`. Satu POST diterima; tidak diulang.
- Dua GET sesudahnya: `feeBurn=false` pada keduanya.
- Diagnosis aktivasi budget 100 USDT: seluruh sepuluh prasyarat lolos pada dua
  snapshot, termasuk fee BNB off, saldo, mode akun dan tanpa posisi/order/algo.
- Probe strategi binary terpasang: dua snapshot lolos, usia maksimum 1814 ms.
- Unit test request membatasi endpoint Futures feeBurn, POST, nilai false,
  field signature/timestamp/recvWindow dan header sensitif; lulus. Clippy example
  `-D warnings` dan pemeriksaan format proyek lulus.
- Tidak ada order dikirim, sesi belum diaktifkan oleh utilitas. Folder sesi
  masih hanya berisi session.lock. Pengguna dapat mengulangi aktivasi melalui UI
  menggunakan batas sesi yang sudah diisi.
- Perubahan live terbatas preferensi fee Futures. Tidak ada perubahan binary
  desktop terpasang, dependency, toolchain, lockfile, atau batas risiko pengguna.
