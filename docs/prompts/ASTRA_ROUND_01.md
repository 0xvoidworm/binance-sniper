# GPT-6 Astra — Putaran 01: mesin mandiri dan bukti yang bisa diaudit

Kamu menjadi engineer pelaksana dan peneliti strategi untuk proyek trading milik saya. Arsitek/auditor lain akan memeriksa hasilmu setelah putaran ini. Kerjakan sampai keluaran putaran terverifikasi atau batas kerja tercapai; jangan berhenti pada rencana. Jangan menjanjikan profit atau melanjutkan sendiri ke putaran berikutnya.

## 1. Tujuan dan otoritas

Tujuan akhir saya adalah mesin trading buatan sendiri yang menghasilkan keuntungan bersih setelah biaya. Saya tidak ingin membeli, menyewa, atau berlangganan robot. Saya ingin aturan dan hasil mesin terlihat, proses tidak berputar tanpa batas, dan keputusan akhirnya jelas.

Pertanyaan putaran ini: dari maksimal tiga mekanisme publik yang masuk akal untuk Binance USD-M perpetual, adakah satu implementasi mandiri yang layak diajukan ke audit berikutnya berdasarkan bukti lokal yang dapat direproduksi?

Kamu boleh meneliti sumber publik, membaca repo, mengunduh data pasar publik secara wajar, menulis kode dan tes lokal, menjalankan simulasi historis/replay, serta menyiapkan tampilan bukti. Utamakan pekerjaan nyata dengan alat yang tersedia. Jika tidak punya akses repo/terminal, katakan keterbatasannya dan minta akses atau berkas yang relevan tanpa rahasia; jangan mengarang pemeriksaan, perubahan file, maupun hasil eksekusi.

Putaran ini tidak mengizinkan order sungguhan, order test ke akun, cancel, perubahan leverage/margin/API key/whitelist, transfer, deposit, pembelian, pemasangan robot baru, atau aktivasi Algo Trading. Jangan menambahkan jalur mutasi akun. Pertahankan kebijakan existing ipRestrict=false sebagai warning tanpa mengubah konfigurasi kredensial. Jangan meminta atau menampilkan secret.

## 2. Lokasi dan keadaan awal

Proyek Windows:
%USERPROFILE%\Desktop\Workspace\projects\binance-sniper

Baca AGENTS.md yang berlaku, kemudian HANDOFF.md, STATUS.md, docs/RESEARCH_DIRECTION.md, docs/DEMO_RUN_01.md, dan dokumen strategi/data terkait kode yang benar-benar akan digunakan. Bedakan status terbaru dari bagian arsip. Audit singkat keadaan aktual; jangan mengulang seluruh fase lama.

Konteks pada penulisan prompt, 14 September 2026, yang harus diverifikasi jika relevan:

- HEAD ce65087 dengan banyak perubahan dan file belum di-commit. Pertahankan seluruh pekerjaan pengguna. Jangan reset, membersihkan worktree, menghapus bukti negatif, atau menganggap repo bersih.
- Sudah ada feed Binance read-only, REST fallback saat WS silent, penyimpanan historis/funding, replay, jurnal, dashboard terminal, dan pipeline bukti laporan MT5. Gunakan bagian yang masih cocok; hindari rewrite menyeluruh.
- Dashboard lokal terakhir http://127.0.0.1:8765; / dan /tester menampilkan bukti MT5, /market menampilkan observer Binance. Status proses/port harus diperiksa ulang bila diperlukan. Jangan mematikan proses yang kepemilikannya tidak terverifikasi.
- Demo vendor Jimmy, produk MQL5 122237 v5.20, menghasilkan +671.44 USD dari saldo simulasi 10000 USD pada satu sampel pendek FBS; 22 trade, PF 1.98, reported equity DD 3.70%, kualitas historis 83%. Itu bukti demo vendor, bukan hasil kode kita, bukan Binance, dan bukan validasi profit masa depan. Rincian keterbatasan tersedia di docs/DEMO_RUN_01.md.
- Adaptasi range breakout Binance sebelumnya negatif: 2685 posisi model tertutup, mean net -0.075788R, PF sekitar 0.875. Delapan varian fase sebelumnya juga tidak lolos. Jangan mendaur ulang hasil itu sebagai ide baru atau memilih subgroup positif setelah melihat hasilnya.
- Dataset 90 hari dan 360 hari sudah pernah dilihat. Jangan melabelinya sebagai holdout baru hanya karena strategi, nama run, atau pembagian tanggal berubah.
- Pengujian kode terakhir yang dilaporkan: 130 Python dan 19 JavaScript; angka ini historis, bukan hasil tes yang kamu jalankan.
- Modal Binance yang dialokasikan, toleransi kerugian, dan target return belum ditentukan. USD 100 sebelumnya konteks potensi deposit MT5; bukan saldo Binance yang disetujui untuk dipakai.

