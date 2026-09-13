# Phase 2 — causal event study

## Kesimpulan utama

Infrastruktur riset dan jurnal sudah berjalan, tetapi baseline strategi **belum menunjukkan edge**. Selama 90 hari pada enam pair, hanya 27 sinyal muncul. Kedua periode menghasilkan rata-rata R setelah asumsi biaya yang negatif. Ini bukti untuk memperbaiki hipotesis pola, bukan dasar mengaktifkan order atau menaikkan leverage.

| Periode | Sinyal selesai | Mean net R | Win rate net | Profit factor | Mean stress net R |
| --- | ---: | ---: | ---: | ---: | ---: |
| Development, 60 hari | 16 | -0.395672 | 25.0% | 0.069965 | -0.651719 |
| Holdout, 30 hari | 11 | -0.403454 | 45.45% | 0.226870 | -0.653217 |

R = perubahan harga per unit dibagi jarak modeled entry ke stop. Bukan 1% modal, bukan return akun, dan tidak memakai leverage/sizing. Sample kecil dan event lintas pair berkorelasi. Penjumlahan event R bukan kurva equity akun.

## Dataset dan provenance

- Binance USDⓈ-M Futures: BTCUSDT, ETHUSDT, SOLUSDT, BNBUSDT, XRPUSDT, DOGEUSDT.
- Periode sinyal: 2026-06-15 00:00 UTC hingga sebelum 2026-09-13 00:00 UTC.
- H4/H1/M15/M5; 220 candle pemanasan per feed. Total 228,840 candle, 24/24 feed lengkap tanpa gap dalam rentang yang diminta.
- 155,520 keputusan M5 selama periode studi; data pemanasan tidak dihitung sebagai keputusan live.
- 270 event funding publik per pair; 1,620 total. Funding mark/rate/waktu dipakai oleh model.
- Dataset ID `binance-1781481600000-1789257600000-v1`.
- Run final `history-6d32c2f80979f7e5dec0486f`.
- Source/config hash `17473b25bc8a6cf97154ea957a9810481a2688d4b59f5da64a35500292ceae72`.
- Checksum isi candle dihitung ulang saat studi; manifest feed, hash funding, parameter, dan hash source tersedia di JSON report. Dua run instrumentation menghasilkan sinyal/outcome sama; bukan dua sampel independen.

Sumber endpoint dan skema diperiksa di [dokumentasi resmi market data Binance USDⓈ-M](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data). Pengunduhan hanya memakai GET public klines/server time/funding; tidak menyentuh endpoint perdagangan.

## Waktu keputusan dan model hasil

Scanner produksi yang sama dijalankan berurutan. Pada penutupan M5, hanya H4/H1/M15 yang sudah tertutup pada waktu itu yang tersedia. Zona baru aktif paling awal ketika seluruh konteks telah diketahui; patch ini mencegah warmup memakai konteks baru untuk menghasilkan entry di masa sebelumnya.

Model hasil terpisah dari four-bar excursion tracker lama:

1. Entry hipotetis pada open M5 setelah sinyal, ditambah slippage merugikan arah posisi.
2. Stop/target dari referensi sinyal; next-open di luar bracket tidak dipaksakan menjadi fill.
3. Keluar pada stop, target, atau close candle keempat. Gap stop memakai open yang lebih buruk; gap target tidak diberi bonus favorable.
4. Jika high-low candle yang sama menyentuh stop dan target, baseline memakai stop-first, dengan alternatif target-first ditandai. OHLC tidak mengungkap urutan tick.
5. Fee 5 bps dan slippage 2 bps per sisi; stress 7.5 bps dan 5 bps. Ini asumsi, bukan tier fee akun atau observasi order book.
6. Funding historis dihitung per unit dari rate × mark × arah; timing boundary/intrabar exit yang tidak pasti diperlakukan konservatif. Live belum merekonsiliasi funding, sehingga `net_r` all-cost kosong.

Horizon penuh wajib berurutan; data hilang tidak diisi angka buatan. Exit dapat selesai sebelum candle keempat. Rugi biaya/slippage/gap bisa melampaui 1R.

## Pemisahan temporal

Split ditetapkan sebelum evaluasi outcome pada 2026-08-14 00:00 UTC. Sinyal development dengan horizon melintasi split dibuang; empat candle awal holdout menjadi embargo. Sampel dengan horizon melewati akhir dataset juga dikecualikan. Pada run ini tidak ada sinyal yang jatuh di batas pengecualian. Tidak ada pencarian parameter atau pemilihan pemenang atas holdout.

Ini satu holdout, **bukan walk-forward**. Sekarang hasil holdout sudah diketahui; perubahan strategi berikutnya harus menganggap periode itu sebagai data yang pernah dilihat dan menyediakan validasi baru.

## Apa yang terukur dari baseline

- Sinyal per pair: BTC 3, ETH 1, SOL 7, BNB 5, XRP 5, DOGE 6.
- 366 zona terbentuk selama replay; hanya 27 menghasilkan sinyal. Pipeline memang jarang memicu.
- 23 exit model karena four-bar close, 4 karena stop, 0 target, 0 candle exit ambigu pada sample ini.
- Mean sebelum fee/funding tetapi **setelah slippage**: -0.089005R. Mean fee sendiri sekitar 0.309704R. Masalah hasil bukan semata biaya; mean setelah slippage sudah negatif.
- State gate H4 tidak lolos pada 107,955 dari 155,520 pengamatan (69.42%). Angka ini bukan jumlah trade gagal; zona yang dibekukan tetap dapat dilacak saat bias H4 berubah.
- Area saat ini terpusat di **origin impulse** ± 0.2 rata-rata range M15, sedangkan validasi pullback 25–70%. Itu strategi retest origin, bukan otomatis sweet spot di dalam area retracement. Ini kandidat hipotesis riset berikutnya, bukan pembenaran mengubah hasil run yang telah dibekukan.

## Cara melihat

Buka `http://127.0.0.1:8765`, refresh tab lama, lalu pilih **Research · Phase 2**. Historical menampilkan laporan di atas; Live Journal menampilkan observasi baru saja. **Lihat candle** menampilkan 50 candle sampai sinyal plus paling banyak empat candle sesudahnya dengan referensi entry/stop/target. Area setelah keputusan diberi warna dan label hindsight. Tidak ada zona palsu yang digambar dari perkiraan.

Jurnal tersimpan di `state/research.sqlite3`; laporan ekspor ada di `Workspace/outputs/binance-sniper-phase2-90d.json`. Opening dashboard/report tidak mengirim order, memulai studi, mengubah leverage, atau mengubah API whitelist.

## Batas dan langkah riset berikutnya

Belum ada fill nyata, order book historis, latency execution, tick/lot rounding, sizing, liquidation, korelasi portfolio, atau multi-day reliability proof. Universe sekarang juga punya selection bias. Model ini dapat menolak hipotesis buruk pada sampel, tetapi tidak membuktikan keuntungan masa depan.

Prioritas berikutnya: tulis hipotesis area entry alternatif yang terukur, cocokkan target gerak dengan horizon 20 menit, ukur biaya relatif terhadap jarak stop, dan validasi pada data baru. Jangan menyamakan memperlonggar filter atau menaikkan leverage dengan menemukan edge.
