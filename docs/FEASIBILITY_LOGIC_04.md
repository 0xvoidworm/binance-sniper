# Kelayakan setup dan sesi 3–4 jam — putaran 0.6.4

## Kontrak sebelum implementasi — 19 September 2026

Permintaan: perbaiki kontradiksi logika setup/sizing/biaya, audit ketidakcocokan,
berikan laporan dan batas konfigurasi untuk sesi pengguna 180–240 menit.
Lingkup satu kandidat: pemeriksaan kelayakan sebelum aktivasi; satu sumber
perhitungan SL/TP dan imbalan setelah biaya; observabilitas penolakan; regresi
supervisor empat jam sintetis; instalasi hanya jika sesi pengguna sudah idle.

Anggaran kerja: maksimum 90 menit, satu kandidat cost-aware, tes deterministik,
dua pembacaan setup GET-only dan satu probe input maksimal 5 menit. Tidak
mengaktifkan live/paper, mengirim order, mengubah leverage/akun/saldo, atau
menjalankan eksperimen uang nyata oleh agen. Durasi 3–4 jam adalah sesi yang
nanti dapat diaktifkan pengguna, bukan klaim sudah diuji live oleh agen.

Kriteria kandidat ditetapkan sebelum pengujian: pola arah, ATR minimum 0.10%
dan maksimum 3%, SL 2 ATR, rasio bersih minimal 1.1, funding reserve, batas
modal/risiko, Stop dan rekonsiliasi dipertahankan. TP dasar 3 ATR boleh diperluas
untuk menutup biaya sampai maksimum 6 ATR (tiga kali jarak SL), bukan tanpa
batas. Batas 6 ATR adalah kebijakan strategi baru, belum bukti edge/optimasi;
jarak lebih jauh dapat menurunkan peluang TP tercapai. Tidak menurunkan
standar rasio, menghapus biaya, memperbesar risiko, atau mengulang tuning agar
terlihat menghasilkan transaksi. Jika dalam batas ini tak layak: tolak.

Pemeriksaan ekonomi menggunakan fee akun, lot/minimum notional, pembulatan,
slippage entry/exit, dan cadangan funding. Harus menggunakan perhitungan sama
di preview, aktivasi dan dispatch. Snapshot/rules berubah harus diperiksa ulang.
Tidak menjadikan rentang ATR hipotetis sebagai bukti sinyal atau profit.

Plafon rugi lama 3 USDT/sesi dan 5 USDT/hari UTC tetap berlaku kecuali pengguna
menetapkan angka baru. Permintaan "maksimum" bukan izin menghabiskan saldo atau
menggunakan leverage tertinggi. Usulan setup akhir harus diuji secara angka
dan tetap dibedakan dari rekomendasi keuntungan atau jaminan kerugian maksimum.

Syarat selesai: regresi sebelum/sesudah, pipeline Rust lengkap, hasil GET dan
probe dicatat termasuk negatif, state lama tetap terbaca/tidak dihapus, runtime
terpasang diverifikasi jika idle, serta keterbatasan strategi dan order nyata
disampaikan. Mesin mencocokkan pola tetap; tidak belajar/menyetel aturan sendiri.

Bukti awal: `outputs/binance-setup-audit-20260919-dRzJ9v/EVALUASI.md`.
Bukti putaran: `~/Workspace/outputs/binance-feasibility-064-20260919-W2jEnK`.

## Temuan dan perubahan

1. **Dua hambatan berbeda.** Sesi pengguna 17:57:05–18:57:09 WIB selesai
   normal setelah sekitar 60 menit, tanpa transaksi. Sepuluh keputusan tercatat
   tertahan ATR 0.073–0.087% <0.10%, sebelum sizing dijalankan. Audit terpisah
   pada harga sekitar 81,275 USDT menemukan nol ukuran yang lolos biaya dan
   risiko dalam 58,002 kasus ATR sintetis dengan TP lama 3 ATR. Jadi biaya adalah
   kontradiksi tambahan, bukan penyebab historis yang boleh dikarang ulang.
2. **Kelayakan angka sebelumnya tidak diperiksa saat aktivasi.** Form angka
   valid tidak membuktikan ada ukuran order yang layak. Sekarang tombol GET-only
   serta aktivasi menjalankan `planning::assess`; entry menggunakan planner
   identik, dengan fee akun, minimum lot/notional dan sisa batas rugi aktual.
   Hasil bersyarat selalu dibedakan dari sinyal pasar saat ini.
