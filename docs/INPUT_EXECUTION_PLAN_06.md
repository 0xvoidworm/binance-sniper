# Input pasar dan rencana eksekusi — putaran0.6.6

## Kontrak sebelum implementasi — 20 September2026, 10:29 WIB

Permintaan: patch lanjutan sesuai desain otomatis, mempertimbangkan market
terkini dan batas/plan yang realistis agar sinyal layak dapat dieksekusi.
Anggaran satu putaran90menit, satu rancangan input/plan, bukan loop optimasi
hingga profit. Sumber awal audit600menit `b1a0baa280e4c6f3`:0entry,39/120candle
dilewati,94quote stale,69revisi candle, satu Short ditolak cap6ATR.

Scope: diagnosis GET harga/candle terkini; pilih snapshot bid/ask bertimestamp
output exchange yang bisa divalidasi; finalisasi candle baru dengan bukti
konsisten sebelum evaluasi; periksa planner lot/biaya/TP dan tambah rincian
penolakan/replay yang aman. Preview/aktivasi/entry wajib memakai perhitungan
yang sama. Perubahan batas strategi hanya jika dijelaskan dan lolos regresi
numerik; tidak memaksa entry, tidak menjanjikan jumlah transaksi/profit.

Tetap nativeRust, BTCUSDT USD-M5menit, Long/Short/menunggu otomatis, satu
posisi, market/reduce-only/SL-TP exchange dan ledger existing. Tidak mengubah
budget margin/risk/notional/leverage/funding/rugi atau catatan sesi pengguna.
Guard data5detik, rasio bersih≥1.1, SL2ATR, capTP6ATR, Stop/deadline/ownership,
no-duplicate, settlement sebelum reentry dipertahankan. Bukan migrasi HFT,
multi-simbol atau pemasangan robot/paket baru tanpa kebutuhan teknis nyata.

Verifikasi: backup source/binary/state, tes regresi sebelum/sesudah pada kasus
yang ditemukan, pipelineRust penuh, perbandingan publik terbatas dan satu
probe GET input≤10menit, dua setup GET, native capture, instalasi hanya idle
dengan lock dan state identik. Semua hasil negatif dicatat. Tidak menjalankan
live/paper/order-test/POST/DELETE atau mengubah akun; aktivasi oleh pengguna.
Dependency/toolchain terpin dipertahankan bila existing mencukupi. Hasil:
laporan perubahan, paket, syarat trigger dan batas market terukur; target
selesai software terverifikasi, bukan acceptance trading24jam atau profit.

Bukti: `~/Workspace/outputs/binance-input-plan-066-20260920-qut8Al`.

## Keputusan implementasi

1. Quote menggunakan depth5, bukan bookTicker. ID pembaruan dan E/T exchange
   diperiksa, level1–5 harus positif/urut/tanpa duplikasi, bid<ask. Kedua umur
   harus≤5detik; Quote tetap menggunakan T, tidak diganti waktu respons lokal
   atau E untuk menyembunyikan transaksi usang. Endpoint dibatasi Futures,
   BTCUSDT, limit5 sebelum jaringan. DNS fallback TLS/TTL existing tetap.
2. Candle baru memerlukan dua history identik terpisah≥2detik menurut jam
   monotonik DAN server. Pembacaan pertama disimpan sebagai calon, bukan
   baseline permanen. Calon yang berubah ditunggu ulang; revisi pada baseline
   lama tetap ditolak, direwarm dua kali baca, dan candle terkait dilewati.
3. Bug interaksi pemulihan berhasil direproduksi: frame sehat masih membawa
   baseline lama saat candle baru pending, lalu konfirmasi bisa memicu candle
   pemulihan. Sekarang interval tertutup saat data pulih langsung dikonsumsi.
   Tidak ada catch-up, perpanjangan15detik atau pengabaian Stop/deadline.
4. Planner biaya tidak terbukti salah secara matematika: oracle15.000kombinasi
   Long/Short, ATR, jarak mark dan notional sudah PASS pada kode lama. Jangan
   menyebut Short09:45 sesi600menit sebagai false rejection: snapshot ekonomi
   lengkap tidak tersimpan. SL2ATR, TP3–6ATR, RRnet≥1.1 dan ATR0.10–3.00% tetap.
   Penolakan cap sekarang menampilkan harga TP yang diperlukan, cap dan ATR.
5. Preview menampilkan kelayakan Long/Short pada ATR aktual dengan qty/SL/TP/
   risiko/hasil-target-net/RR. Planner tunggal dipakai review dan eksekusi;
   maksimal8percobaan sizing terekam di `LIVE_ENTRY_REVIEW`. Data whitelisted
   tanpa wallet/credential/raw private response. Roundtrip fixture replay
   ekonomi lulus; bukan replay candle/strategi/account keseluruhan.