Ikuti runtime/lockfile repo. Pakai .venv proyek, jangan memasang paket ke runtime bawaan. Jangan membuat file/shortcut di Desktop atau mengubah desktop-guard. Gunakan patch lokal yang terfokus. Jangan commit/push atau mengubah layanan permanen tanpa instruksi baru.

## 3. Batas putaran — catat sebelum eksperimen

Angka berikut adalah usulan batas pekerjaan arsitek untuk prompt ini, bukan kesepakatan risiko finansial atau target profit pengguna:

- Maksimal 4 jam waktu dinding sejak mulai, atau sampai jatah sesi alat/model tidak cukup untuk menyelesaikan tahap berikutnya, mana yang lebih dulu. Catat waktu mulai dan deadline aktual. Sisihkan 30 menit terakhir untuk verifikasi dan handoff; jangan sengaja menghabiskan waktu jika sudah selesai.
- Riset sumber maksimal 45 menit dan maksimal 10 referensi substantif. Tidak perlu memenuhi kuota bila bukti yang layak sudah cukup.
- Maksimal 3 mekanisme berbeda, maksimal 2 konfigurasi aturan per mekanisme: total maksimal 6. Varian parameter, filter, exit, timeframe dan pengecualian simbol termasuk konfigurasi; jangan menyembunyikan pencarian di bawah label sensitivity test.
- Ruang pasar dibatasi BTCUSDT dan ETHUSDT perpetual USD-M untuk perbandingan awal, selama data/kontraknya dapat diverifikasi. Ini cakupan riset, bukan rekomendasi alokasi atau klaim keduanya paling profit.
- Dua skenario biaya yang didaftarkan sebelumnya: dasar dan satu stress. Tidak ada pencarian leverage, hyperparameter sweep, ML/AI ensemble, atau penambahan simbol untuk menyelamatkan hasil negatif.
- Maksimal 2 putaran koreksi terfokus setelah hasil uji pertama. Koreksi bug harus mempertahankan maksud aturan; perubahan hipotesis memakai slot konfigurasi tersisa. Simpan jejak hasil sebelum koreksi. Bug yang mengungkap/mencemari evaluasi final wajib dilaporkan.
- Jika deadline/batas tercapai, hentikan pekerjaan baru, jangan tinggalkan eksperimen tak terawasi, dan serahkan bukti yang tersedia. Jangan memperpanjang diam-diam.

Sebelum membaca hasil P&L baru atau menjalankan kandidat, tulis PROTOCOL.md yang mencakup mekanisme/konfigurasi, sumber aturan, rentang tanggal UTC, manifest data, pembagian pengembangan/seleksi/evaluasi akhir, warmup dan pemisahan posisi lintas batas, model fill/biaya, normalisasi eksposur, kriteria numerik penerimaan, dan kondisi berhenti. Bekukan dan simpan hash protokol. Metadata cakupan/kualitas data boleh diperiksa sebelumnya; bukan mencari periode yang menguntungkan.

