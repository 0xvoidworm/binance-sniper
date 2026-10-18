# Known Gaps

## 0.6.11 — patch stale/clock, acceptance live masih diperlukan

- [Regresi77menit](docs/STALE_CLOCK_11.md) sudah dipatch dan dipasang: stale
  flat menjeda entry, clock tidak dibaca lagi setelah quote. Future/schema/
  replay/unknown tetap terminal; guard tidak dilemahkan. Lihat STATUS untuk
  rincian verifikasi dan aplikasi terpasang.
- Uji deterministik dan probe GET flat tidak membuktikan recovery posisi nyata
  0.6.11, stabilitas24jam atau profit. Durasi tetap batas maksimum, bukan janji
  tak akan dihentikan guard. Dua episode recovery tidak menjamin koneksi sehat.
- Clock memakai waktu Binance + elapsed monotonic dalam lease5detik; bukan
  jam dinding lokal. Ketidakpastian jaringan/latensi server tetap dapat menolak
  snapshot; tidak ada toleransi future tambahan atau pemalsuan umur data.
- Strategi tidak belajar otomatis dari panjang sesi. Hermes/provider tetap
  ditunda; tidak ada pemasangan model, biaya model atau pengiriman data baru.

## Riwayat: 0.6.10 — pemulihan terbatas

- [Patch pemulihan](docs/TRANSPORT_RECOVERY_10.md) memisahkan close trade dari
  Stop sesi untuk error transport bertipe. Maksimal dua episode dengan
  intent, lalu60detik data sehat setelah flat/accounting. Bukan janji sesi
  selalu mencapai durasi, tidak menahan exposure dengan data basi.
- Snapshot/order evidence basi, schema/integritas/auth/rate-limit/proteksi
  tak pasti, Stop/deadline dan batas risiko tetap terminal. Auto-recovery
  bukan blanket retry. Penutupan bergantung koneksi/bukti baru, SL/TP dapat
  terkena gap/slippage. Reentry tambahan juga dapat menambah biaya.
- Tetap belum ada bukti edge/profit atau acceptance24jam. Uji virtual/GET
  flat tidak membuktikan recovery posisi real. Posisi order asing seluruh
  simbol belum diperiksa oleh query order khususBTCUSDT; jangan menjalankan
  bot/order lain pada akun selama sesi.
- Episode adalah counter, bukan full health/decision/price-path timeline.
  Paket lama tetap unknown/absent untuk data yang tidak direkam. Hermes
  ditunda; tidak ada provider/model/runner, auto-apply atau data egress baru.
- Rollback0.6.9 setelah state0.6.10 dibuat memerlukan pemeriksaan metadata/hash;
  jangan overwrite ledger/state baru, atau mengganti binary saat live.
  Hasil pemasangan/probe aktual ada di STATUS/HANDOFF dan laporan putaran.

## Riwayat: 0.6.9 — reliability dan diagnosis, belum acceptance 24 jam

- [Investigasi sesi 78 menit](docs/ACCOUNT_SNAPSHOT_DIAGNOSTICS_09.md) membuktikan
  snapshot account5407ms setelah GET symbolConfig4310ms pada produksi0.6.6.
  Enam GET akun kini serentak; REST belum atomik dan satu endpoint/outage
  terlalu lambat tetap dapat menghentikan sesi saat exposure aktif. Guard
  tetap5detik; tidak mengklaim bug jaringan/receipt exchange seluruhnya selesai.
- Counter sesi baru bertipe dan versi pembuat tersedia, tetapi bukan full
  decision/health/price-path history. Sesi lama tetap unknown, bukan nol.
  Ekspor final hanya latest-session; harga fill mentah tidak dikarang.
  Validator kompatibel dengan paket0.6.8, tidak mengeksekusi proposal.
- Hermes/model ditunda oleh pengguna; bukan penghalang rilis inti. Tidak ada
  provider/runner/pilot/auto-apply baru. Keputusan model/biaya/egress terpisah.
- Tes fixture/GET flat tidak membuktikan operasi posisi aktif0.6.9, ketahanan
  24jam atau profit. Tidak ada perubahan strategi/ATR/SL/TP/risk agar memaksa
  order. Status pemasangan dan hasil probe ada di STATUS/HANDOFF.
- Rollback binary lama setelah sesi0.6.9 harus memeriksa kompatibilitas
  metadata/hash. Jangan restore state lama menimpa order/fill terbaru.