6. Tambah diagnostik endpoint publik lambat/gagal dan konfirmasi candle.
   Tidak ada crate, paket OS, runtime atau perubahan jaringan sistem. Cargo.lock
   hanya mengubah versi paket aplikasi0.6.5→0.6.6; pin Rust1.98.1 tetap.

Menurut [dokumentasi resmi Binance USD-M market data](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data),
depth menyediakan E/outputtime, T/transactiontime dan lastUpdateId. Limit5
memakai weight2, sama dengan bookTicker satu simbol. Pemilihan snapshot lebih
sesuai didukung perbandingan pendek, bukan bukti seluruh gangguan lama hilang.

## Bukti awal dan batas eksperimen

- Regresi candle FAIL sebelum patch; regresi recovery-pending FAIL sebelum
  guard tambahan. Keduanya PASS sesudah patch. Seluruh137tes desktop lulus,
  2tes publik diabaikan pada suite default. Salah satunya (perbandingan20sampel)
  dijalankan eksplisit dan PASS. Fmt/Clippy/doc/release/audit489 lulus.
- Dua puluh sampel GET publik: depth transactionage101–118ms; bookTicker
  bervariasi dan selalu lebih tua dalam sampel berurutan tersebut. Metode
  membaca ticker dahulu, lalu depth, lalu time; perbedaan termasuk urutan
  request, bukan benchmark jaringan acak/serentak atau bukti10jam.
- Percobaan curl melalui DNS sistem gagal konek sekali. DNS-over-HTTPS read-only
  dan TLS resolve dengan IP hasil TTL berhasil. Tidak mem-pin IP pada produksi.
- Sesi asli tetap600menit, margin100/risk1/notional200/leverage5/funding0.1,
  rugi3/sesi dan5/hariUTC, maksimal5transaksi. Hash baseline diverifikasi
  sebelum probe, tanpa perubahan catatan. Tidak mengaktifkan live/paper.
- Satu probe input600detik dan dua setup GET diselesaikan sekali. Tidak
  mengulang probe hingga memperoleh hasil yang terlihat bersih.

## Market dan batas eksekusi yang realistis

Dua GET setup pada20September2026 10:41:51 dan10:41:58WIB **PASS**. ATR14
5menit141.00386658USDT /0.1757% sudah di dalam filter; fee akun5bps taker,
minimum0.001BTC dan50USDT. Pada snapshot kedua bid80325.00/ask80325.10,
mark80324.09373913. Hasil planner pada ATR aktual, **bukan order atau sinyal**:

| Arah hipotetis | QtyBTC | SLUSDT | TPUSDT | Risiko modelUSDT | Hasil bila target model tercapaiUSDT |
|---|---:|---:|---:|---:|---:|
| Long | 0.002 | 80042.0 | 80944.7 | 0.8596 | 0.9457 |
| Short | 0.002 | 80607.2 | 79706.8 | 0.8575 | 0.9434 |

Kedua rasio bersih≥1.1, dua iterasi sizing; TP masih di bawah cap6ATR.
Notional konservatif160.6663USDT; kebutuhan balance model sekitar32.3945USDT,
bukan kewajiban memakai seluruh cap margin100. Cadangan fee bolak-balik
sekitar0.161USDT dan funding0.1USDT masuk perhitungan bersama slippage.
Angka harga di atas kedaluwarsa setelah snapshot; **jangan diisi sebagai SL/TP
manual**. Mesin menghitung ulang dengan quote, fee, lot dan sisa batas rugi
ketika ada sinyal aktual. Ini bukan perkiraan keuntungan sesi atau bukti
bahwa target akan tercapai sebelum timeout posisi30menit.

Batas yang dipertahankan: margin100USDT, risiko1/posisi, notional200, leverage5,
rugi3/sesi dan5/hariUTC, fundingreserve0.1, slippage1bps, bufferlikuidasi10bps,
durasi600menit dan maksimal5transaksi selesai. Nilai ini batas pengguna yang
sudah tersimpan, bukan rekomendasi menambah dana/risiko. Kerugian nyata bisa
melewati model saat gap/slippage/likuidasi; target transaksi tidak dijamin.

Plan eksekusi tetap jelas dan otomatis:

1. Tunggu candle baru setelah aktivasi, grace2detik dan dua pembacaan sama
   terpisah≥2detik. Usia sinyal tetap≤15detik; terlambat berarti dilewati.
2. ATR0.10–3.00% lalu breakout/pullback/range memilih Long/Short; konflik/tanpa
   pola berarti menunggu. Quote/account/order harus segar, akun kompatibel,
   belum ada posisi/order asing, dan di luar10menit sebelum funding.
3. Planner memakai batas efektif terketat dan lot exchange; SL2ATR tidak
   digeser. TP mulai3ATR disesuaikan biaya sampai6ATR dengan RRnet≥1.1.
   Jika tidak ada lot/target layak, tampilkan rincian penolakan, tidak memaksa.