3. **TP tetap 3 ATR tidak cukup menutup biaya dalam setup yang diaudit.**
   Kandidat `ensemble-2-cost-aware` mempertahankan SL 2 ATR, menyesuaikan TP
   mulai 3 hingga maksimum 6 ATR. Setiap perubahan TP menghitung ulang quantity
   dan fee. Jika gagal syarat rasio bersih 1.1, minimum lot, risiko, likuidasi
   atau cap, tidak ada entry. Ini perubahan kebijakan strategi yang disengaja,
   bukan sekadar bugfix dan bukan bukti strategi menjadi menguntungkan.
4. **Imbalan sebelumnya belum memasukkan slippage exit TP.** Sekarang reward
   model mengurangi entry/exit slippage, fee dan funding reserve tepat sekali.
   Harga SL/TP final disimpan pada keputusan, intent dan trigger proteksi yang
   sama. Tidak ada TP tampilan yang berbeda dari rencana order supervisor.
5. **Budget margin sempat diperlakukan sebagai kebutuhan saldo penuh.**
   Sekarang merupakan plafon; effective margin adalah minimum plafon pengguna
   dan saldo tersedia. Rencana order tetap wajib terjangkau, memenuhi risiko
   dan semua batas. Tidak menganggap saldo seluruhnya boleh dibelanjakan.
6. **Observabilitas dan durasi historis.** Penolakan/phase berubah masuk journal;
   label maksimum transaksi/sesi tidak lagi rancu dengan posisi bersamaan.
   Analisis GET terbaru tidak boleh memperpanjang durasi sesi lama yang selesai.
7. **Presisi ATR.** ATR adalah rata-rata desimal, bukan payload harga exchange.
   Validasi presisi payload 12 tempat desimal pada ATR antara dapat menolak
   indikator sah; perhitungan kini mempertahankan presisi dan membulatkan hanya
   harga order ke tick. Regresi khusus mencakup ATR >12 tempat desimal.

Pola breakout/pullback/range, minimum ATR, satu posisi bersamaan, batas modal,
Stop/recovery, akuntansi, usia data dan timeout posisi tidak dilonggarkan.
Tidak ada perubahan dependency/toolchain; implementasi tetap Rust 1.98.1.

## Verifikasi dan hasil negatif

- Pipeline `rust-workbench check`: **119 passed, 0 failed, 1 ignored**.
  Tes ignored membutuhkan jaringan publik dan tidak dijadikan bukti lulus.
  Format, Clippy semua fitur/target dengan `-D warnings`, doc, release serta
  audit 489 dependency lulus (`workbench-final.log`).
- Tes integrasi menjalankan supervisor produksi pada exchange tiruan untuk
  kedua arah: entry, pemasangan SL lalu TP final, fill TP, biaya, wallet/ledger
  dan penghentian batas transaksi. Bukan order produksi Binance.
- Simulasi deadline 240 menit mencakup dua periode gangguan data multi-menit.
  Sesi flat menunggu pemulihan, deadline tidak bergeser, candle pemulihan tidak
  diperdagangkan dan tidak ada restart otomatis. Ini waktu sintetis, bukan
  bukti koneksi/trading nyata empat jam.
- Regresi penolakan mencakup minimum lot/notional, biaya berlebihan, sisa rugi
  harian, dana tidak mencukupi, akun asing/order awal, data usang dan input UI
  kosong. Penolakan setup tidak menulis sesi baru atau mengirim command.
- Hasil negatif lama dipertahankan. Contoh ATR tepat 0.10% pada fixture audit
  masih gagal cap 6 ATR untuk Long; cap tidak dinaikkan supaya contoh lolos.
  Contoh positif deterministik memakai ATR 0.20%. Ini bukan penyetelan
  minimum ATR strategi atau klaim seluruh ATR lolos.
- Percobaan integrasi awal punya satu assertion lama yang mengharapkan semua
  contoh biaya tertahan. Assertion diperbarui untuk perubahan strategi yang
  disengaja; contoh fee berlebihan tetap wajib ditolak. Log tersimpan, tidak
  menganggap kegagalan awal sebagai sukses.
