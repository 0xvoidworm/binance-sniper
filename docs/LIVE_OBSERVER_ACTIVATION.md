# Aktivasi observasi pasar — 14 September 2026

## Kontrak sebelum implementasi

- Permintaan: mesin harus berjalan dan dapat diamati; konteks pengguna adalah tanpa order nyata.
- Scope: sambungkan engine kanal H4 v2 yang sudah ada ke feed publik Binance USD-M dan halaman `/observe`. Kedua konfigurasi 20/10 dan 55/20 pada BTC/ETH ditampilkan setara, bukan seleksi kandidat baru.
- Anggaran kerja internal: satu integrasi dan satu siklus verifikasi operasional; nol pencarian parameter, nol backtest historis baru. Ini batas pelaksana, bukan angka persetujuan pengguna.
- Data warmup 220 candle hanya untuk indikator, tidak membuat trade masa lalu. Sesudah aktivasi, engine memproses candle H4 yang baru selesai. Posisi/fill adalah **model OHLC yang dicatat pada penutupan bar**, bukan eksekusi paper intrabar atau fill Binance.
- Harga preview terus diperbarui, tetapi tidak diperlakukan sebagai candle selesai. Jika koneksi stale/gap/revisi, tampilkan penghambat dan jangan meloloskan keputusan secara diam-diam.
- Catatan input persisten dan identitas kode menjaga restart/idempotensi. Hasil yang direkonstruksi setelah gangguan harus diberi label.
- Biaya fee/slippage memakai asumsi BASE yang lama. Funding live belum lengkap: jangan mengklaim net lengkap atau return akun/USD.
- Jaringan: fallback DNS-over-HTTPS hanya untuk GET data publik pada hostname resmi `fapi.binance.com`, sertifikat TLS tetap diverifikasi terhadap hostname tersebut. Tidak mengubah Windows DNS, hosts, kredensial, private reader, atau WebSocket.
- Selesai bila: data publik aktual bergerak, observer di API/UI sesuai data engine, restart tidak menduplikasi input, pengaman stale/gap dan no-mutation lolos tes. Bila koneksi tetap gagal, keputusan TERHAMBAT dengan bukti, bukan LIVE palsu.
- Putaran 01 tetap TOLAK. Aktivasi observasi bukan validasi profit dan bukan aktivasi Turtle harian atau transaksi live.

## Status

**OBSERVASI AKTIF**, tanpa order routing. URL: http://127.0.0.1:8765/observe.

Session mulai 2026-09-14 17:27:07 WIB. Empat tampilan model (BTC/ETH × 20/10 dan 55/20), masing-masing warmup 220 bar, nol trade baru ketika diaktifkan. Decision boundary berikutnya 19:00 WIB pada hari aktivasi; tidak ada janji sinyal/trade pada boundary itu.

283 Python / 23 JavaScript tests passed, pip check bersih; satu warning Starlette/AnyIO lama. Live REST 24/24, lag 0, task 29/29 RUNNING. Harga dan level UI diperiksa terhadap API; desktop/mobile tanpa overflow. Restart aktual mempertahankan session dan jumlah input, browser menampilkan disconnect lalu pulih. Layanan dibiarkan berjalan menggunakan runner proyek; tidak ada auto-start Windows.

Network diagnosis: resolver lokal memberi 103.133.62.251 untuk fapi/fstream dan koneksi timeout; resolver publik memberi alamat berbeda yang menerima koneksi HTTPS terverifikasi ke hostname asli. Perbaikan hanya public REST. Akun masih AUTH_UNAVAILABLE dan order counts tidak diketahui; tidak berarti ada atau tidak ada order akun. Tidak mengirim order/mutasi.

Engine/source/output audit v1/v2 tidak diedit; adapter baru terpisah, tidak menjalankan CLI studi atau P&L historis ulang. Input waktu depan disimpan ke DB observer baru. Net lengkap null karena funding live missing. Tidak ada kandidat baru atau trading uang asli diaktifkan.

Rujukan teknis jaringan: [Binance USD-M General Info](https://developers.binance.com/en/docs/products/derivatives-trading-usds-futures/general-info) dan [HTTPcore network backends](https://www.encode.io/httpcore/network-backends/). Koneksi tetap memakai REST mainnet resmi dan TLS hostname; bukan harga spot/testnet.
