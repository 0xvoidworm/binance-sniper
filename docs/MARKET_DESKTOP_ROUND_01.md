# Putaran pasar desktop 01 — 16 September 2026

Mulai 16:20 WIB. Pengguna meminta melanjutkan desain aplikasi/mesin sebelumnya
dan melaporkan pada titik selesai yang ditetapkan. Batas tahap: feed pasar publik
yang benar-benar terlihat dan dapat dipakai pada aplikasi native.

## Lingkup sebelum implementasi

- Rust, aplikasi `desktop-rs` existing; pertahankan engine, simulasi, Python/web lama.
- BTCUSDT dan ETHUSDT, pilihan tampilan Spot / USDⓈ-M, timeframe 1m/5m/15m/1h.
  Default tampilan USDⓈ-M mengikuti referensi proyek; bukan pilihan akun final.
- Candle OHLCV publik, polling REST dua detik pada worker di luar UI, sejarah
  terbatas, zoom/pan/reset, harga watchlist dan waktu pembaruan.
- Status menyambung/aktif/data terlambat/gagal/jeda; retry/backoff dan perbaikan
  sejarah setelah koneksi pulih. Data lama tidak boleh ditampilkan sebagai baru.
- DNS sistem diuji dahulu. Jika koneksi gagal, fallback DNS-over-HTTPS hanya
  untuk host publik Binance, TLS dan hostname tetap diverifikasi; tanpa ubah DNS OS.
- Tanpa API key, akun, order, sinyal strategi, HFT atau riset profit.
- Anggaran satu integrasi, maksimal 90 menit aktif sebelum evaluasi; nol trial strategi.

## Penerimaan / titik berhenti

1. Parser/urutan candle, stale/reconnect, pergantian pilihan dan kontrol chart diuji.
2. fmt/Clippy/test/doc/release/audit dependency terkunci lulus.
3. Data nyata BTC/ETH dibaca, beberapa pembaruan dan pemulihan setelah jeda
   diverifikasi; kegagalan jaringan dilaporkan dengan jujur.
4. Native framebuffer live dan ukuran minimum diperiksa; aplikasi diperbarui
   user-local dan dibuka agar hasil dapat dilihat pengguna.
5. Bukti/keputusan di `Workspace/outputs/binance-market-desktop-round01-20260916`;
   STATUS/HANDOFF/KNOWN_GAPS diperbarui.

LANJUT = tahap feed/UI dapat dipakai, bukan kelayakan strategi atau izin trading.
Jika transport tidak dapat dipulihkan dalam batas ini, catat TERHAMBAT beserta
diagnostik; jangan mengganti data nyata dengan fixture berlabel live.

## Referensi

- https://developers.binance.com/docs/derivatives/usds-margined-futures/market-data/rest-api/Kline-Candlestick-Data
- https://developers.binance.com/docs/binance-spot-api-docs/rest-api/market-data-endpoints
- https://docs.rs/reqwest/0.13.5/reqwest/struct.ClientBuilder.html

## Hasil — LANJUT untuk observasi pasar native

Selesai sekitar 16:40 WIB, di bawah batas 90 menit. Binance Desk 0.2.0 terpasang
dan dibuka. Seluruh delapan kombinasi pasar/timeframe membaca BTC/ETH nyata;
pause/resume lulus. Pemantauan 70 detik menghasilkan 34 batch tambahan, tanpa
gangguan koneksi teramati. Jendela native normal dan minimum diperiksa.

16 tes dan fmt/Clippy/doc/release/audit 422 dependency lulus. Scope tetap feed/UI;
tidak ada akun, strategi, order, backtest atau bukti profit. Kegagalan layout dan
koneksi awal disimpan beserta perbaikannya. Engine existing tetap identik.

[Bukti, perbaikan dan batas](../../../outputs/binance-market-desktop-round01-20260916/DECISION.md).