## Riwayat: kandidat0.6.8 P1+P2, belum dipasang pada putaran P2

- Exporter dan validator lokal tersedia; Hermes runtime/runner/egress/meter,
  vendor pin, provider/model/budget/izin data keluar dan pilot belum selesai.
  Tidak ada model run, proposal yang diterima atau auto-apply. Ketersediaan
  Podman tidak dianggap pengujian batas OS/jaringan Hermes.
- Profil paket hanya final latest-session; incomplete/active ditolak. Full
  decision/health/price path/raw fills/harga fill/build sumber tidak tersedia
  dari arsip; ditandai missing. Legacy exit tidak dikarang. Checksums bukan
  signature exchange; perhitungan diverifikasi dari arsip lokal, tanpa GETulang.
- Validator mencocokkan scalar facts dan schema; narasi bebas/inference tetap
  butuh review. Usage JSON bukan meter biaya; belum ada izin model berbayar.
- Kandidat source/release0.6.8 tidak sama dengan proses0.6.6. Acceptance posisi
  aktif versi baru/ketahanan24jam/profit tetap belum terbukti. Jangan memaksa
  entry dengan melonggarkan risiko/ATR/TP. [Bukti dan batas](docs/EVALUATION_BRIDGE_P2_08.md).
- Probe6menit sehat, tetapi setup pertama gagalrequestresolverpublik dan
  keduapulih. Detail transport tidak cukup untuk diagnosis eksklusif; tidak
  dianggap sudah bebas gangguan. Candle terkonfirmasi pun dapat tiba terlambat
  untuk15detik entry. Readiness harus dicek ulang sebelum deploy/aktivasi.

## Riwayat: kandidat0.6.7 P1, belum dipasang

- Patch source/build tersedia dan tes lolos; runtime terpasang masih0.6.6.
  Jalur live posisi aktif0.6.7, latency/proteksi nyata dan kestabilan24jam belum
  diterima. Uji deadline virtual atau probe GET flat bukan penggantinya.
- History tidak menghalangi snapshot, tetapi tetap perlu dua read terkonfirmasi
  dan batas15detik. GET lambat/revisi tetap bisa melewatkan sinyal; error history
  tetap memicu pause/stop sesuai exposure. Snapshot REST belum atomik, batas
  5detik tetap dan metadata kedaluwarsa tetap ditolak.
- Legacy exit reason tidak diisi dengan dugaan. Sebagian penutupan internal
  flow memakai `lifecycle_close`, bukan sebab yang lebih spesifik. Data jalur
  harga, ekspor aman/schema dan evaluasi strategi/Hermes P2–P4 masih terbuka.
- Pembacaan legacy kompatibel, tetapi rollback binary lama setelah ada field/
  hash metadata baru memerlukan pemeriksaan; jangan restore state lama secara
  buta. [Hasil, pengujian dan batas P1](docs/RELIABILITY_P1_07.md).

## Riwayat: audit sesi300menit dan rancangan Hermes — 20 September 2026

- Sesi0.6.6 terbaru berhasil3trade, tetapi berakhir194menit5,332detik dari300
  karena umur bukti order5069ms>5000ms. Perbaikan input sebelumnya belum
  menyelesaikan keandalan posisi aktif; pipeline snapshot perlu direproduksi
  dan dipatch tanpa melonggarkan guard. Belum ada patch pada putaran desain.
- Net−0.34155740USDT setelah fee0.48235739; tiga trade tidak membuktikan edge.
  Data jalur harga dan alasan exit terstruktur belum cukup untuk menerima
  perubahan TP/holding. Watching setelah settlement saat Stop aktif masih
  perlu diperbaiki; tidak ditemukan reentry akibat pesan tersebut pada audit.
- Hermes baru masuk rancangan. Export schema/validator/runner/isolasi belum
  diimplementasikan, pin vendor/provider/model/budget belum dipilih. Ekspor
  audit existing belum otomatis boleh dikirim ke provider. Memori agent bukan
  ledger/arsip bukti; replay penuh ensemble live belum tersedia.
- Kelulusan fixture/GET/pilot evaluator tidak membuktikan profit, seluruh
  lifecycle produksi atau kesiapan24jam. Tidak ada perluasan risiko/ATR/TP/
  leverage sebagai bagian rancangan. [Rencana dan syarat uji](docs/HERMES_INTEGRATION_PLAN_07.md),
  [bukti sesi](../../outputs/binance-session-review-300m-20260920-ip2XD9/LAPORAN.md).