Pilih detail teknis kecil secara mandiri dan jelaskan alasannya. Jangan membuat pekerjaan ini terhambat hanya karena modal live belum diputuskan. Pertanyaan diperlukan hanya jika akses/otoritas/informasi material benar-benar menghalangi. Bila suatu instruksi alat/skill memaksa berhenti atau mengubah arah, sebutkan instruksi dan dampaknya dengan singkat.

## 4. Riset yang menghasilkan kandidat implementasi

Cari mekanisme dengan aturan publik cukup lengkap, alasan pasar yang masuk akal, dan bukti yang dapat ditelusuri. Paper primer, dokumentasi pembuat, source berlisensi jelas, serta laporan transaksi/pengujian lengkap lebih bernilai daripada rating atau klaim marketing. YouTube boleh menjadi pintu masuk; catat video/timestamp bila benar-benar ditinjau. Jangan mengarang isi video yang tidak bisa diakses.

Pisahkan pembanding performa black-box dari dasar implementasi terbuka. Source yang bisa dibaca belum tentu boleh disalin. Jangan decompile EX5, membypass proteksi, atau mengklaim mengetahui filter internal Jimmy. Simpan URL, tanggal akses, versi/izin, pasar/periode yang diuji sumber, serta keterbatasannya. Dukungan sumber pada forex tidak otomatis menjadi bukti pada Binance.

Buat kartu ringkas maksimal tiga mekanisme yang berbeda: fenomena yang ditargetkan, aturan entry, invalidasi, exit/holding, sizing untuk simulasi, kondisi gagal, bukti pendukung, dan adaptasi Binance. Jangan memilih tiga merek yang sebenarnya memakai logika sama. Tidak ada kandidat wajib menang atau aturan bahwa Jimmy pasti terbaik.

Preferensi saya adalah entry selektif dan mempertahankan posisi selama alasan strategi masih berlaku. Itu bukan kewajiban trade harian dan bukan alasan memaksakan stop waktu empat candle atau TP 1R. R adalah satuan pembanding risiko, bukan plafon keuntungan. Aturan exit/trailing harus eksplisit dan punya alasan; jangan menggabungkan fitur banyak robot tanpa menguji pengaruhnya. Jangan memaksakan risiko 1% sebagai sasaran hasil. Jangan memperbaiki hasil strategi negatif dengan memperbesar lot, all-in, atau menggandakan taruhan setelah rugi.

## 5. Implementasi dan bukti

Bangun perubahan minimum untuk menjalankan kandidat terpilih dari shortlist di harness bersama. Tidak wajib mengimplementasikan tiga kandidat jika hanya satu yang memiliki dasar layak. Gunakan aturan inti yang sama untuk replay dan proyeksi chart/observer; jangan membuat engine visual yang berbeda dari engine pengujian.

Keputusan hanya menggunakan informasi yang tersedia saat itu. Catat waktu data tersedia, waktu sinyal, asumsi waktu fill, entry/stop/exit, alasan menahan atau keluar, dan sumber data. Jangan memakai candle higher-timeframe yang belum selesai, fill pada harga yang sudah lewat, atau trailing dengan high/low masa depan. Perlakukan spread/slippage, gap, intrabar stop/entry ambiguity dan partial candle secara eksplisit; gunakan asumsi konservatif yang konsisten dan laporkan keterbatasannya.

Manifest data harus berisi simbol, interval, batas UTC, sumber, tanggal pengambilan, coverage, gap/duplikat, dan checksum. Gunakan data publik resmi serta cache yang tervalidasi; patuhi rate limit dan bounded retry. Jangan memilih hanya hari data bagus tanpa melaporkan eksklusi. Verifikasi funding dan aturan kontrak yang relevan dari sumber resmi; biaya akun yang tidak diketahui harus dilabeli asumsi, bukan biaya aktual. Funding/biaya yang hilang tidak boleh diam-diam dianggap nol lalu disebut net lengkap.

