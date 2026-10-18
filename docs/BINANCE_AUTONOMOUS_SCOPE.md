# Arah mesin trading otonom Binance

Ditetapkan dari penegasan pengguna pada 15 September 2026.
Dokumen ini memisahkan tujuan produk, temuan kode, dan rancangan yang belum diimplementasikan.

**Implementasi P2, 20 September:** [kandidat0.6.8](EVALUATION_BRIDGE_P2_08.md)
mencakup P1 + exporter/validator/proposal boundary lokal teruji. Tidak ada
Hermes runtime/model run/auto-apply. Pilihan provider/model/biaya/data keluar
belum ada. Runtime pengguna tetap0.6.6; jangan menyebut sudah deploy/siap24jam.

**Riwayat implementasi P1, 20 September:** [kandidat0.6.7](RELIABILITY_P1_07.md) sudah
dibangun/diuji untuk snapshot, history asinkron, status Stop dan bukti exit.
Belum dipasang; runtime pengguna tetap0.6.6. Hermes dan exporter belum tersedia.

**Pembaruan 20 September 2026:** baseline native kini 0.6.6. Audit sesi pengguna
300 menit mencatat tiga trade, net −0.34155740 USDT, berhenti pada 194 menit
5,332 detik akibat bukti order usang. Pengguna meminta rancangan lanjutan dengan
Hermes di lapisan evaluasi, tanpa kewenangan trading. [Rancangan 07](HERMES_INTEGRATION_PLAN_07.md)
mengurutkan patch reliability → ekspor aman → pilot Hermes → eksperimen terukur.
Rancangan belum implementasi/pemasangan atau bukti profit/operasi24jam.

**Riwayat 0.6.0, 16 September:** sesi otomatis native telah menghubungkan
tiga aturan strategi, arah Long/Short/menunggu, SL/TP, sizing margin, order,
pengawas posisi, recovery dan ledger fill/fee/funding. Pengguna mengaktifkan
satu sesi dengan batas modal/risiko, tanpa menentukan arah atau klik per order.
Kode diuji dengan exchange tiruan dan probe Binance GET-only. Tidak ada order
uang nyata oleh agen atau bukti profit. Catatan versi sebelumnya di bawah adalah
riwayat, bukan daftar kemampuan0.6.0. [Hasil putaran](AUTONOMOUS_SESSION_01.md).

**Perkembangan 16 September 2026:** fondasi `engine-rs` sudah diimplementasikan
dan diverifikasi untuk simulasi lokal. Sizing, state order/posisi, TP/SL, halt,
dan jurnal tersedia; adapter/live execution dan validasi strategi belum tersedia.
[Kontrak dan CLI](../engine-rs/README.md), [keputusan putaran](EXECUTION_ROUND_01.md).
Tabel di bawah mempertahankan hasil pemeriksaan 15 September sebagai titik awal.

**Perkembangan desktop 16 September:** aplikasi native Rust `desktop-rs` sudah
terpasang. Versi 0.2.0 menampilkan candle/volume BTC/ETH publik pada Spot/Futures
dan empat timeframe lewat polling REST, dengan status stale/reconnect dan
kontrol chart. Feed hanya observasi, belum input strategi atau pembacaan akun.
[Putaran pasar native](MARKET_DESKTOP_ROUND_01.md). Diskusi HFT ditunda sesuai
arahan pengguna; desain mesin eksekusi/risiko terpisah dari UI tetap dipertahankan.

**Perkembangan akun desktop 16 September:** versi 0.3.0 menambahkan konektor
Futures read-only Rust dan form key/secret HMAC dengan KWallet opsional.
Saldo/posisi/margin/leverage dan mode akun memiliki UI desimal, polling dan
stale/error yang eksplisit. Software, transport publik dan KWallet terverifikasi;
**autentikasi akun live menunggu kredensial Linux**. Bukan aktivasi strategi,
order atau penetapan angka leverage. [Putaran akun](ACCOUNT_DESKTOP_ROUND_01.md).

**Patch akun 0.3.1:** validator nama simbol Unicode diperbaiki setelah keluhan
pengguna. Dua pembacaan akun nyata berhasil pada worker UI; hambatan autentikasi
putaran awal di atas terlewati. Scope tetap observasi, tanpa order/perubahan akun.
[Perbaikan parser](ACCOUNT_PARSER_FIX_01.md).

**Forward paper 0.4.0:** harga publik BTCUSDT Futures terhubung ke strategi EMA/ATR
dan engine simulator dengan jurnal persisten. UI native menampilkan sesi60menit,
alasan keputusan, posisi/TP/SL dan modal virtual1000USDT. Belum bukti profit,
adapter order exchange atau model leverage/margin/likuidasi penuh.
[Putaran paper](FORWARD_PAPER_ROUND_01.md).