- Dua pembacaan setup GET-only dicoba dengan durasi 240 menit. Snapshot pertama
  layak bersyarat: contoh Long ATR 0.1000%, Short 0.1025%, quantity 0.002 BTC,
  rasio bersih dibulatkan 1.100. Perbedaan dari fixture di atas memakai harga
  quote/mark/anchor candle saat itu, bukan pengecualian aturan cap.
  Fee aktual 5 bps per sisi, minimum 0.001 BTC dan 50 USDT. ATR pasar saat itu
  0.0640%, sehingga belum membuktikan kesempatan entry aktual.
- **Snapshot setup kedua gagal `Harga preflight usang/future`; exit code 1.**
  Tidak ada klaim dua snapshot PASS. Data gagal tidak dilabeli layak, guard
  tidak dilonggarkan dan tidak dilakukan tuning/retry tanpa batas.
- Capture native `futures-064.png` diperiksa: versi 0.6.4, tombol pemeriksaan
  setup, penjelasan strategi dan durasi sesi lama 60 menit 3 detik tampil;
  form/angka privat disamarkan dan aktivasi capture dinonaktifkan. Koneksi UI
  berhasil membaca akun. Ini tidak membuktikan eksekusi order nyata.
- Probe produksi GET-only berjalan tepat **300 detik**: **79 snapshot sehat,
  3 kegagalan harga usang/future**, satu pergantian candle, nol sinyal, nol
  revision candidate/rewarm. Usia snapshot sehat maksimum 3,907 ms, gap sehat
  maksimum 6,964 ms. Sesudah setiap error pembacaan pulih, tetapi **exit code
  tetap 1** (`Probe input sesi belum stabil`), bukan PASS atau koneksi bersih.
  Candle baru tertahan ATR 0.069% <0.10%. Tidak ada mutasi Binance. Sumber
  spesifik timestamp yang tidak selaras belum dipastikan; data tidak dipaksa
  menjadi segar atau diberi toleransi tambahan demi menghilangkan error.
- `git diff --check` seluruh repository menemukan whitespace/CRLF existing
  di banyak file di luar putaran. File tersebut tidak dibersihkan/ditimpa.
  Pemeriksaan terbatas empat dokumen status yang diubah lulus; format Rust
  lulus. Tidak mengklaim seluruh dirty worktree adalah perubahan putaran ini.

## Instalasi dan integritas state — 20:14 WIB

Sebelum instalasi, global session lock berhasil diambil; sesi lama dipastikan
`finished=true`, `stop=true`, `current=null`, dan identik backup. Binary lama
beserta source/state dicadangkan privat pada folder bukti. Aplikasi idle 0.6.3
ditutup, binary dipasang melalui file sementara lalu rename atomik, dan 0.6.4
dibuka pada halaman Futures. Tidak ada start/resume trading.

- Unit `binance-desk-debug-064.service` aktif, PID terverifikasi `38062`.
- Hash SHA256 release, file terpasang dan `/proc/38062/exe` identik:
  `c241eaec26eae87909a3b3123b7523304d9eccee508cfd1b7f99b3989ea581cd`.
- SHA256 keseluruhan file sesi sebelum/sesudah identik:
  `a4a1cfb7782f18de1ef939282911bd0a1bc6b1fc4fbbf9419983cf7397a8e1b7`.
- Checksum payload di dalam envelope state tetap
  `ac8c62c940532370fb12853d34d0b2998db59848d4196305dc707401ce9b78d8`.
- Catatan terakhir tetap selesai, completed0, tanpa current intent. Batas
  tersimpan tetap60 menit; pengguna perlu mengisi180 atau240 sendiri. Acknowledgment
  dana nyata tidak dipulihkan. Session lock kembali bebas setelah instalasi.
- Journal runtime: `TERHUBUNG · BACA SAJA`. Koneksi baca akun bukan bukti
  kesiapan seluruh poll input, izin TRADE atau penerimaan order exchange.
- Binary lama dapat dipulihkan dari `rollback/binance-desk-0.6.3` ketika idle.
  Jangan menyalin balik snapshot state setelah ada sesi/trade baru karena dapat
  merusak rekonsiliasi dan batas rugi persisten.

## Isian kandidat 3–4 jam