Bandingkan konfigurasi pada data, fill, biaya dan eksposur yang sebanding. Pertahankan baseline no-trade dan, jika kompatibel tanpa merombak scope, baseline lama yang dibekukan. Laporan R boleh untuk analisis sinyal, tetapi keuntungan akun dalam USD/persen memerlukan ledger posisi, ukuran, biaya dan equity yang konsisten. Untuk skenario futures, margin, rounding, minimum notional, kontrak linear, posisi bersamaan dan potensi likuidasi tidak boleh diabaikan jika kamu mengklaim hasil akun.

Boleh tampilkan sensitivitas modal simulasi USD 100 sebagai contoh kelayakan order, bukan modal live yang sudah disetujui. Jika model akun belum memadai, laporkan keterbatasan itu dan jangan mengarang proyeksi dolar. Laporkan trade/hari aktual termasuk hari tanpa trade; tidak ada target pendapatan harian yang dijanjikan.

Seleksi maksimal enam konfigurasi hanya memakai bagian pengembangan/seleksi. Bekukan paling banyak satu kandidat dan konfigurasinya SEBELUM membuka evaluasi akhir. Jangan menjalankan semua kandidat pada holdout akhir lalu memilih pemenang; jika kandidat final gagal, jangan mencoba runner-up di holdout yang sama dalam putaran ini. Simpan seluruh percobaan, termasuk gagal dan hasil negatif. Data yang sebelumnya sudah diperiksa tetap berlabel eksploratif; jika tidak ada evaluasi yang layak disebut belum tersentuh, jangan membuat klaim tersebut atau menunggu berhari-hari tanpa batas.

Tetapkan jumlah sampel minimum dan uji kestabilan yang sesuai sebelum hasil dilihat; jelaskan alasan dan keterbatasannya. Kriteria setidaknya menilai net setelah biaya, ketahanan skenario stress, drawdown equity, penyebaran hasil antarperiode/simbol, dan konsentrasi pada sedikit trade. Net positif sendiri bukan sertifikat edge. Jangan memilih pemenang hanya karena total profit tertinggi atau mengubah ambang supaya lolos. Toleransi kerugian live tetap keputusan pengguna.

Uji yang wajib relevan dengan perubahan: anti-lookahead/prefix invariance, closed-bar timing, biaya/funding long-short, fill ambigu, gap, pembulatan bila digunakan, idempotensi/restart state bila disentuh, rekonsiliasi ledger dengan ringkasan, dan kecocokan keputusan replay dengan chart. Simpan command dan hasil aktual. Jalankan suite regresi yang terdampak; ulangi/besarkan cakupan hanya karena perubahan baru, kegagalan, atau kekhawatiran spesifik.

UI tetap bergaya terminal yang ada. Tambahkan hanya panel/route bukti minimal yang diperlukan, tanpa mengganti beranda atau merusak halaman lama. Tampilkan identitas mesin/run, asal HISTORICAL_REPLAY atau OBSERVATION, entry/stop/exit dan alasan, gross/biaya/net beserta kelengkapannya, serta status kandidat. Label hasil vendor dan hasil kode kita harus berbeda. Jangan membuat confidence, zona atau angka profit yang bukan keluaran engine. Jika UI dibuat/diubah, verifikasi actual render dan setidaknya satu replay berurutan; screenshot saja tidak membuktikan logika. Jika belum terintegrasi, laporkan sebagai belum dikerjakan. Jangan mengaktifkan observer strategi baru secara permanen dalam putaran ini.

## 6. Keputusan dan syarat berhenti

Akhiri dengan tepat satu keputusan utama:

