# Dasar riset mesin mandiri

**Penegasan terbaru — 15 September 2026:** fokus produk adalah mesin trading
otonom pada Binance, mencakup order, TP/SL dan risiko. Strategi boleh tunggal
atau kombinasi; Rust diutamakan untuk komponen baru. Lihat
[arah produk dan pemetaan kemampuan](BINANCE_AUTONOMOUS_SCOPE.md). Metode dan
angka contoh di bawah tetap berupa usulan terdahulu, bukan parameter yang
telah disetujui pengguna.

Diperbarui 14 September 2026. Bagian tujuan merupakan preferensi eksplisit pengguna; tahapan di bawah merupakan rekomendasi asisten, belum laporan pekerjaan yang sudah dijalankan.

## Tujuan tetap dari pengguna

Bangun mesin trading sendiri, beracuan dasar terbaik dan bukti yang dapat diperiksa. Tidak berencana membeli robot. Tidak terikat Jimmy atau identitas strategi lama. Referensi pihak ketiga berfungsi untuk menemukan dan menguji mekanisme, bukan menggantikan kepemilikan mesin sendiri. Hasil perlu terlihat pada UI yang sesuai data dan keputusan mesin.

Konteks terakhir: dana sudah ada di Binance; akun MT5 belum dideposit dan rencana modal MT5 mungkin maksimal sekitar USD 100. Jumlah alokasi eksperimen Binance belum ditentukan. Jangan menganggap seluruh saldo tersedia untuk bot. Tujuan awal mencakup entry selektif dan mempertahankan posisi ketika alasan strategi masih berlaku; itu preferensi desain, bukan aturan entry/exit yang sudah lengkap.

## Metode yang diusulkan: riset luas, eksperimen terbatas

### Batas kerja yang ditegaskan setelah koreksi alur

Pengguna meminta mesin yang menghasilkan, tanpa loop membingungkan: batas, keluaran dan goal harus nyata. Tujuan akhir tetap implementasi mandiri yang memiliki bukti hasil bersih pada lingkungan sasaran dan dapat dijalankan sesuai modal serta lingkup yang disepakati; bukan sekadar infrastruktur selesai atau demonstrasi profit EA orang lain. Profit masa depan tidak dapat dijamin.

Setiap putaran wajib diawali kontrak singkat: satu pertanyaan keputusan, ruang lingkup, batas kandidat/konfigurasi/revisi dan waktu kerja, artefak yang diserahkan, kriteria penerimaan serta alasan penghentian. Usulan angka awal di dokumen ini belum merupakan angka yang secara eksplisit dipilih pengguna. Jangan memulai pengujian panjang atau live dengan menganggap angka modal, batas kerugian, target return, atau tenggat sudah disepakati.

Keputusan akhir putaran:

- LANJUT: bukti memenuhi kriteria yang ditetapkan sebelumnya; sebutkan hanya tahap yang benar-benar lolos, bukan otomatis siap live.
- TOLAK: bukti gagal memenuhi kriteria; simpan hasil dan berhenti dari kandidat tersebut. Tidak membuka pencarian/penyetelan baru tanpa keputusan ruang lingkup berikutnya.
- BELUM DAPAT DIPUTUSKAN: bukti kurang atau ada blocker; sebutkan tepat yang kurang, pemeriksaan aman yang telah dilakukan dan apa yang dibutuhkan. Tidak dilabeli sukses atau diperpanjang diam-diam.

Tidak menambah UI, indikator, kandidat, fase atau optimisasi kecuali dibutuhkan untuk menjawab pertanyaan putaran. Saat batas kerja habis, serahkan bukti yang tersedia dan keputusan, bukan mengganti batas. Riset yang ditutup dengan TOLAK/kurang bukti adalah hasil putaran yang sah tetapi bukan pencapaian tujuan akhir mesin profit.

### 1. Pisahkan dua jenis referensi

- Pembanding performa: EA komersial/demo, laporan lengkap, monitoring akun, penjelasan pembuat. Catat versi, pasar, periode, biaya, ukuran posisi, kerugian, dan apakah benar bot yang sama dijalankan. Rating/video marketing hanya alat penemuan, bukan validasi profit.
- Dasar implementasi: aturan publik yang cukup rinci, paper primer yang bisa direproduksi, atau source code dengan hak penggunaan jelas. Kode yang bisa dilihat tidak otomatis boleh disalin tanpa batas.
- Referensi black-box bisa bagus sebagai pembanding tetapi buruk sebagai dasar pembangunan jika mekanismenya tidak terbuka. Jangan mengarang bagian yang hilang atau mengganti nama implementasi kita menjadi vendor EA.

### 2. Buat shortlist mekanisme, bukan sekadar merek