## Riwayat tambahan 0.6.6 sebelum sesi300menit

- Candle baru dikonfirmasi dua kali dan quote memakai depth5; ini tidak
  menjamin tidak ada revisi susulan, quote stale, kegagalan DNS atau snapshot
  account usang. REST tetap tidak atomik; grace/konfirmasi memakai sebagian
  jendela sinyal15detik, tidak memperpanjangnya. Terlambat tetap tanpa entry.
- Probe600detik0.6.6 PASS (168snapshot/0error/0rewarm,2candle/0sinyal).
  Konfirmasi12486ms pada candle pertama dekat batas15s; jaringan lebih lambat
  atau calon yang terus berubah masih dapat melewatkan peluang. Dua candle
  bersih tidak membuktikan seluruh39skip pada10jam lama sudah teratasi.
- Dua setup GET ATR0.1757% layak Long/Short, tetapi bukan sinyal dan bukan
  hasil order/profit. Input10jam sebelumnya memiliki0transaksi:59ATR rendah,
  39candle recovery terlewat, satu Short tertahan capTP. Snapshot lengkap
  Short lama tidak tersedia, sehingga tidak dapat dibuktikan false rejection.
- Oracle15.000kombinasi mendukung konsistensi planner, bukan edge strategi.
  Penambahan jejak ekonomi memungkinkan replay sizing berikutnya; tidak
  merekam seluruh OHLCV/status akun atau melatih strategi secara otomatis.
- Live posisi/partial/cancel/restart0.6.6 masih perlu acceptance produksi
  pengguna. Tes fixture dan probe GET akun flat bukan pengganti. Tidak ada
  jaminan jumlah trade, profit, atau kesiapan berjalan24jam tanpa pengawasan.
- Tidak menaikkan modal/risiko/funding/leverage/capTP atau menurunkan ATR demi
  mengejar entry. Kondisi snapshot dapat berubah setelah preview.

[Hasil dan batas putaran0.6.6](docs/INPUT_EXECUTION_PLAN_06.md).

## Riwayat tambahan 0.6.5

- Bug frame menua akibat query order sesudah capture direproduksi dan dipatch;
  field/latency spesifik yang menyebabkan Stop sesi lama tidak tersimpan.
  Guard5detik tetap: jaringan atau quote yang benar-benar usang masih dapat
  menghentikan sesi. REST belum atomik dan belum user-data stream.
- Sepuluh tes baru melengkapi129 tes lulus, tetapi GET nyata saat akun flat
  tidak membuktikan jalur batch proteksi aktif, fallback receipt mutation,
  cancel nyata, partial fill nyata atau ketahanan24jam pada0.6.5.
- Konfirmasi GET setelah respons market tanpa `cumQuote` mengurangi satu
  penundaan sampai poll berikut; tidak menghilangkan celah entry→acceptance SL.
  Timeout/NotFound tetap Uncertain, bukan izin resend mutation.
- Strategi `ensemble-2-cost-aware` dan risiko tetap, belum terbukti profit.
  Satu trade negatif pengguna0.6.4 bukan evaluasi statistik strategi.
- Probe600detik0.6.5 mendapat168snapshot sehat tetapi dua penolakan revisi
  volume candle; rewarm berhasil, exit1 tetap. Tidak ada penolakan stale pada
  jendela ini; bukan bukti data selalu segar atau sesi berposisi pasti bertahan.

[Hasil dan batas patch](docs/FRESHNESS_RECONCILIATION_05.md).

## Riwayat tambahan 0.6.4

- Kelayakan biaya kini diperiksa sebelum aktivasi/entry, tetapi contoh ATR
  hipotetis bukan sinyal. MinimumATR0.10% tetap dapat menahan seluruh sesi.
- TP cost-aware hingga6ATR adalah perubahan strategi belum tervalidasi profit;
  hit rate dapat turun dan timeout posisi30menit tetap. Tidak ada pembelajaran
  otomatis. Uji empat jam hanya sintetis, bukan acceptance live.
- Pemeriksaan setup aktual kedua ditolak harga usang. Polling REST dapat gagal
  sementara; sumber spesifik ketidakselarasan timestamp tidak dipastikan pada
  putaran ini dan penjaga usia data tidak dilonggarkan.
- Batas dukungan eksekusi di bawah tetap berlaku. [Hasil0.6.4](docs/FEASIBILITY_LOGIC_04.md).

## Sesi Futures otomatis sejak 0.6.0 — batas yang masih berlaku

- Integrasi strategi, sizing, supervisor, order, recovery, ledger dan UI aktivasi
  tersedia. Sesi pengguna19September membuktikan satu siklus entry/SL/TP/exit
  serta ledger produksi, net−0.39563580USDT. Ini memperbarui kekurangan bukti
  putaran0.6.0, bukan bukti profit, seluruh cabang order atau trading24jam.
  Pengujian patch oleh agen tetap exchange tiruan dan Binance GET-only.
- Satu posisi BTCUSDT USD-M standar, One-way/Single-asset/Isolated, komisi USDT.
  BNB fee, Hedge, Portfolio Margin, Multi-assets, COIN-M, multi-simbol dan
  `notionalCoef !=1` belum didukung. Key HMAC saja, tidak ada testnet execution.
- REST berurutan, bukan snapshot atomik atau user-data stream. Snapshot/quote
  maksimal5s; polling2s+durasi permintaan. Tier/filter/fee cached≤60s; perubahan
  aturan exchange dapat menolak order. Ada celah antara entry dan acceptance SL.
  Gap, disconnect, liquidation atau slippage dapat melewati budget/rugi model.
- Setelah fill, supervisor mengecek harga fill dan liquidation price exchange
  terhadap stop jika tersedia. Tidak merekonstruksi isolated wallet sendiri.
  Harga likuidasi nol berarti tidak tersedia, bukan bukti proteksi tanpa risiko.
- Sesi mengasumsikan akun didedikasikan: posisi non-BTC, order BTC asing dan
  perubahan wallet menahan/menutup sesi bot. Order aktif simbol lain belum
  diinventarisasi; perubahan eksternal yang saling mengimbangi dapat luput.
- Pending/tidak lengkapnya fill atau income menahan entry berikutnya. Net
  memakai fill actual, fee USDT dan funding; toleransi wallet1e-6. Tidak ada
  konversi fee aset lain. Daily net dicatat pada hari UTC penutupan trade.
- Ketidakpastian POST/GET permanen tidak diselesaikan dengan resend. Maks3
  exit residual hanya setelah upaya sebelumnya terminal/reject; sesudahnya
  perlu pemeriksaan manual. Crash sebelum ACK SL dapat meninggalkan exposure.
  Tutup normal menunggu Stop; kill/listrik mati tidak menjamin posisi flat.
- Folder state satu writer, terikat hash API key. Rotasi key/migrasi akun belum
  tersedia. Menghapus/memindahkan state dapat merusak batas/recovery; tidak ada
  retensi arsip otomatis atau koordinasi dengan aplikasi/perangkat lain.
- Capture native memakai X11; masalah screenshot Wayland putaran lama belum
  diselesaikan. Packaging RPM/Flatpak dan pengujian lintas distro belum dilakukan.

[Panduan operasi](desktop-rs/README.md), [kontrak](desktop-rs/src/futures/README.md),
[bukti](../../outputs/binance-autonomous-session01-20260916/DECISION.md).

## Riwayat: Futures production components 0.5.0

- Model margin, konektor produksi, outbox dan pengendali keputusan tersedia.
  **Live end-to-end belum tersedia**: supervisor, recovery sesi, ledger nyata
  serta batas rugi belum terhubung; UI tidak memiliki aktivasi uang nyata.
  Otonomi produk setelah satu aktivasi tetap menjadi target, tanpa klik per order.
- Kontrak mutasi diuji melalui exchange lokal dalam memori. Binance hanya
  diakses GET-only, sehingga izin TRADE, acceptance order/margin/leverage,
  real partial fill/proteksi/latency belum teruji. `canTrade` bukan bukti izin key.
- Bracket/fee aktual bukan semua aturan akun. Command leverage≤125 meskipun
  metadata tier bisa lebih tinggi; maxNotional hasil perubahan leverage dan
  batas akun lainnya masih harus diperiksa saat integrasi dispatch.
- Perhitungan isolated memakai slippage/funding reserve asumsi, belum ledger
  funding aktual. Gap atau perubahan tier/fee/margin dapat melewati risiko model.
  `notionalCoef !=1`, Portfolio Margin, COIN-M, Hedge, Multi-assets belum didukung.
  Tidak ada dukungan HFT atau klaim profit baru.
- Readiness adalah GET berurutan, refresh manual dan stale30s. Tidak cukup
  untuk pengambilan keputusan live. Controller mensyaratkan posisi/quote≤5s
  serta caller yang terus memperbarui status order/proteksi.
- Outbox mengunci satu path, belum satu akun lintas seluruh jendela/proses.
  Intent/sesi belum memiliki persistence/recovery lengkap. Residual setelah
  exit terminal/reject membutuhkan rekonsiliasi, belum loop pemulihan otomatis.
  Maks1024 record/4MiB, belum arsip otomatis. Checksum bukan autentikasi file.
- P&L/fill/fee/funding dari child Algo harus diselesaikan sebelum entry baru;
  controller mengembalikan `NeedsAccounting`, tidak menganggap hasil final.
- Capture framebuffer Wayland tidak selesai pada sesi verifikasi ini. Jalur
  X11 native menghasilkan PNG; penyebab Wayland belum ditentukan. UI normal
  tetap berjalan. Ini tidak membuktikan masalah order atau autentikasi akun.

[Kontrak integrasi](desktop-rs/src/futures/README.md),
[hasil putaran](../../outputs/binance-futures-execution-round01-20260916/DECISION.md).

## Riwayat simulator 0.4.0 — harga nyata, uang virtual

- Simulator kini memakai public bid/ask/candle Futures dan mengambil keputusan
  EMA/ATR pada candle baru. Belum ada evaluasi performa atau strategi yang terbukti
  menghasilkan profit. Tidak mencampurkan strategi/menaikkan risiko mengejar target.
- Modal1000, risiko2.5, notional250, fee6bps/sisi dan slippage≤1bp adalah asumsi
  virtual. Dana akun pengguna tidak digunakan; fee akun/leverage belum dipetakan.
- Full fill virtual langsung, tanpa order latency/depth/partial-fill nyata.
  REST polling dapat melewatkan sentuhan stop/TP di antara quote; gap dapat membuat
  rugi melampaui anggaran. Belum WebSocket/HFT atau produksi order/reconciliation.
- Margin/leverage/likuidasi belum dimodelkan. Entry/exit menghindari funding;
  posisi yang melewati jadwal saat data putus ditandai P&L tidak lengkap.
- Stop menunggu quote segar bila posisi terbuka. Failure dapat menyisakan posisi
  virtual terbuka dengan hasil tidak final. Tidak ada resume atau service autostart.
  Kill/crash bisa meninggalkan snapshot bertuliskan Running; periksa timestamp.
- Jurnal paper terpisah dari halaman Jurnal demo; gunakan tombol folder sesi
  untuk melihat JSONL. Laporan tersimpan tidak memperbarui otomatis.
- User sudah mengalokasikan dana khusus proyek. Batas rugi/angka leverage masih
  perlu dibedakan dari alokasi; riwayat yang menyebut alokasi belum ada sudah usang.

## Desktop native 0.3.1 — akun nyata berhasil dibaca

- Bug nama simbol Unicode telah diperbaiki; dua snapshot akun nyata berhasil
  dibaca pada worker UI. Hambatan credential/autentikasi yang dicatat pada 0.3.0
  di bawah sudah terlewati pada sesi ini.
- Ini verifikasi koneksi/parser, belum strategi, order, engine margin/funding/
  likuidasi atau profit. GET polling masih non-atomik dan belum ACCOUNT_UPDATE.
  Batas fitur 0.3.0 selain bug/credential tetap berlaku.
- Tes 897 nama berasal dari metadata publik; payload akun privat tidak disimpan.
  Diagnosis hanya endpoint/check dan status, tanpa nilai respons. Jendela lama
  tetap memakai parser lama sampai ditutup; gunakan judul/footer 0.3.1.

## Desktop native 0.3.0 — riwayat akun Futures

- Adapter/form akun selesai, tetapi autentikasi akun nyata **belum terverifikasi**:
  kredensial aplikasi belum tersedia pada KWallet Linux. Penyimpanan lama Windows
  memakai DPAPI, tidak otomatis kompatibel. Isi pair HMAC langsung di aplikasi.
- KWallet/Secret Service lokal telah diuji dengan entry dummy lalu dihapus.
  Backend wallet distro lain, key RSA/Ed25519, migrasi DPAPI dan Portfolio Margin/
  COIN-M/testnet tidak dicakup. Host akun tetap USD-M produksi, read-only.
- REST sekitar 10 detik, empat endpoint berurutan, bukan snapshot atomik atau
  ACCOUNT_UPDATE. Belum open orders, fee, funding, realized P&L atau histori.
  Nilai terakhir dapat tertinggal; UI menandai stale/error. Tidak ada arsip akun.
- Leverage/margin hanya dibaca, bukan dipilih atau dihitung oleh bot. Semua
  kebutuhan modal/risiko dan margin/funding/likuidasi engine masih terbuka.
  `canTrade` bukan verifikasi izin trading key. Tidak ada eksekusi nyata.
- Pemutusan koneksi berlaku per jendela. Entry KWallet akan dimuat lagi saat
  membuka aplikasi; gunakan Hapus yang tersimpan bila ingin menghilangkannya.
  Capture audit menyembunyikan data akun; screenshot manual OS tetap mengikuti
  tampilan normal, dengan opsi Samarkan nilai akun untuk menyembunyikan angka.
- Pasar publik, keterbatasan packaging/aksesibilitas dan engine di bawah tetap
  berlaku. Riwayat versi lama yang menyebut belum ada form/adapter akun tidak
  menggambarkan kemampuan 0.3.0.

## Desktop native 0.2.0 — riwayat feed publik

- Pasar publik BTC/ETH kini terhubung lewat REST polling dua detik; belum
  WebSocket, order book/tick stream, feed HFT atau input ke strategi/engine live.
  BTC/ETH diambil berurutan. Umur batch bukan umur setiap transaksi individual.
- Hanya 240 candle per simbol untuk satu pasar/timeframe aktif; belum arsip
  persisten, pagination sejarah atau pemulihan pilihan viewport setelah restart.
- DNS sistem gagal pada sesi Fedora ini; fallback DNS Google dengan TTL dan TLS
  Binance berhasil. Gangguan internet/layanan eksternal tetap bisa memutus feed;
  status stale/error/jeda menggantikan status aktif. Kedua simbol harus valid
  agar batch baru diterima. UI tidak mengaku akun kosong saat akun belum dibaca.
- Akun, fee aktual, posisi/order, API key, strategi dan live execution belum
  terhubung. Simulasi tetap sintetis. Pengguna sudah memilih Futures; besaran
  leverage dan mode margin/posisi belum ditetapkan. Dukungan leverage/margin/
  funding/likuidasi engine masih diperlukan. Belum ada bukti profit baru.
- Feed/chart memakai f64 untuk tampilan; inti sizing/order tetap desimal.
  Field read-only ini tidak boleh langsung dipakai sebagai nilai order nyata.
- UI/installer Fedora native terverifikasi; batas packaging/distro/aksesibilitas
  dan engine yang dicatat di bawah tetap berlaku. Bagian 0.1.0 adalah riwayat.

## Desktop native 0.1.0

- UI utama kini `desktop-rs`; belum tersambung feed, akun atau strategi Binance.
  Saldo/posisi akun tetap belum diketahui. Demo 50 event menggunakan fixture,
  bukan pasar aktual, paper forward test, atau backtest.
- Instalasi user-local Fedora KDE/Wayland/OpenGL terverifikasi. Belum paket
  RPM/Flatpak/AppImage, pengujian distro lain, screen-reader atau multi-monitor.
  X11 disertakan pada build tetapi sesi verifikasi memakai Wayland.
- Replay di memori; simpan sesi dengan Ekspor sesi. Tidak memulihkan cursor dari
  sesi sebelumnya secara otomatis. Settings skala/grid/kecepatan disimpan eksplisit.
- Jurnal/aturan/order yang tampil merupakan state demo engine. Tidak ada order
  sender, credential input, live risk settings atau kontrol trading akun.
- `gtk-launch` dari lingkungan tool tidak mempertahankan proses; app terpasang
  dapat dijalankan langsung, dan sesi ini dipertahankan oleh user unit sementara.
  Tidak memasang layanan/autostart permanen. Tutup jendela untuk berhenti.

## Inti Rust paper v1 — 16 September 2026

- Modul `engine-rs` baru diuji dengan event sintetis. Tidak ada koneksi Binance,
  signing, order dispatcher, strategi atau integrasi observer/UI. Tidak membuktikan
  profit maupun kelayakan execution live. Pilihan pasar/alokasi/risiko belum final.
- Schema laporan internal bukan payload Binance. Caller dipercaya memberi hasil
  rekonsiliasi; restart terhadap exchange, protection fills, cancel race, koreksi
  order terminal dan laporan out-of-order belum ditangani adapter nyata.
- Satu posisi/simbol one-way linear, tanpa leverage/margin/funding/likuidasi/order
  book. Filter/cost fixture belum berasal dari exchange/akun; fee laporan hanya
  nonnegatif dan sudah dinormalisasi ke quote currency.
- Exit ditolak/expired/canceled dengan residual exposure melatch halt; belum ada
  retry/recovery otomatis. Pembersihan proteksi memerlukan konfirmasi caller.
- CLI berbasis batch bukan daemon. Timer baru diperiksa saat event/Tick diterima.
  Rejected event tidak mengubah state sebelumnya; caller wajib berhenti/reconcile,
  seperti CLI `apply`, dan tidak boleh mengabaikan error lalu melanjutkan trading.
- Loss threshold meminta stop/exit, bukan batas rugi yang dijamin. Gap bisa melewati
  threshold. Daily boundary memakai equity terakhir sebelum hari UTC baru;
  biaya dicatat saat laporan diterima. Belum accounting statement Binance.
- Jurnal maksimal 16 MiB/20.000 record; tanpa rollover/migrasi/recovery tail otomatis.
  Checksum bukan proteksi terhadap penulisan ulang yang disengaja. Lock eksklusif
  membuat CLI status tidak bisa membaca jurnal saat writer masih terbuka.
- Sebelum adapter nyata, crash antara persist dan dispatch wajib diselesaikan
  dengan query seluruh ID stabil; deduplikasi bukan jaminan exactly-once exchange.
- Detail dan kontrak: [engine-rs/README.md](engine-rs/README.md). Bagian berikut
  mempertahankan keterbatasan komponen/riwayat sebelumnya.

## Observer kanal H4 yang baru diaktifkan

- Mode aktif adalah observasi pasar dan model OHLC close-bar, bukan demo/testnet exchange atau simulasi eksekusi intrabar. Entry next-open baru terlihat pada pencatatan close H4 berikutnya; stop intrabar tidak berwaktu presisi. Jangan menampilkan posisi model sebagai posisi akun saat ini.
- Kedua konfigurasi pernah ditolak; belum ada dasar profit. Aktivasi tidak mengubah hasil/gate lama. Belum ada trade pasar baru pada saat verifikasi awal.
- Funding live missing; fee/slippage hanya asumsi BASE, hasil parsial unitless. Lot/tick/min-notional, margin/likuidasi dan biaya akun tidak menjadi model eksekusi.
- WebSocket masih RECONNECTING. Public REST pulih dengan scoped DoH/TLS; private reader tetap AUTH_UNAVAILABLE lewat jalur sebelumnya. Tidak mengganti DNS Windows atau merutekan secret melalui resolver.
- Belum soak beberapa hari atau auto-start Windows. Observer aktif selama proses/komputer hidup. Gangguan lebih panjang daripada cache (400 bar) memerlukan perbaikan data eksplisit; tidak melewati gap. Revision latch saat proses berjalan akan diperiksa ulang terhadap input tersimpan saat restart.
- Restart replay membaca semua input sesi; volume jangka panjang belum dibatasi/diarsipkan. Jika identitas kode berubah, startup menolak sampai migrasi eksplisit; jangan hapus riwayat untuk memaksanya jalan.
- Endpoint lama `/api/state` dan `/market` tetap range reference; observer baru `/api/observer` dan `/observe`. Jangan mencampur statistik atau engine keduanya.

## Current MT5 evidence pipeline

- Original demo is installed and one visual compatibility run completed/imported. Positive sample is not validated profitability: 22 trades, reported history quality 83%, XAUUSD real ticks begin 2026-09-03 although test starts September 1, and DE40 did not participate despite enabled input (cause not established). No repeat, multi-year/walk-forward or live edge validation. See docs/DEMO_RUN_01.md.
- Vendor logs contain daily OCS randomization values 0-10; seed/mechanism not exposed in exported inputs. Tester INI saves relative Dates=1 without fixed date bounds, so do not reuse it blindly on another day. Broker timezone, frozen dataset/specification identity and verified costs remain missing; commission is zero in this report, swap -1.19 USD.
- Offline English HTML import, not automated tester/export/watcher. Unsupported deal types/cash adjustments/locales fail closed; one unit-harness format rejected.
- Artifact hashes/context are not execution attestation. Broker history/specifications/costs/randomizer are not independently frozen or verified. DB immutability does not protect against a malicious filesystem owner.
- No fabricated equity/candles/zones/MFE/MAE. PF/DD/trades are reported metrics; detailed trade pairing and large-archive pagination remain future work (input size/rows bounded).
- One legacy parser-check record is not a candidate result. FBS differs from source IC Markets. No purchase or live trading control.