**Komponen eksekusi 0.5.0:** model margin/leverage/likuidasi isolated, preflight
bracket dan fee akun, konektor production order/Algo TP-SL/config/cancel/query,
outbox durabel serta pengendali keputusan otomatis tersedia. Mutasi diuji pada
exchange lokal di memori; GET akun nyata terverifikasi. Supervisor, recovery
sesi dan ledger nyata masih perlu diintegrasikan sebelum aktivasi live UI.
[Putaran eksekusi Futures](FUTURES_EXECUTION_ROUND_01.md),
[kontrak komponen](../desktop-rs/src/futures/README.md).

## Tujuan pengguna

**Pilihan pasar, 16 September 2026:** pengguna memilih **Binance Futures** untuk
menggunakan leverage. Acuan implementasi tetap USDⓈ-M/USDT sesuai fondasi proyek;
ini bukan penetapan angka leverage, mode margin, mode posisi atau aktivasi order.
Default binary desktop sudah `futures` dan telah diperiksa. Spot tetap tersedia
sebagai pilihan tampilan publik, sementara integrasi akun/mesin diarahkan ke Futures.

Bangun mesin trading milik pengguna yang memakai API Binance untuk membaca
perubahan pasar, menentukan arah serta waktu transaksi, mengelola order,
take-profit/stop-loss, posisi dan risiko secara otonom. Pengguna menyatakan
dana berada di Binance dan API sudah disiapkan. Sasaran hasil adalah profit
bersih yang berarti bagi pengguna. Arahan terbaru16September: dana yang disediakan
di Binance dialokasikan untuk proyek ini, strategi/rules diserahkan. Targetangka,
batas rugi dan besaran leverage belum ditetapkan; alokasi tidak sama dengan
kerugian maksimum yang diterima. Agen membangun/menguji software, tidak menjalankan
transaksi uang nyata secara otonom.

Penegasan terbaru: pengguna meminta langsung membangun dukungan margin/leverage
dan order nyata, tanpa sesi paper tambahan. **Otonom berarti satu aktivasi sesi
dengan batas modal/risiko, lalu program mengirim dan mengelola setiap order**.
Tidak perlu klik manual setiap transaksi. Batas agen tidak menjalankan transaksi
keuangan dari chat tidak mengubah target kemampuan software tersebut.

**Penegasan arah otomatis:** pengguna menginginkan mesin memilih Long atau Short
sesuai kondisi pasar. Rancangan keputusan mencakup Long/Short/menunggu; pergerakan
harga kecil tidak otomatis memicu order. Strategi mengusulkan arah dan SL/TP,
pengelola sinyal menyelesaikan konflik serta memeriksa batas risiko/biaya sebelum
entry. Dalam desain satu posisi awal, posisi lama harus dikelola dan penutupannya
terkonfirmasi sebelum membuka arah berlawanan. Long/Short manual pada form
desktop hanya kalkulator satu posisi, bukan batas arah sesi. Integrasi strategi
campuran belum ada pada patch0.5.1; integrasi tersebut kini tersedia pada0.6.0.

Sniper, scalping, tren, atau kombinasi diperbolehkan sebagai kandidat. Pemilihan
ditentukan oleh bukti pada pasar sasaran, kelayakan biaya/eksekusi dan batas
risiko. Nama proyek `binance-sniper` tidak membatasi strategi yang dapat dipakai.

## Temuan saat pemeriksaan 15 September

Pemeriksaan berupa pembacaan sumber dan catatan proyek. Tidak ada login akun,
probe API privat, order, perubahan akun atau pengujian performa baru.

| Kemampuan | Bukti yang sudah ada | Kebutuhan menuju produk |
|---|---|---|
| Data pasar Binance | REST, WebSocket, validasi candle, penjaga data usang/gap | Verifikasi feed Fedora dan sumber data yang sesuai horizon strategi |
| Akun Binance | `PrivateReadOnlyClient` membatasi akses ke GET yang diizinkan | Verifikasi autentikasi dan konfigurasi akun tanpa menampilkan kredensial |
| Strategi dan penelitian | Mesin reference/round01, jurnal, replay dan laporan | Kandidat dengan bukti yang memenuhi kriteria; hasil lama belum memenuhi tujuan profit |
| Order dan posisi | Snapshot order/posisi; posisi model pada simulasi | Siklus order kirim/diterima/terisi sebagian/penuh/batal/ditolak/tidak pasti; rekonsiliasi setelah restart |
| TP/SL | Stop model pada riset | Order proteksi pada exchange, penyesuaian terhadap jumlah fill, penutupan dan pembersihan order terkait |
| Risiko portofolio | Model biaya dan hasil penelitian dengan batas yang tercatat | Alokasi modal bot, sizing, filter instrumen, eksposur gabungan, batas rugi, serta kondisi penghentian |
| Evaluasi berkelanjutan | Riwayat eksperimen dan keputusan tersedia | Evaluasi terjadwal, pelacakan usulan perubahan, pengujian sebelum perubahan aktif |

