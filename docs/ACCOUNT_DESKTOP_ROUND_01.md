# Putaran akun Futures desktop 01 — 16 September 2026

Mulai sekitar 16:48 WIB. Pengguna menginstruksikan melanjutkan ke pembacaan akun
Futures setelah memilih Futures untuk kebutuhan leverage.

## Lingkup sebelum implementasi

- Rust, aplikasi native existing. Satu adapter USD-M Futures read-only.
- Saldo wallet/tersedia, margin, unrealized P&L, posisi terbuka, konfigurasi
  leverage/margin BTC/ETH dan simbol posisi aktif, mode posisi dan multi-assets.
- GET bertanda tangan HMAC ke endpoint Binance resmi yang dibatasi enum;
  sinkronisasi waktu server, TLS tervalidasi, tanpa redirect/proxy lingkungan,
  error tersanitasi dan rate backoff. Tanpa order/mutasi akun/user-data-stream POST.
- Input key/secret tersamarkan di aplikasi, opsi sesi saja atau Secret Service
  Linux/KWallet. Tidak menulis rahasia ke config/proyek/log/audit/chat/argv.
- UI akun belum terhubung/error/stale harus berbeda dari saldo/posisi nol.
  Data akun hanya di memori; screenshot audit default menyamarkan nilai akun.
- Penyimpanan lama memakai Windows DPAPI; metadata lokal diperiksa tanpa
  membaca rahasia ke output. Tidak mengubah backup. Jika kredensial belum
  tersedia, selesaikan aplikasi dan uji kontraknya lalu laporkan hambatan live.
- Anggaran satu integrasi, maksimal 100 menit aktif sebelum evaluasi. Nol
  trial strategi, nol order atau perubahan leverage/margin, tanpa autostart.

## Penerimaan dan titik berhenti

1. Signing terhadap vektor publik; endpoint/method allowlist, redaksi error,
   parser desimal/mode posisi/multi-assets, stale dan disconnect/reconnect diuji.
2. Input/keyring diuji tanpa kredensial nyata di output, UI kontrol/layout diuji.
3. fmt/Clippy/test/doc/release/audit dependency terkunci lulus.
4. Aplikasi dipasang dan dibuka, halaman akun native diperiksa. Pembacaan akun
   nyata hanya bila kredensial tersedia; catat bukti ringkas tanpa angka akun
   atau secret di audit. Jangan menyebut integrasi live lulus hanya dari mock.
5. Status dan batas diperbarui; bukti di
   `Workspace/outputs/binance-account-desktop-round01-20260916`.

LANJUT berarti kemampuan read-only berhasil diverifikasi. Jika autentikasi
belum dapat diuji, keputusan TERHAMBAT untuk penerimaan live, dengan komponen
software yang sudah selesai dijelaskan terpisah.

## Referensi resmi

- https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/account
- https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade
- https://docs.rs/keyring/3.6.3/keyring/

## Hasil putaran

- Software 0.3.0 terpasang dan halaman akun dibuka. 30 tes offline, 1 transport
  publik nyata, fmt/Clippy/doc/release/audit 489 dependency lulus. KWallet dummy
  tulis/baca/hapus berhasil. Native form normal/minimum dan workspace diperiksa.
- Entry aplikasi belum ada di KWallet. Probe binary terpasang berhenti dengan
  `ACCOUNT_CREDENTIAL_MISSING`, bukan mencoba akun sintetis atau mengaku akun nol.
- **TERHAMBAT untuk penerimaan akun live**. Seluruh pekerjaan software yang
  tidak memerlukan kredensial selesai; pengguna perlu memasukkan pair HMAC
  langsung di aplikasi. Kredensial tidak boleh diminta lewat chat/log/file proyek.
- Nol order/mutasi akun atau trial strategi. Source engine tetap cocok checksum.
  Pengujian account stream/reconciliation dan strategi/eksekusi tetap di luar
  putaran ini. Bukti dan batas ada di
  [DECISION.md](../../../outputs/binance-account-desktop-round01-20260916/DECISION.md).