- LANJUT KE AUDIT: satu kandidat memenuhi kriteria yang dibekukan, implementasi dan hasilnya dapat direproduksi, serta keterbatasan bukti jelas. Ini izin untuk diperiksa auditor, bukan persetujuan live atau klaim tujuan akhir sudah tercapai. Jika bukti baru eksploratif, tulis itu secara menonjol.
- TOLAK: kandidat yang diuji tidak memenuhi kriteria dengan bukti yang cukup untuk kesimpulan tersebut. Simpan kode/hasil dan jelaskan kegagalan dominan. Jangan otomatis mencari keluarga strategi keempat.
- BELUM DAPAT DIPUTUSKAN: data, jumlah kejadian, pemodelan biaya/akun, akses alat, bug, atau batas kerja membuat kesimpulan belum sah. Nyatakan tepat apa yang kurang, apa yang sudah dicoba, dan batas yang tercapai. Tidak boleh dilabeli sukses atau gagal strategi jika implementasinya belum valid.

Kesuksesan rekayasa, hasil vendor, hasil historis kode sendiri, dan keuntungan akun live adalah empat hal berbeda. Jangan menyamakan selesai putaran dengan tercapai mesin profit. Berhenti setelah paket audit tersedia; tidak ada putaran lanjutan otomatis.

## 7. Paket yang harus bisa diperiksa auditor

Simpan keluaran di folder unik:
%USERPROFILE%\Desktop\Workspace\outputs\binance-sniper-round01-<timestamp>

Isi minimum, sesuai pekerjaan yang benar-benar selesai:

1. PROTOCOL.md beserta versi/hash awal; catatan perubahan protokol bila ada, tidak ditimpa.
2. SOURCES.md: kartu kandidat, sumber primer, izin/atribusi, klaim sumber vs temuanmu.
3. Manifest data dan seluruh trial/result ledger dalam format mesin, berisi run ID, config, source/data/protocol hash, status, biaya, jumlah trade dan metrik. Jangan memasukkan rahasia atau membagikan berkas privat.
4. Entry point dan command nyata untuk reproduksi, beserta lokasi cache/data yang diperlukan. Jangan sekadar menampilkan pseudocode atau command yang belum pernah dicoba.
5. AUDIT_PACKET.md: keputusan utama, alasan, tabel semua konfigurasi, hasil seleksi dan final terpisah, net/PF/DD/sampel/biaya, perbedaan dari mesin lama, kendala, file berubah, command tes beserta hasil, dan apa yang belum terbukti.
6. Snapshot perubahan: HEAD awal/akhir, status git awal/akhir, daftar perubahan sebelumnya vs perubahanmu, diff yang relevan, serta manifest/hash file baru. Commit tidak diwajibkan dan repo tidak harus bersih. Jangan memasukkan .env, token, cookie, identitas akun, atau raw secret-bearing logs.
7. Bila tersedia: screenshot/rekaman replay yang terhubung ke run ID dan sumber data, dengan klaim verifikasi sesuai yang benar-benar diamati.

Perbarui HANDOFF.md dan STATUS.md secara ringkas sesuai keadaan aktual; pertahankan arsip. Ekspor ringkasan lengkap yang mudah dicopas sebagai ASTRA-HASIL-PUTARAN-01-<timestamp>.md ke %USERPROFILE%\Downloads, dengan tautan/lokasi paket audit. Jangan menimpa file lama bernama sama.

Jawaban akhir harus bisa langsung saya copas ke auditor: keputusan, apa yang dibangun, apa yang benar-benar dijalankan, angka hasil dan keterbatasan terpenting, tes, lokasi artefak/kode, status tidak ada transaksi atau mutasi akun, serta paling banyak satu langkah berikutnya bila diperlukan. Jangan hanya mengatakan 'fase selesai', 'semua tes lolos', atau menawarkan melanjutkan.

Gunakan bahasa Indonesia yang jelas dan ringkas. Beri update singkat saat ada temuan atau pergantian tahap penting. Tidak perlu memaparkan proses berpikir internal; cukup keputusan, alasan teknis ringkas, dan bukti. Mulai sekarang dengan inspeksi terarah, catat kontrak putaran, lalu kerjakan dalam batas di atas.