Sumber utama: `src/binance_sniper/binance/private_readonly.py`,
`src/binance_sniper/round01/engine.py`, `STATUS.md`, `KNOWN_GAPS.md`.
Catatan runtime 14 September berasal dari lingkungan lama dan tidak dianggap
verifikasi koneksi privat pada Fedora hari ini.

## Rancangan kerja yang diusulkan

1. **Mesin eksekusi dan risiko dalam Rust.** Aturan numerik mengendalikan jumlah,
   eksposur, batas rugi dan kelayakan order. Pengelolaan status order memakai
   identitas persisten dan memeriksa keadaan exchange ketika hasil kirim tidak
   pasti, sehingga retry tidak otomatis membuat order ganda.
2. **Strategi sebagai komponen terpisah.** Tiap strategi menghasilkan usulan
   entry/exit disertai alasan dan identitas versi. Pengelola portofolio menangani
   sinyal berlawanan, modal bersama dan korelasi eksposur. Gabungan strategi
   diuji sebagai portofolio, bukan sekadar penjumlahan hasil masing-masing.
3. **Satu aturan perilaku untuk replay dan pengamatan.** Adapter pasar/simulasi
   boleh berbeda, tetapi keputusan strategi dan batas risikonya bisa dilacak
   ke versi yang sama. Biaya, keterlambatan, rounding dan fill dimodelkan dengan
   label asumsi yang jelas.
4. **Evaluasi dengan batas kerja.** Setiap putaran menetapkan pertanyaan,
   konfigurasi, data, anggaran percobaan/waktu, syarat selesai dan keputusan.
   Perubahan strategi tidak boleh menurunkan standar penilaian secara diam-diam.
5. **Hermes sebagai evaluator terpisah.** Pengguna meminta memasukkannya ke
   rancangan pada 20 September. Tugas awal membaca ekspor setelah sesi,
   mengingat eksperimen dan menyusun hipotesis; penjadwalan menyusul jika pilot
   berguna. Laporan/proposal wajib divalidasi, diuji dan direview sebelum menjadi
   perubahan versi. Tidak ada akses key/order/risiko/live-state. [Kontrak dan
   batas pelaksanaan](HERMES_INTEGRATION_PLAN_07.md) belum berupa izin instalasi
   atau run model; Hermes tetap bukan prasyarat mesin Binance berjalan.

## Urutan keluaran yang dapat diperiksa

- **Eksekusi simulasi:** satu siklus entry → fill → posisi terproteksi → exit →
  rekonsiliasi, dengan skenario fill parsial, timeout, restart, penolakan
  exchange dan data usang. Kelulusan membuktikan perilaku perangkat lunak.
- **Evaluasi strategi:** hasil bersih dan risiko dengan aturan biaya serta
  data yang dinyatakan sebelumnya. Hasil lama yang pernah dilihat tetap
  eksploratif. Keputusan kandidat dicatat tanpa menghapus hasil negatif.
- **Kesiapan operasi:** verifikasi adapter pada lingkungan yang disepakati,
  pencocokan order/posisi/biaya dan pemulihan gangguan. Penggunaan uang nyata
  memerlukan alokasi, batas risiko dan instruksi aktivasi yang jelas.

Ini urutan rancangan, bukan laporan tahap yang sudah dikerjakan atau komitmen
tenggat. Batas angka tiap putaran ditetapkan sebelum implementasi/pengujian
panjang dimulai. Sasaran profit masa depan belum terbukti.

## Informasi yang masih diperlukan

- Nilai batas margin, risiko posisi, rugi sesi/harian dan leverage diisi pada
  form sesi0.6.0. Dana sudah dialokasikan khusus proyek; jangan menanyakan ulang.
- Target profit/periode dan evaluasi strategi masih belum ditetapkan/terbukti.
  Dukungan awal margin/funding/likuidasi tersedia pada komponen Futures Rust,
  dengan batas yang dicatat di KNOWN_GAPS; engine paper v1 tetap terpisah.

## Referensi teknis

Dokumentasi resmi Binance yang diperiksa pada 15 September 2026 memisahkan
operasi order reguler dan algo/conditional, serta menyediakan query/cancel.
Adapter harus mengikuti API untuk produk yang akhirnya dipilih:

- [USDⓈ-M Futures Trade API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade)
- [USDⓈ-M User Data Streams](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/ws-api/user-data-streams)