4. Sinyal+plan yang lolos dapat mengirim market entry setelah aktivasi pengguna;
   program memasang SL/TP exchange, merekonsiliasi fill, lalu mengelola exit
   reduce-only. Stop/deadline/rugi, proteksi gagal dan ketidakpastian tidak
   diabaikan. Tidak ada resend entry jika status kirim belum pasti.
5. Setelah posisi/order selesai DAN fill/fee/funding dibukukan, kembali
   menunggu candle berikutnya. Satu trade tidak otomatis mengakhiri sesi;
   tetap tunduk durasi, batas rugi, maks5transaksi dan guard lain. Arah berikut
   bisa berbeda. Tidak ada pelatihan/belajar strategi otomatis dalam sesi.

Mengapa tidak menurunkan ATR/menambah risk:59candle pada sesi lama memang
bervolatilitas di bawah filter, sementara39candle terlewat karena recovery.
Patch ini memperbaiki peluang mengevaluasi candle yang layak, bukan membuat
setiap candle menjadi sinyal. Satu Short lama belum dapat dibuktikan salah
sizing. Snapshot sekarang sudah layak dengan batas existing; memperbesar
risiko atau capTP tanpa bukti bukan perbaikan bug yang diperlukan.

## Hasil akhir dan keputusan

**SELESAI: patch0.6.6 terpasang, aplikasi berjalan baca saja, sesi tidak aktif.**
Instalasi20September2026 10:52:55WIB, unit`binance-desk-debug-066.service`,
PID74909 pada pemeriksaan. Unit065 hanya dihentikan setelah lock sesi berhasil,
status finished/currentNone dan seluruh baseline checksum cocok. Binary diganti
atomik dengan backup0.6.5 tetap ada; tidak ada state trading yang ditulis ulang.

SHA256 release, `~/.local/bin/binance-desk` dan `/proc/74909/exe` sama:
`77842c0b8492423e0403613d332960954d9e720302da1e95d70e0897ad774dcb`.
Seluruh14file state/outbox/ledger tetap identik. Sesi terakhir tetap finished,
sequence0/completed0/currentNone,600menit. Startup melaporkan akun terhubung
baca saja. Capture nativeX11 v0.6.6 diperiksa; akun dan batas disamarkan.

Probe produksi GET-only **600detik PASS/exit0** sekitar10:42–10:52WIB:

- 168 snapshot valid, 0 error, 0 revision candidate, 0 rewarm, 0 held snapshot.
- 2 rollover/konfirmasi candle; keduanya dievaluasi dalam 15 detik, 0 sinyal.
- Quote T 193–686 ms; umur account maksimum 3916 ms; gap sehat maksimum 7598 ms.
- Candle 10:45 dikonfirmasi 12486 ms setelah close. Dua log calon berarti calon
  berubah sebelum konfirmasi (inferensi dari cabang logger); tidak menjadi
  revisi baseline/skip. Candle 10:50 dikonfirmasi 7536 ms. Keduanya tanpa pola.
- Ada DNS/clock/public request lambat, jadi bukan klaim koneksi selalu cepat.
  Semua data pada probe ini tetap memenuhi guard. Tidak ada order/mutasi.

Pengujian akhir:137desktop PASS/2network ignored pada suite default;36engine
PASS; perbandingan20sampel publik eksplisit PASS; fmt/Clippy/doc/release/audit489
PASS. Lockfile hanya versi aplikasi, tidak ada paket tambahan. Regresi FAIL
sebelum patch tetap tersimpan, demikian juga kegagalan curl koneksi awal.

Keputusan: terima patch input/diagnostik untuk evaluasi pengguna berikutnya;
**belum menerima klaim profit, peningkatan frekuensi trade atau full-live24jam**.
Belum ada order aktif nyata pada versi ini; proteksi/partial/restart/cancel
masih diverifikasi fixture, bukan diganti klaim dari probe akun flat. Kondisi
market berubah dan konfirmasi dekat15s masih dapat melewatkan candle. Tidak
ada sesi live/paper baru, accountwrite, POST/DELETE, ordertest atau penambahan
modal/risiko oleh agen. Pengguna mengaktifkan sesi melalui GUI jika melanjutkan.

Artefak utama: `workbench-check.log`, `engine-tests.log`,
`public-quote-compare.log`, `setup-get.log`, `input-probe-600s.log`,
`native-066.png`, `installed-checksums.log`, `state-unchanged-after.log`.
Folder`rollback` menyimpan source/pin/binary/state awal. Restore binary hanya
saat idle di bawah lock; jangan mengembalikan snapshot state setelah transaksi
baru karena dapat membuang bukti order/ledger yang lebih baru.