Ini **plafon usulan yang mempertahankan batas pengguna sebelumnya**, bukan
"maksimum aman", maksimum exchange, target profit, atau izin agen mengubah akun.
Pengguna belum menetapkan kenaikan batas rugi, maka tidak ada kenaikan.

| Field Sesi otomatis | Isian |
|---|---:|
| Budget margin USDT | 100 |
| Risiko per posisi USDT | 1 |
| Notional maksimum USDT | 200 |
| Batas rugi sesi USDT | 3 |
| Batas rugi harian UTC USDT | 5 |
| Cadangan funding USDT | 0.1 |
| Leverage | 5× |
| Durasi | 240 menit; 180 untuk tiga jam |
| Maks. transaksi/sesi | 5 |

Slippage model tetap 1 bps dan buffer likuidasi 10 bps sesuai form existing;
keduanya asumsi model, bukan jaminan kondisi fill. BTCUSDT USD-M, One-way,
Single-asset USDT, Isolated dan fee USDT tetap merupakan batas dukungan.
Budget margin 100 adalah plafon, bukan keharusan menghabiskannya; notional
200 pada leverage 5 membatasi initial margin model hingga sekitar 40 USDT
sebelum cadangan. Penggunaan aktual bergantung minimum lot dan sizing risiko.

Urutan pengguna: isi batas → **Periksa kelayakan setup** → baca alasan/kesegaran
dan hasil bersyarat. Jika error, jangan menganggap hasil snapshot sebelumnya
sebagai izin; periksa ulang setelah data kembali segar. Aktivasi, jika dipilih
pengguna, selalu mengambil snapshot baru dan memeriksa ulang sebelum sesi.
Jangan menaikkan leverage/risiko atau membuang filter hanya agar ada transaksi.
Persetujuan dana nyata dan tombol aktivasi tetap tindakan pengguna; agen tidak
menekan keduanya. Konfigurasi tersimpan lama tidak ditimpa dengan kandidat ini.

240 menit adalah **durasi maksimum**, bukan janji mesin berjalan penuh:
batas rugi/jumlah transaksi/Stop/masalah saat ada exposure bisa mengakhiri lebih
cepat. Holding satu posisi maksimum 30 menit tetap berlaku. Entry ditahan
10 menit menjelang funding; posisi diminta ditutup 2 menit sebelumnya.
Sesudah selesai tidak ada sesi baru otomatis. Pastikan komputer tidak sleep
dan koneksi tetap tersedia; agen tidak mengubah pengaturan daya/jaringan.

Durasi 180–240 menit mencakup sekitar 36–48 interval lima menit, bukan
36–48 sinyal/order. Mesin mencocokkan tiga aturan tetap, **tidak belajar atau
menyetel strategi sendiri**. ATR rendah dapat membuat seluruh sesi tanpa
entry. TP lebih jauh mungkin lebih sulit tercapai dalam timeout 30 menit.
Nilai tambah statistik dan profit memerlukan bukti terpisah; tiga sampai empat
jam sendiri tidak cukup membuktikan keunggulan strategi.

## Batas bukti dan keputusan

Perbaikan konsistensi hitungan/lifecycle lulus tes lokal; setup punya contoh
matematis yang layak pada satu snapshot produksi. Koneksi belum boleh disebut
bersih karena pemeriksaan setup kedua ditolak usang. Tidak ada POST/DELETE,
perubahan leverage/akun, order, aktivasi live atau sesi paper oleh agen.
Penerimaan order nyata, proteksi pada exchange, slippage dan profit belum
terverifikasi. Batas rugi adalah pemicu stop, bukan jaminan kerugian tidak bisa
melewati angka tersebut saat gap/gangguan.

Keputusan putaran: **terima perbaikan software; kelayakan bersyarat, bukan
pengesahan live bebas gangguan/profit**. Probe nyata juga memiliki tiga error
yang pulih. Sebelum memulai, pengguna harus mendapatkan hasil pemeriksaan
setup segar; jangan mulai selama error masih tampil. Target tiga hingga empat
jam tetap sesi yang diaktifkan pengguna, bukan pekerjaan monitoring aktif agen.

Dokumentasi resmi yang diperiksa:
[Account API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/account)
untuk fee/rules akun dan
[Trade API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade)
untuk pemisahan query dan mutasi. Angka fee/minimum pada laporan berasal dari
GET akun/metadata saat audit, bukan angka contoh dokumentasi.
