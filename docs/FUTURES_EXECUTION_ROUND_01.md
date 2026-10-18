# Dukungan eksekusi Futures 01 — 16 September 2026

## Lingkup sebelum implementasi

Pengguna meminta mulai membangun margin/leverage dan order nyata, tanpa menambah
paper, serta menegaskan produk otonom setelah aktivasi sesi oleh pengguna.

- Satu putaran Rust maksimal 110 menit aktif, nol optimasi strategi/sesi paper baru.
- Komponen Futures production: model leverage bracket/maintenance margin,
  sizing desimal memakai modal margin dan risiko terpisah, estimasi likuidasi
  isolated, fee aktual akun, metadata instrumen dan kesiapan akun read-only.
- Protokol typed untuk market entry/reduce-only exit, TP/SL melalui Algo Order,
  query/cancel berdasarkan client ID stabil, isolated margin dan leverage.
- Transport signed fixed-host, GET/POST/DELETE allowlist, secret hanya memori,
  tanpa retry mutation ketika hasil ambigu. Outbox durabel mendahului pengiriman,
  deteksi restart/duplikat dan rekonsiliasi. Driver dapat dipanggil controller
  otomatis; tidak mensyaratkan persetujuan manusia per order dalam sesi live.
- UI native memperlihatkan kesiapan Futures dan perhitungan margin/leverage;
  tidak menyalakan uang nyata melalui sesi agen ini. Pertahankan fitur existing.
- Batas awal: USD-M USDT linear one-way isolated; Hedge/multi-assets/Portfolio
  Margin ditolak, bukan diubah diam-diam. Estimasi likuidasi bukan harga pasti.
- Jangan menyatakan siap otonom end-to-end jika supervisor/recovery/proteksi
  penuh masih belum terintegrasi. Catat batas nyata pada evaluasi penerimaan.

## Penerimaan dan bukti

1. Uji margin, tier/leverage, desimal/filter, risiko/biaya, long/short/likuidasi.
2. Uji kontrak order/algo/signing, redaksi, error unknown/429/5xx, outbox durabel,
   duplikat/restart dan rekonsiliasi; gunakan server/transport lokal uji.
3. Pipeline Rust, build/install UI native, probe Binance GET-only untuk data
   bracket/fee/account. Tidak ada POST/DELETE Binance oleh agen, termasuk order-test.
4. Bukti di `Workspace/outputs/binance-futures-execution-round01-20260916`.
   Keputusan LANJUT hanya untuk komponen yang terverifikasi, bukan profit atau
   aktivasi trading nyata. Jika integrasi belum lengkap, laporkan apa yang kurang.

Referensi: [Trade API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade),
[Account API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/account),
[General info](https://developers.binance.com/en/docs/products/derivatives-trading-usds-futures/general-info).

## Keputusan penerimaan

**TERIMA komponen yang terverifikasi; live end-to-end belum siap.** Versi0.5.0
terpasang, 67 tes desktop +36 engine lulus, pipeline penuh/audit489 lulus,
GET bracket/fee/account nyata berhasil, native capture final diperiksa.
Tidak ada POST/DELETE Binance atau sesi paper baru.

Outbox dan pengendali keputusan otomatis tersedia; supervisor/recovery sesi,
ledger nyata, batas rugi dan aktivasi live masih harus diintegrasikan.
Satu aktivasi sesi oleh pengguna tetap menjadi desain produk; tidak meminta
klik manual per order. Putaran selesai dalam batas waktu yang ditetapkan.

[Hasil, kegagalan yang diperbaiki dan bukti](../../../outputs/binance-futures-execution-round01-20260916/DECISION.md).