## Retained Binance reference engine

- Public source rules have been implemented, but the source account was not independently audited. Binance instead of MT5/USDJPY, UTC instead of an unconfirmed broker timezone, weekend operation, and M5 OHLC are adaptations; this is not exact performance replication.
- The fixed 450-day exploratory replay is still negative. Existing data are already seen; no new holdout or future profitability is claimed. A slight positive single-symbol subgroup is not grounds for selection.
- No execution, sizing, actual fees, order-book fills, margin/liquidation, or exchange-managed stops. All positions in the model are hypothetical; account remains GET-only.
- No TP or trailing by design in this reference. Stop or session-end exits replace the old 4/8-bar limits. Holding can give back floating gains; there is no dynamic trend-health detector.
- Intrabar first-direction ambiguity remains unresolved and is reported separately. Durations count observed M5 bars, not precise tick-level holding times. Live funding is missing, so all-cost net stays null.
- Deterministic restart tests are not a multi-day soak. Recorded current/older sessions are labeled reconstruction; they never prove that a live order was sent or fill was available.
- Main M5 chart exposes up to 400 recent bars; other timeframes 80. Recorded session charts are bounded to one actual day, with post-exit hindsight labeled.

## Archived Phase 1–3 limitations

The following points describe retired engines/UI, not the active homepage. Their original data and negative results remain preserved.

- Binance market WebSocket connects but does not deliver payloads on this local network. Silent reconnect behavior is tested deterministically; a real WS recovery handover still requires a network where payloads arrive.
- The longest completed runtime observation is a short local soak, not a multi-day reliability run.
- Phase 2's 90-day baseline and Phase 3's eight alternatives over the preceding 360 days all remain negative after modeled costs. Phase 3 adds training-selected retrospective walk-forward (all three folds negative) and a held-out final 60 days (76 selected-candidate events, -0.227142R). This is evidence of failure under these assumptions, not an established edge. No true future/live profitability validation exists.
- Baseline scanner lifecycle remains in atomic JSON; research journal uses SQLite WAL. Their state/journaling are not one transaction. The new experimental shadow engine commits its signals, outcomes and engine state together and has synthetic restart/dedupe coverage, but neither subsystem has a multi-day outage/restart soak. Long gaps can leave explicitly missing observations; no continuous live opportunity history is invented after a restart.
- Execution, leverage selection, liquidation modeling, and live order management are deliberately absent.
- The terminal chart pans/zooms within the most recent 80 closed bars exposed by the existing snapshot API. It is not an unlimited historical chart or a detachable native MT5 window system.
- Interactive step/play replay remains a synthetic six-frame example. Historical production-scanner replay now runs offline; the terminal's historical event dialog is a fixed hindsight window, not a tick-by-tick visual simulator.
- Fill model uses M5 OHLC, assumed 5 bps fee plus 2 bps slippage per side (stress: 7.5 + 5), no order-book depth, queue, latency fill, tick-size rounding, sizing, or portfolio allocation. Funding timing on intrabar exits is uncertain; possible charges are included conservatively.
- Live journal does not reconcile actual funding yet: all-cost net is null, before-funding outcomes are separately labelled. Historical public funding is available for all six pairs in this dataset.
- Historical next-open fills assume immediate decision availability. Actual feed delays are journaled for live decisions but not simulated as executions. Delayed/gapped observations are not proof an order could have been filled.
- Gate totals describe the displayed decision state each M5 close; archived zones can remain visible. They are not counts of independently rejected trade opportunities. No counterfactual profitability is assigned to blocked bars.
- The baseline's origin-centered zone is preserved for comparison. Phase 3's actual retracement, breakout/retest and sweep geometries are separate experimental hypotheses; none passed validation. Main-chart overlays are baseline; variant overlays are opened from Strategy Lab.
- Source/config changes create new trial/shadow namespaces rather than overwriting old observations. The new 360-day period has now been seen and cannot be called unseen again. Current-universe selection, only eight hypotheses, coarse OHLC fills and limited bootstrap blocks restrict generalization. The selected variant is the least-bad training choice, not a profitable winner.
- The cost model still uses legacy identifier `FOUR_BAR_CLOSE` internally for time stops. Actual horizon is stored in `costs.horizon` and tested/displayed correctly as 4 or 8 bars. There is no claim that an eight-bar result is a four-bar sniper.