Usulan satu putaran awal: petakan sekitar 10–15 referensi secara ringkas, kemudian maksimal 3 mekanisme berbeda untuk pengujian mendalam. Contoh keluarga yang perlu dinilai, bukan pemenang yang ditetapkan: ekspansi volatilitas/breakout sesi, kelanjutan tren setelah pullback, dan breakout tren dengan exit mengikuti pergerakan. Hindari tiga produk dengan logika praktis identik.

Setiap kandidat punya kartu satu halaman: sumber/versi/izin, fenomena pasar yang ditargetkan, entry, invalidasi, exit/holding, sizing, asumsi data/biaya, kondisi gagal, bukti aktual, informasi yang belum tersedia, dan kecocokan Binance/modal kecil. Beri status lanjut/tolak/tidak cukup informasi beserta alasannya. Jangan buat skor 'peluang profit' dari rating atau kelengkapan dokumen.

### 3. Tulis kontrak perilaku sebelum kode

Nyatakan kapan sinyal diketahui, candle yang boleh dibaca, kapan order masuk, pembatalan, stop, exit, dan kondisi mempertahankan posisi. Bedakan aturan dari sumber dan penyesuaian Binance yang kita usulkan. Aturan keluar yang mengikuti tren harus eksplisit; jangan menambahkan trailing hanya karena terdengar lebih bagus. Jangan menggabungkan bagian terbaik beberapa robot menjadi satu strategi tanpa uji terpisah.

### 4. Uji implementasi sendiri dalam lingkungan yang sama

Gunakan data Binance dengan batas waktu/simbol ditentukan sebelum melihat hasil, biaya akun yang terverifikasi jika tersedia, spread/slippage/funding, minimum order, rounding, margin serta likuidasi bila mengklaim hasil akun futures. US$100 dapat menjadi skenario sensitivitas, bukan asumsi saldo Binance atau ukuran risiko yang telah disetujui.

Bandingkan pada alokasi/eksposur yang sebanding. Pisahkan sampel pembentukan/seleksi dan periode evaluasi yang belum dipakai memilih aturan; data yang sudah pernah dilihat tetap berlabel eksploratif. Catat semua konfigurasi, percobaan, kegagalan, dan revisi. Uji sensitivitas parameter dan biaya, ulangi bila ada randomisasi. Jika percobaan diperluas, evaluasi bias seleksi; satu holdout saja bukan jaminan bebas overfitting.

Ukuran keputusan: hasil setelah biaya, drawdown, distribusi kerugian, jumlah/penyebaran trade, kontribusi per pasar/periode, dan kelayakan order pada modal kecil. 'Lebih profit karena lot lebih besar' tidak dihitung sebagai strategi lebih baik. Bila semua kandidat gagal, simpulkan demikian, bukan menurunkan standar diam-diam agar ada pemenang.

### 5. Bangun satu inti yang dapat dibuktikan, kemudian tambah komponen

Pertahankan transport data, jurnal dan UI yang masih cocok. Jalur replay dan observasi harus memanggil aturan strategi yang sama. Tampilkan range/area, sinyal, stop, alasan menahan/keluar dan label sumber bukti; jangan menciptakan confidence atau zona yang tidak berasal dari mesin.

Pilih paling banyak satu inti untuk iterasi berikutnya. Tambah filter atau exit satu per satu dan bandingkan dengan baseline yang tetap. Uji keselamatan eksekusi dipisahkan dari uji profit: order ganda, pembulatan, disconnect, restart, rekonsiliasi posisi dan stop. Pelaksanaan live terbatas memerlukan lingkup/alokasi yang disepakati tersendiri; diskusi metode tidak mengaktifkannya.

## Batas keadaan sekarang

- Satu hasil demo Jimmy di FBS telah direkonsiliasi, net +671.44 USD dengan modal simulasi 10000 USD. Ini belum hasil mesin mandiri atau Binance. Lihat DEMO_RUN_01.md untuk kualitas data, empat pasar, biaya dan randomisasi yang belum tuntas.
- Adaptasi Binance sebelumnya tetap negatif dalam eksplorasi yang tercatat; tidak dipromosikan sebagai mesin profit.
- Belum ada shortlist baru yang selesai, kandidat baru yang diuji, source code baru yang diadopsi, atau perubahan kemampuan order dari penetapan arah ini.

## Dasar metode

- MetaQuotes Strategy Tester mendukung visualisasi dan pemisahan forward period: https://www.metatrader5.com/en/terminal/help/algotrading/testing
- MQL5 Code Base menyediakan kode untuk dipelajari/diuji; izin spesifik tetap harus diperiksa: https://www.mql5.com/en/code/mt5
- Bailey, Borwein, Lopez de Prado, Zhu, The Probability of Backtest Overfitting (2015): pengujian banyak konfigurasi pada data yang sama dapat menghasilkan temuan profit semu; seluruh percobaan dan seleksi harus diperhitungkan. https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf
