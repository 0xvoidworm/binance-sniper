# Binance Desk

Aplikasi desktop Linux native untuk mesin trading pribadi. Seluruh UI ditulis
dalam Rust memakai [egui/eframe](https://docs.rs/eframe/0.36.2/eframe/), memakai
Wayland/X11 dan renderer OpenGL. Tidak memakai browser, WebView, server HTTP
lokal, atau JavaScript.

Patch **0.6.11** membedakan data usang dari data invalid: saat flat jeda entry,
lewatkan candle pemulihan; saat ada intent tetap tutup/accounting dan recovery
terbatas. Clock snapshot memakai lease GET sebelum pembacaan data, tanpa GET
waktu tambahan setelah quote. Guard 5 detik, Stop dan risiko tetap.
[Rincian](../docs/STALE_CLOCK_11.md); status pemasangan aktual di STATUS.

Patch **0.6.10** memisahkan penutupan posisi akibat transport sementara dari
Stop seluruh sesi. Maksimal dua episode dengan intent; setelah posisi/order
kosong dan accounting cocok, perlu 60 detik data sehat (gap ≤10 detik), lalu
menunggu candle baru. Stop/deadline/batas uang/integritas/proteksi tidak pasti
tetap terminal. Klasifikasi error, log time/DNS dan batas persiapan 20 detik
diperjelas; tidak mengubah strategi/risiko. [Rincian](../docs/TRANSPORT_RECOVERY_10.md).
Status pemasangan/probe aktual ada di [STATUS](../STATUS.md).

Patch **0.6.9** menambahkan enam GET snapshot akun serentak dan terbatas,
cooldown bersama, log durasi per endpoint, counter keputusan aktual dan versi
runtime/sesi. Tidak melonggarkan guard5detik atau risiko. Hermes/model tetap
tidak aktif sesuai penundaan pengguna. Paket evaluasi0.6.8 tetap diterima;
diagnostics baru opsional, bukan rekonstruksi riwayat lama. Status pemasangan
terbaru ada di [STATUS](../STATUS.md); [diagnosis sesi78menit](../docs/ACCOUNT_SNAPSHOT_DIAGNOSTICS_09.md).

**0.6.8 (P2, termasuk 0.6.9)** menambahkan CLI offline
`evaluation export/validate/validate-proposal`: snapshot arsip final, allowlist,
Decimal/hash, paket privat atomik dan pemeriksaan fakta/proposal tanpa apply.
Tidak membaca KWallet, menjalankan model atau mengubah logika trading.
[Kontrak dan pemakaian](../docs/EVALUATION_PACKET_V1.md),
[hasil166+36tes dan batas integrasi](../docs/EVALUATION_BRIDGE_P2_08.md).
Hermes/model/pilot masih menunggu provider, batas biaya dan izin data keluar.

Patch **0.6.7 (P1, termasuk kandidat)** memisahkan history publik dari loop
pengelolaan posisi dan membagi satu persiapan DNS/waktu privat untuk GET order
dan akun. Guard data5detik tetap; quote juga diperiksa pada awal frame dan
sebelum entry. Settlement saat Stop tidak kembali menunggu sinyal. Catatan
exit kini membedakan alasan keputusan dari SL/TP/reduce-only yang terbukti
lewat ledger. Tidak mengubah strategi/risiko atau memasang Hermes.
[Laporan, kompatibilitas dan hasil uji](../docs/RELIABILITY_P1_07.md).
P1 awalnya belum dipasang; hasil deployment gabungan terbaru dicatat di STATUS.

Patch **0.6.6** mengganti quote preflight/eksekusi dengan depth BTCUSDT limit5,
memvalidasi ID, kedua timestamp exchange, urutan level dan spread. Umur quote
tetap memakai waktu transaksi exchange T, maksimal5detik; bukan waktu lokal.
Candle baru harus sama pada dua pembacaan terpisah≥2detik sebelum dikonsumsi.
Jika berubah selama konfirmasi, tunggu ulang; revisi candle yang sudah disahkan
tetap melalui rewarm dan skip. Pemulihan koneksi mengonsumsi interval tertutup
meskipun candle tersebut masih pending, sehingga tidak dikejar setelah konfirmasi.

Preview setup kini menyertakan hitungan Long/Short **pada ATR aktual**, terpisah
dari contoh ATR hipotetis: qty, SL/TP, risiko, hasil target bersih dan penolakan.
`LIVE_ENTRY_REVIEW` merekam input ekonomi terpilih dan maksimal8percobaan sizing
dengan planner yang sama, tanpa wallet/credential/respons privat mentah.
ATR0.10–3.00%, SL2ATR, TP3–6ATR, rasio bersih≥1.1 dan seluruh batas uang tetap.
Tidak ada paket baru, pembelajaran otomatis, aktivasi sesi atau janji profit.
[Kontrak, market terkini dan hasil patch](../docs/INPUT_EXECUTION_PLAN_06.md).

Patch **0.6.5** memindahkan rekonsiliasi order sebelum pengambilan frame akun/
harga. GET order berbagi persiapan DNS/waktu dalam batch terbatas; umur bukti
order dihitung sejak GET pertama dan harus ≤5 detik. Frame akun/quote tetap
harus segar, dan proteksi yang hilang pada snapshot terbaru tidak dianggap
masih aman. Metadata disegarkan sebelum batch jika mendekati TTL60 detik.
Respons market order tanpa `cumQuote` dikonfirmasi dengan satu GET ber-timeout,
bukan mengarang fill atau mengulang POST. Log mencatat umur data, tahap dan
latency tanpa credential/raw respons. Risiko, strategi, Stop dan ledger tidak
dilonggarkan. [Kontrak dan hasil patch](../docs/FRESHNESS_RECONCILIATION_05.md).

Sesi pengguna19September membuktikan satu entry, SL/TP, exit reduce-only dan
ledger produksi selesai; hasil bersihnya negatif. Ini bukan bukti profit,
ketahanan24jam atau penerimaan seluruh jalur order (misalnya explicit cancel).

Patch **0.6.4** menambahkan **Periksa kelayakan setup** (GET-only) dan
pemeriksaan ulang sebelum aktivasi. Preview dan entry memakai planner yang sama:
SL tetap 2 ATR, TP mulai 3 ATR dan boleh diperluas hingga 6 ATR untuk memenuhi
rasio bersih minimal 1.1 setelah fee, funding reserve dan slippage entry/exit.
Risiko, minimum ATR, minimum order dan batas likuidasi tidak dilonggarkan. Jika
tidak ada ukuran yang memenuhi semuanya, entry ditolak. Ini strategi baru
`ensemble-2-cost-aware`, bukan bukti edge; TP lebih jauh bisa mengurangi hit rate.
Budget margin adalah plafon, sehingga saldo tidak harus memenuhi seluruh plafon
selama rencana order aktual terjangkau. Alasan penolakan dicatat dalam journal,
dan analisis baru tidak memperpanjang durasi sesi historis yang sudah selesai.
Durasi 180–240 menit tidak membuat mesin belajar otomatis: pola tetap sama.
[Laporan dan konfigurasi kandidat](../docs/FEASIBILITY_LOGIC_04.md).

Patch **0.6.3** memperbaiki cache yang terus menolak revisi candle. Entry ditahan
saat perbedaan ditemukan; baseline baru memerlukan dua pembacaan penuh yang
identik, terpisah minimal dua detik pada waktu monotonik dan waktu server.
Candle yang terdampak/teramati selama pemulihan dilewati, bukan diperdagangkan.
Sinyal hanya boleh kembali dari candle yang lebih baru; jika sudah ada intent,
pengaman stop/rekonsiliasi tetap berlaku. REST meminta interval selesai dengan
`endTime` eksplisit dan grace dua detik, tanpa memperpanjang usia sinyal 15 detik.
Log merekam timestamp candle dan kolom/nilai publik yang berubah. UI menunjukkan
durasi aktual serta label **BERAKHIR LEBIH AWAL**. Tidak mengubah batas risiko,
leverage atau ambang strategi; startup tidak mengaktifkan sesi.

Uji panjang juga menemukan jeda data valid >30 detik. Build akhir 0.6.3 menjeda
entry selama tidak ada intent bot, sampai data pulih atau deadline sesi asli
habis; bukan menaikkan timeout reconnect. Snapshot usang tetap ditolak. Setelah
pulih, akun/posisi/order/wallet diperiksa dan candle pemulihan dilewati tanpa
entry. Jika sudah ada intent (termasuk pengiriman belum pasti), tetap memakai
Stop/rekonsiliasi. Deadline monotonik menyela GET yang menggantung; jika akun
belum dapat diverifikasi saat deadline, entry sudah berhenti tetapi penutupan
atau konfirmasi selesai tetap menunggu koneksi. Tidak ada restart otomatis.

Probe `--probe-auto --probe-auto-seconds 3600` mengamati input produksi GET-only
selama maksimal 60 menit. Hasil menyertakan gap antar-snapshot sehat, pergantian
candle, revalidasi dan keputusan. Error tetap dihitung dan exit code tetap
nonzero bila ada error. Metrik `within_30s_recovery_budget` hanya pembanding
batas versi lama, bukan batas pause versi akhir. Pemulihan bukan bukti tanpa
error, uji order nyata, atau keuntungan. [Catatan putaran](../docs/CANDLE_RECOVERY_03.md).

Patch **0.6.2** mengurangi sinkronisasi waktu menjadi sekali per batch akun,
dan menghitung umur snapshot sejak GET akun pertama, setelah persiapan DNS/waktu.
Gangguan data sementara menahan entry dan dicoba ulang sampai 30 detik sejak
snapshot sehat terakhir jika belum ada intent/posisi. Jika sudah ada intent,
pengguna meminta Stop, atau batas coba ulang habis, jalur penghentian tetap berlaku.
Batas kesegaran akun tetap lima detik. Patch ini juga mengambil ulang candle terbaru jika respons pertama setelah
pergantian interval masih tertinggal, sehingga cache tidak tertahan lima menit.
Penyebab penghentian sesi disimpan sampai selesai dan setelah restart. Tombol
pemulihan hanya tersedia untuk catatan sesi yang belum selesai; sesi baru tidak
dapat dimulai sebelum catatan tersebut diselesaikan. Status menampilkan umur
data, hitung mundur evaluasi candle dan angka volatilitas yang menahan sinyal.
Diagnosis polling berulang tersedia melalui
`binance-desk --probe-auto --probe-auto-seconds 360` (GET-only; batas 0.6.2 adalah 600 detik).
Parameter strategi dan batas risiko tidak diubah.

Patch **0.6.1** memisahkan pemulihan akun dari riwayat candle strategi. Sesi
yang sudah dihentikan dapat menyelesaikan rekonsiliasi saat riwayat candle gagal
atau direvisi, selama data akun/quote kembali segar. Footer membedakan sesi
tertahan dari sesi yang menunggu sinyal; pesan kegagalan juga masuk catatan sesi.
Batas sesi terakhir dimuat lagi setelah restart, dengan konfirmasi dana tetap
tidak dicentang dan tanpa aktivasi otomatis.

Versi **0.6.0** menyambungkan **Sesi otomatis** pada Eksekusi Futures: tiga aturan
strategi memilih Long/Short/menunggu, menghitung SL/TP dan ukuran posisi, lalu
supervisor mengirim entry, mengawasi proteksi, menutup dan mencocokkan hasil
transaksi. Aktivasi dilakukan sekali oleh pengguna dengan batas modal/risiko;
order berikutnya tidak membutuhkan klik manual. Semua komponen baru dalam Rust.
Siklus order diuji dengan exchange tiruan; pembacaan Binance asli GET-only.
Pada putaran0.6.0 penerimaan order uang nyata dan profit belum terbukti;
bukti satu siklus pengguna19September dijelaskan pada catatan0.6.5 di atas.

Versi **0.4.0** menyediakan **Mesin paper** pada harga publik dengan modal virtual.
Fitur existing dipertahankan; putaran 0.6.0 tidak memulai sesi paper baru.

Patch **0.5.1** memperjelas form sebagai **Kalkulator satu posisi — contoh manual**.
Pilihan Long/Short dan harga SL/TP hanya input contoh kalkulasi. Desain mesin
otonom kini menentukan arah dan SL/TP pada panel terpisah **Sesi otomatis**;
kalkulator contoh dapat dibuka sebagai bagian opsional.

Patch **0.3.1** memperbaiki penolakan simbol Futures beraksara Unicode. Daftar
simbol publik resmi pada 16 September 2026 dipakai sebagai fixture regresi;
nama tidak ditransliterasi atau diubah. Error respons kini menyebut endpoint
dan pemeriksaan yang gagal dengan label tetap, tanpa menyalin isi respons akun.

## Buka aplikasi

Setelah instalasi, cari **Binance Desk** pada menu aplikasi KDE, atau jalankan:

```bash
binance-desk
```

### Yang tersedia

- **Ruang kerja:** candle dan volume Binance BTC/ETH, pilihan Spot / USD-M
  Futures, timeframe 1m/5m/15m/1h, 240 candle per simbol. Harga watchlist berasal
  dari close candle terbaru. Data publik diperbarui sekitar dua detik via REST.
  Scroll untuk zoom, drag horizontal untuk riwayat, **Ke terbaru** untuk reset;
  hover menampilkan waktu UTC dan OHLCV. Tombol jeda/lanjut mengendalikan feed.
- **Koneksi:** menyambung/aktif/jeda/data terlambat/menghubungkan ulang. Umur
  pembaruan terlihat; setelah delapan detik tanpa batch baru, data tidak dianggap
  segar. Kesalahan mempertahankan chart lama dalam warna redup. Riwayat dimuat
  ulang setelah pulih; perubahan pasar/timeframe membuang konteks lama.
- **Akun Futures:** form API HMAC, koneksi baca saja untuk saldo wallet/tersedia,
  margin, P&L mengambang, posisi long/short, mode Hedge/One-way, leverage dan
  margin yang sedang berlaku. Nilai uang memakai desimal. Data yang belum
  berhasil dibaca tampil sebagai belum tersedia, bukan nol.
- **Eksekusi Futures:** aktivasi/stop sesi otomatis BTCUSDT, batas margin/risiko,
  leverage, notional, rugi harian/sesi, durasi serta jumlah posisi. Arah dan SL/TP
  dari strategi. Menampilkan alasan, posisi, hasil bersih, fee/funding dan status
  rekonsiliasi. Preflight GET dan kalkulator manual tersedia di bawah panel sesi.
- **Mesin paper:** sesi 60 menit, alasan menunggu/entry/exit, posisi dan TP/SL
  virtual, ekuitas/P&L virtual, stop sesi dan jurnal persisten. Lihat aturan di bawah.
- **Demo sintetis:** demo `SYNTHUSDT`, chart quote sintetis dengan entry/SL/TP,
  ekuitas, P&L, posisi, order, proteksi dan risiko. Tombol jalankan/jeda,
  maju/mundur, reset, slider waktu dan klik chart untuk memilih langkah.
- **Jurnal demo:** event dan effect engine hingga langkah yang dipilih; pencarian
  teks dan ekspor sesi ke `~/Workspace/outputs/binance-desk-demo-*`.
- **Pengaturan:** skala UI, grid chart, kecepatan demo; simpan eksplisit ke
  `~/.config/binance-desk/settings.json` (mengikuti `XDG_CONFIG_HOME` jika ada).

Demo menggerakkan library `../engine-rs` dan menampilkan state sebenarnya pada
setiap langkah. Harga/fill/biaya/modal berasal dari satu fixture yang dibekukan;
bukan feed Binance, akun pengguna, strategi, atau bukti profit. Replay disimpan
di memori; ekspor merekonstruksi jurnal persisten sampai langkah terpilih dan
memastikan state-nya cocok. Penutupan aplikasi mengakhiri replay yang belum diekspor.

Koneksi pasar publik tersedia tanpa API key. Konektor akun baca saja tersedia
mulai 0.3.0; autentikasi akun nyata memerlukan key/secret di aplikasi. Alokasi
modal dan order nyata dipakai supervisor Futures setelah aktivasi eksplisit di UI.
Strategi paper terpisah tersedia.
Pengguna memilih **Futures** pada
16 September 2026 untuk mesin trading; default aplikasi sudah USD-M Futures.
Selector Spot/Futures hanya mengubah pasar yang dilihat, tanpa mengubah akun,
margin atau leverage. Leverage sesi ditetapkan pada form sesi. Hasil penelitian lama
yang gagal tetap dipertahankan.

### Sesi Futures otomatis

```bash
binance-desk --page futures
binance-desk --probe-futures  # dua pembacaan GET, output status tanpa nilai akun
binance-desk --probe-auto     # input strategi/akun/fee; GET saja, tidak aktivasi
binance-desk --check-setup    # dua snapshot GET; memeriksa batas sesi terakhir
# --check-setup --setup-config /path/config.json memeriksa Config tanpa menyimpannya
binance-desk --finish-empty-session # perbaikan catatan kosong; tutup aplikasi dahulu
```

1. Hubungkan key melalui **Akun Futures** atau simpan di KWallet.
2. Buka **Eksekusi Futures → Sesi otomatis**. Isi budget margin, risiko per
   posisi, maksimum notional, rugi sesi/harian UTC, cadangan funding, leverage,
   durasi dan maksimum transaksi per sesi. Risiko posisi harus ≤ kedua batas rugi ≤ budget
   margin; cadangan funding harus < risiko posisi. Nilai modal/risiko kosong
   sampai diisi pengguna; kalkulator contoh tidak memasok nilai ke sesi ini.
3. **Analisis arah sekarang** hanya membaca satu snapshot dan menjelaskan sinyal.
   **Periksa kelayakan setup** memeriksa ukuran/biaya dengan batas yang diisi.
   Status **LAYAK BERSYARAT** berarti ditemukan contoh ATR yang layak pada
   snapshot itu, bukan bahwa pasar sekarang memberi sinyal. Perubahan isian
   membatalkan kesesuaian hasil preview. Aktivasi selalu memeriksa ulang.
4. Centang penggunaan dana nyata lalu **Aktifkan sesi live** untuk satu sesi.
   Software memeriksa akun flat, menyesuaikan Isolated/leverage jika perlu, lalu
   menunggu candle baru. Tidak mengejar sinyal candle sebelum sesi dimulai.
5. **Hentikan & tutup posisi** melarang entry baru, menutup posisi milik sesi,
   membersihkan proteksi setelah flat, dan memeriksa fill/biaya sebelum selesai.
   Leverage dan mode Isolated tetap mengikuti pengaturan terakhir sesi.

Hanya BTCUSDT USD-M standar, One-way, Single-asset USDT, Isolated dan komisi USDT.
Fee BNB ditolak sebelum perubahan konfigurasi; tidak dimatikan diam-diam.
Jangan menjalankan order/bot lain atau transfer pada akun yang sama selama sesi:
perubahan wallet/posisi asing menahan kelanjutan. Batas rugi merupakan pemicu
penghentian, bukan jaminan fill saat gap/latency/putus koneksi.

Mesin memakai candle selesai 5m: breakout 20 candle dengan volume, pullback tren
EMA20/50, atau pantulan range. Sinyal bertentangan menjadi menunggu; SL 2×ATR14,
TP dasar 3×ATR14, disesuaikan biaya hingga maksimum 6×ATR14 dan dibulatkan
menurut tick. Sizing mengecek biaya taker, margin, tier,
notional akun dan jarak likuidasi. Entry ditahan 10 menit sebelum funding;
posisi ditutup 2 menit sebelumnya, setelah 30 menit, atau saat sinyal berlawanan.
Pengujian aturan ini bukan evaluasi profit; tidak ada optimasi parameter atau
pembelajaran otomatis selama sesi. Sesi 240 menit tetap dapat selesai tanpa entry.

Restart aplikasi **tidak** mengaktifkan transaksi. Catatan sesi terakhir dibaca
sebagai riwayat; posisi terkini masih perlu GET. **Pulihkan & tutup sesi lama**
merekonsiliasi ID milik sesi dan menyelesaikan penutupan, tanpa entry baru.
Tutup jendela saat sesi aktif meminta Stop dan menunggu selesai. Kill/crash/
mati listrik dapat menyisakan posisi; hanya SL/TP yang sudah diterima Binance
yang tetap bekerja di exchange. Koneksi gagal menahan entry. Sesi tanpa intent
mencoba pulih hingga deadline sesi asli; sesi dengan intent/posisi meminta penutupan
saat snapshot segar tersedia. Ketidakpastian order yang tidak terpecahkan
memerlukan pemeriksaan akun; program tidak membuat ID baru untuk mengulang entry.

`--finish-empty-session` hanya menyelesaikan catatan lokal sesi yang dihentikan
atau kedaluwarsa dan **belum pernah membuat intent/transaksi**. Perintah ini
mengunci state, memverifikasi binding akun, lalu memerlukan dua snapshot GET
yang segar, tanpa posisi/order/algo dan wallet yang cocok. Jika ada exposure,
riwayat transaksi, perubahan wallet atau data tidak valid, perintah gagal tanpa
menandai sesi selesai. Tidak membuat transport live atau mengirim order.

Data privat berada di `~/.local/share/binance-desk/live` atau `XDG_DATA_HOME`:
`session.json`, outbox per intent, dan arsip `*-ledger.json` berisi strategi,
batas sesi, rencana, alasan serta hasil. Direktori 0700/file 0600, tanpa key/secret.
Lock global folder melarang dua jendela mengeksekusi sesi bersamaan. State terikat
hash satu API key; pergantian key/akun belum memiliki alur migrasi otomatis.
Jangan menghapus/mengedit state aktif untuk melewati batas atau pemulihan.

### Preflight dan kalkulator opsional

**Baca dari KWallet** memuat entry aplikasi yang telah disimpan. Kredensial yang
dimasukkan pada **Akun Futures** juga tersedia selama sesi untuk tombol
**Perbarui data**. Preflight hanya berjalan ketika diminta; umur snapshot terlihat
dan kalkulasi dinonaktifkan setelah 30 detik. Data beberapa endpoint dibaca
berurutan, sehingga preflight bukan rekonsiliasi posisi yang atomik.

Isi budget margin dan risiko secara terpisah. Misalnya leverage 10× tidak
berarti kerugian dibatasi sepersepuluh notional. Kalkulator membatasi jumlah
berdasarkan margin, jarak stop, fee taker akun, cadangan funding, notional,
filter instrumen dan bracket. Slippage model 1 bp; buffer stop dari estimasi
likuidasi 10 bps. Kedua angka tersebut asumsi kalkulator, bukan batas risiko
akun yang telah disepakati. Estimasi likuidasi isolated dapat berubah akibat
biaya, funding, perubahan tier/margin atau gap harga.

Metadata leverage akun dapat melebihi batas 125 yang tertulis pada dokumentasi
request Binance. Parser membaca metadata tersebut, sedangkan form/perintah
tetap dibatasi 1–125 dan tier terkait. Dukungan awal hanya Futures USDT standar,
One-way, Single-asset, Isolated; Hedge dan Multi-assets menjadi blocker.
Portfolio Margin/COIN-M belum didukung. `canTrade` bukan bukti izin TRADE key.

[Kontrak konektor dan pengendali otomatis](src/futures/README.md),
[lingkup putaran](../docs/AUTONOMOUS_SESSION_01.md).

### Jalankan mesin paper (fitur 0.4.0)

Buka **Mesin paper → Mulai sesi 60 menit**, atau:

```bash
binance-desk --paper-run
```

- BTCUSDT USD-M, candle selesai 5m, EMA20/50 + penembusan high/low bar sebelumnya;
  SL 2×ATR14 dan TP 3×ATR14. Satu posisi virtual. Riwayat hanya menyiapkan indikator;
  entry menunggu candle yang selesai setelah sesi dimulai. Belum bukti profit.
- Modal **1000 USDT virtual**, anggaran risiko/entry 2.5, notional maksimum 250,
  batas rugi harian 10/total 30. Ini parameter simulator, bukan alokasi akun.
  Gap harga dapat membuat kerugian melewati anggaran. Tidak ada leverage simulasi.
- Fee model 6 bps/sisi, adverse slippage sampai 1 bp mengikuti tick. Fill penuh
  virtual langsung pada bid/ask; likuiditas, partial fill dan latency order belum
  direplikasi. Harga di antara polling dapat melewati stop tanpa terekam.
- Entry ditahan 30 menit sebelum funding dan posisi ditutup 5 menit sebelumnya.
  Gap data melewati funding ditandai P&L tidak lengkap. Margin/likuidasi/funding
  penuh belum dimodelkan; parameter leverage akun tidak dipakai.
- **Hentikan & tutup virtual** menghentikan entry dan menutup pada quote segar
  berikutnya. Sesi berakhir otomatis setelah 60 menit. Gangguan ≥8 detik menahan
  entry dan mengakhiri sesi; saat masih terbuka, penutupan menunggu data segar.
  Tutup jendela mengakhiri worker dan mencatat sesi terputus, tanpa resume otomatis.
  Kill/crash/listrik mati dapat meninggalkan snapshot terakhir; periksa waktunya.
- Folder `~/Workspace/outputs/binance-paper-<waktu>-<pid>` memuat `journal.jsonl`
  (event/state engine), `decisions.jsonl` (alasan strategi) dan `status.json`.
  Isinya data publik/virtual, tanpa kredensial/saldo akun. Jangan mengubah jurnal
  yang sedang berjalan. Halaman **Jurnal demo** tetap khusus fixture sintetis;
  jurnal paper dibuka melalui **Buka folder sesi** pada halaman Mesin paper.

```bash
# Probe feed publik sampai tiga pembaruan, lalu stop flat (tanpa UI/order nyata):
binance-desk --probe-paper ~/Workspace/outputs/paper-probe
# Lihat snapshot tersimpan; tidak meneruskan trading/simulasi:
binance-desk --offline --paper-report /path/ke/sesi/status.json
```

Sumber protokol publik: [Binance market data](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data).
Kline diambil setelah waktu server yang menjadi batas candle selesai; cache
tidak pernah mempromosikan candle yang sedang terbentuk menjadi candle final.

### Hubungkan akun Futures

1. Buka **Akun Futures**, atau jalankan `binance-desk --page account`.
2. Masukkan pasangan API key/secret **HMAC** langsung ke form tersamarkan.
   Jangan menaruhnya di chat, terminal, `.env`, atau file proyek. Key RSA/Ed25519
   dan blob DPAPI dari Windows belum didukung.
3. Opsional: centang **Simpan di KWallet**. Tanpa centang ini, kredensial hanya
   dipakai selama sesi. Secret Service Linux/KWallet tersedia pada Fedora ini;
   KWallet yang terkunci harus dibuka melalui dialog sistem. Gagal menyimpan
   tidak diam-diam beralih ke penyimpanan file atau sesi.
4. Klik **Hubungkan akun**. Status **TERHUBUNG · BACA SAJA** baru tampil sesudah
   keempat pembacaan akun berhasil. Penyimpanan kredensial sendiri bukan bukti
   autentikasi berhasil. Aplikasi membaca ulang sekitar 10 detik setelah batch.

Saat mesin live aktif, hentikan sesi sebelum mengganti/memutus kredensial.
**Putuskan** membatalkan pembacaan dan menghapus data/credential worker dari
memori sesi; **Hapus yang tersimpan** juga menghapus entry milik aplikasi di
KWallet. Menutup jendela mengakhiri sesi. Jika ada entry KWallet, aplikasi mencoba
memakainya saat dibuka lagi. Tombol putus tidak memengaruhi jendela lain yang
telah terhubung. Kotak **Samarkan nilai akun** menyamarkan angka di UI.

Worker akun membatasi signed GET ke `/fapi/v3/account`, `/fapi/v3/positionRisk`,
`/fapi/v1/accountConfig`, dan `/fapi/v1/symbolConfig` pada `fapi.binance.com`.
Waktu server disinkronkan sebelum batch. Host/TLS, metode, path dan query
divalidasi; API key hanya ada di header sensitif, secret tidak dikirim. DNS
fallback tidak menerima kredensial. Response/error/URL bertanda tangan tidak
dicetak. Tidak ada endpoint order, perubahan leverage/margin, withdrawal,
transfer atau pembuatan user-data-stream.

Pembacaan empat endpoint berurutan **bukan snapshot atomik**. Polling ini untuk
observasi; belum event stream/reconciliation yang diperlukan eksekusi. Data
lebih dari 30 detik, atau batch gagal, ditandai tidak segar; gangguan jaringan
mempertahankan data terakhir dengan label jelas. Auth gagal, perubahan koneksi
atau disconnect menghapus data lama. Cooldown 429/418 mengikuti Retry-After dan
tidak dihapus oleh reconnect. Leverage yang ditampilkan milik akun, bukan
rekomendasi atau pengaturan modal bot. `canTrade` bukan pemeriksaan izin API.

Total single-asset mengikuti satuan USDT Binance; multi-assets memakai nilai
setara USD. Tabel per aset tetap terpisah. Harga likuidasi nol dari Binance
ditampilkan **Tidak tersedia**, bukan dihitung sendiri. Fee, open orders,
funding, realized P&L dan riwayat transaksi sesi ditangani supervisor pada
**Eksekusi Futures**, bukan tabel akun ini. Tidak ada ekspor kredensial/response
akun; screenshot audit `--capture` menyamarkan nilai dan rincian posisi akun.

### Transport dan batas feed

Worker Rust terpisah dari render UI. Feed tampilan memakai GET `time` dan `klines`;
paper juga memakai `exchangeInfo`, `ticker/bookTicker`, `premiumIndex` di Futures.
Transport hanya ke `api.binance.com` atau `fapi.binance.com`; tidak memuat kredensial,
cookie atau proxy lingkungan, tidak mengikuti redirect. Kegagalan koneksi DNS
sistem memicu lookup `dns.google` untuk host Binance tersebut. Alamat nonpublik
ditolak, TTL dihormati, tiap koneksi tetap memverifikasi TLS/hostname Binance.
Pengaturan jaringan OS tidak diubah. Worker mengikuti Retry-After HTTP 429/418,
retry eksponensial 2–30 detik; mengganti tampilan tidak menghapus cooldown server.
Respons publik dibatasi 1 MiB; khusus exchangeInfo 4 MiB karena metadata seluruh
simbol Futures sudah melebihi 1 MiB. Paper polling sekitar 2 detik ditambah waktu
permintaan; mempertahankan cooldown server, timeout batch 15 detik.

Ini feed **observasi candle lewat polling**, belum WebSocket, order book atau
feed eksekusi HFT. Pengambilan BTC/ETH berlangsung berurutan, bukan satu snapshot
atomik bursa. Riwayat terbatas 240 candle dan berada di memori; belum arsip pasar.
Data tampilan memakai f64; sizing/risiko engine tetap memakai desimal existing.
Pembaruan REST bukan timestamp setiap transaksi; perubahan harga di antara poll
tidak direkam. Satu simbol gagal membuat batch ditandai gagal agar data tidak
terlihat lengkap ketika sebagian koneksi bermasalah.

## Build dan instalasi user-local

```bash
cd ~/Workspace/projects/binance-sniper/desktop-rs
source ~/.cargo/env
rust-workbench check
bash scripts/install.sh
binance-desk
```

Rust 1.98.1, dependency langsung terpin dan transitif di `Cargo.lock`. Header
Wayland, X11, xkbcommon, EGL/OpenGL yang diperlukan sudah tersedia pada Fedora
saat implementasi. Pengujian di distro lain/packaging RPM/Flatpak belum dilakukan.

Installer menaruh:

| Hasil | Lokasi |
|---|---|
| Binary native | `~/.local/bin/binance-desk` |
| Launcher menu | `~/.local/share/applications/io.yuri.BinanceDesk.desktop` |
| Ikon | `~/.local/share/icons/hicolor/scalable/apps/io.yuri.BinanceDesk.svg` |

Lokasi data menu/ikon mengikuti `XDG_DATA_HOME`. Instalasi tidak membutuhkan root,
tidak membuat startup otomatis atau layanan background permanen. Saat live aktif,
tutup jendela meminta penutupan sesi sebelum keluar. Semua file instalasi dapat dihapus untuk
uninstall; ekspor dan pengaturan tidak dihapus oleh installer.

## Verifikasi

```bash
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
desktop-file-validate ~/.local/share/applications/io.yuri.BinanceDesk.desktop
```

Tes meliputi playback yang memetakan state engine, export/replay jurnal,
validasi/persistensi pengaturan dan klik pointer pada widget nyata egui, termasuk
navigasi, transport playback, ekspor dan simpan pengaturan. Parser candle menolak
gap, duplikat, OHLCV invalid, dan interval salah. Worker diuji dengan sumber uji
yang terputus/pulih, pembatalan permintaan saat pilihan berubah, serta pause.
Kontrol pasar/zoom dan status stale ikut diuji. UI juga dirender
pada ukuran minimum 1024×720. Framebuffer native dapat diambil tanpa menangkap
jendela aplikasi lain:

```bash
binance-desk --page simulation --step 35 \
  --offline \
  --capture ~/Workspace/outputs/binance-desk-preview.png
```

`--capture` membuka jendela nyata, menyimpan PNG frame OpenGL lalu keluar;
tidak menimpa file yang sudah ada. Pilihan `--page`: workspace/account/futures/paper/simulation/journal/settings.
Ukuran bawaan 1400×900, dapat diubah lewat `--width` dan `--height`.

```bash
# Capture data nyata setelah memberi waktu koneksi awal:
binance-desk --market futures --timeframe 5m --capture-wait 25 \
  --capture ~/Workspace/outputs/binance-market-preview.png
# Probe publik kedua pasar/empat timeframe + jeda/resume; folder harus baru:
binance-desk --probe-market ~/Workspace/outputs/binance-feed-probe
```

`--offline` mematikan networking untuk sesi tersebut. `--capture-wait` menunda
tangkapan layar (maksimum 60 detik), bukan menjamin koneksi berhasil. Probe CLI
gagal jika data segar tidak didapat dalam 45 detik per pilihan; ia memakai worker
yang sama dengan UI. Jendela observasi tidak membuat jurnal harga; sesi paper
membuat jurnalnya sendiri ketika dimulai.

Tes akun mencakup vektor HMAC publik RFC 4231, GET allowlist, redaksi error,
desimal dan validasi payload, Hedge/One-way, stale/recovery/disconnect, KWallet
kosong serta input password dan pengosongan buffer. Tes jaringan publik sengaja
dipisahkan dari suite offline:

```bash
cargo test --locked unsigned_account_transport -- --ignored
binance-desk --test-vault     # entry dummy sementara, lalu hapus
binance-desk --check-vault    # hanya status entry, tanpa nilai rahasia
binance-desk --probe-account  # dua batch akun, output status saja
```

Probe akun hanya memakai entry yang disimpan lewat UI; kredensial sesi tidak
tersedia pada proses lain. Ketiadaan key mengembalikan error eksplisit, tidak
diganti akun mock. Bukti autentikasi live tetap terpisah dari hasil tes sintetis.

Mulai 0.3.1, stderr mencatat **perubahan status koneksi saja** dan konfirmasi dua
snapshot sukses. Tidak ada key/secret, signed URL, isi respons, saldo, simbol/
jumlah posisi, atau data akun dalam diagnosis. Jika diluncurkan melalui unit
user sementara, status dapat dibaca dari journal unit tersebut. Ini memungkinkan
verifikasi koneksi sesi-only tanpa memindahkan kredensial ke penyimpanan lain.

[Protokol putaran](../docs/DESKTOP_ROUND_01.md).
[Putaran integrasi pasar](../docs/MARKET_DESKTOP_ROUND_01.md).
[Putaran akun Futures](../docs/ACCOUNT_DESKTOP_ROUND_01.md).
[Perbaikan parser](../docs/ACCOUNT_PARSER_FIX_01.md).
[Putaran forward paper](../docs/FORWARD_PAPER_ROUND_01.md).
[Putaran sesi otomatis](../docs/AUTONOMOUS_SESSION_01.md).
