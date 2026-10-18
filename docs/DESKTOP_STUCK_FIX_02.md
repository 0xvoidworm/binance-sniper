# Debug sesi Binance Desk — 19 September 2026

Permintaan pengguna: cari penyebab aplikasi tetap macet/tidak dapat dipakai,
perbaiki penyebab dan bug terkait, lalu verifikasi.

Lingkup putaran: reproduksi jalur aktivasi, polling input, pergantian candle,
status/pemulihan sesi, dan UI; perbaiki bug yang terbukti; uji regresi, pipeline
Rust, probe GET-only berulang dan pemasangan binary lokal. Pertahankan state,
outbox, parameter strategi/risiko, toolchain dan dependency existing.

Batas kerja: satu putaran diagnosis/perbaikan, anggaran kerja awal 90 menit;
probe berulang dibatasi paling lama 10 menit per eksekusi. Anggaran ini batas
kerja teknis, bukan persetujuan modal/risiko pengguna. Tidak memulai sesi,
mengirim order, mengubah konfigurasi akun atau menghentikan sesi aktif pengguna.

Selesai ketika penyebab yang teramati dijelaskan, regresi dan pipeline lolos,
binary terpasang terverifikasi, serta status runtime dan keterbatasan aktual
dicatat. Koneksi/test software bukan bukti penerimaan order atau profit.

## Bukti awal

- Binary terpasang 0.6.1 identik dengan build release existing.
- Proses UI terhubung ke akun; probe input strategi lolos dua snapshot GET.
- Saat pemeriksaan berlangsung muncul sesi aktif baru, belum membuat intent
  posisi dan sedang menunggu candle berikutnya. Sesi ini dipantau read-only.
- Worktree berisi banyak pekerjaan existing; perubahan dibatasi desktop Rust
  dan dokumentasi putaran ini.

## Temuan yang direproduksi

- Pengguna mengonfirmasi sesi yang dipantau berhenti sendiri. State akhir
  `finished=true`, sequence/completed=0, tanpa current intent. Versi 0.6.1
  menimpa penyebab dengan `Sesi berhenti, posisi/order bot kosong`; pemicu
  persis sesi tersebut tidak tersimpan dan tidak dapat dipastikan ulang.
- Probe input produksi GET-only selama enam menit merekam 85 snapshot sukses
  dan satu `Snapshot account/posisi usang` pada detik 133, lalu pulih pada
  detik 137. Umur snapshot sukses tertinggi 4969 ms, sangat dekat batas 5000 ms.
  Jalur live lama memanggil Stop permanen pada setiap kegagalan data, termasuk
  gangguan singkat ini. Ini bukti mekanisme penghentian, bukan rekonstruksi
  pasti error sesi pengguna yang sudah tertimpa.
- Enam pembacaan privat masing-masing menyinkronkan waktu; persiapan DNS/waktu
  juga dihitung sebagai umur akun walaupun GET akun belum dimulai.
- Respons candle tertinggal di awal interval disimpan dalam cache seolah sudah
  lengkap. Tes regresi gagal sebelum patch: candle baru yang sudah tersedia pada
  polling berikutnya tidak terbaca sampai interval berikutnya.
- Tes penyebab penghentian juga gagal sebelum patch: penyebab asli hilang
  setelah cleanup/restart. UI menawarkan recovery tanpa catatan unfinished,
  dan kegagalan preflight ditampilkan sebagai wajib recovery walaupun belum ada sesi.

## Perbaikan 0.6.2

- Satu sinkronisasi waktu untuk batch enam GET, timestamp tiap permintaan tetap
  maju memakai jam monotonik. Umur akun dimulai setelah persiapan jaringan,
  sebelum GET pertama. Batas validitas lima detik tidak diperlonggar.
- Gangguan data saat sesi tanpa intent menahan entry dan mencoba ulang hingga
  30 detik sejak snapshot sehat terakhir. Intent/posisi, Stop pengguna, atau
  jendela retry habis tetap memakai jalur penghentian. Snapshot stale tidak
  boleh menghasilkan entry. Cooldown publik dan privat sama-sama dihormati.
- Cache candle baru dianggap lengkap jika candle penutupan yang diharapkan
  sudah hadir. Revisi candle/gap tetap ditolak untuk entry.
- Penyebab Stop pertama disimpan sampai cleanup dan setelah restart. Field
  opsional dihilangkan saat kosong agar fingerprint state 0.6.1 tetap kompatibel.
- Aktivasi baru/recovery disesuaikan dengan keberadaan sesi belum selesai.
  UI membedakan data terlambat dan menunggu candle, menampilkan hitung mundur
  serta angka volatilitas. Parameter strategi/risiko tidak berubah.
- Probe `--probe-auto --probe-auto-seconds 360` memakai input produksi berulang,
  hanya GET, tanpa supervisor live/order, tanpa nilai akun atau kredensial.

## Verifikasi

Pipeline final `rust-workbench check` **lulus**: fmt, Clippy semua target/fitur
tanpa warning, **94 tes passed / 1 network ignored**, doc, release, serta audit
489 dependency. Toolchain dan versi dependency tetap; hanya versi package
aplikasi berubah menjadi 0.6.2. Tes baru mencakup cache candle tertinggal,
pemulihan data sementara ketika flat, penghentian saat ada intent/Stop pengguna/
batas retry habis, penyebab Stop yang bertahan setelah cleanup/restart, tombol
recovery, countdown dan penandaan data stale.

Probe enam menit sesudah patch: **97 snapshot sukses, empat error sementara,
satu pergantian candle**. Tiga error `Harga preflight usang/future`, satu
`Snapshot account/posisi usang`; masing-masing diikuti snapshot sehat dalam
3–4 detik. Probe stabilitas ketat tetap keluar nonzero karena memang ada error;
tidak diklaim sebagai koneksi tanpa gangguan. Median umur snapshot sukses
turun dari 1913 ms menjadi 1199 ms; maksimum sukses 3455 ms. Ini pengukuran pada
dua jendela waktu berbeda, bukan benchmark terkontrol. Log negatif dipertahankan.

Penanganan retry pada supervisor produksi diuji dengan exchange deterministik:
ketika flat tidak mengirim order saat data gagal, dapat melanjutkan evaluasi pada
frame segar, dan tidak melanjutkan entry setelah Stop atau ketika ada intent.
Probe Binance hanya memakai transport ReadOnly; bukan uji sesi uang nyata.

Binary terpasang kemudian diuji lagi dengan dua snapshot GET: sukses, tanpa
error, umur maksimum 1176 ms. UI native online dan offline berhasil di-capture
dan diperiksa; online memperlihatkan akun terhubung dan preflight terbaca.
Pada candle 16:00 WIB, volatilitas ATR 0.086% berada di bawah minimum strategi
0.10%; keputusan menunggu pada waktu itu sesuai aturan, bukan freeze UI.

## Pemasangan dan keputusan

**Selesai: patch 0.6.2 dipasang dan aplikasi dibuka ulang** pada Eksekusi Futures.
Unit `binance-desk-debug-062.service` running, PID saat verifikasi 12833.
Hash SHA-256 binary proses, binary terpasang dan build release sama:
`9e8ab1111f4921eb434059db6e690ba29405e65e051200e11ebece6b750bc36e`.

Sebelum restart, lock sesi berhasil diambil dan state diverifikasi selesai,
tanpa current intent/riwayat transaksi. Binary 0.6.1 dan folder live dicadangkan
ke direktori `rollback/` berizin 0700 pada folder bukti. Perbandingan sesudah
restart memastikan `session.json` tidak berubah; tidak ada penghapusan/reset
jurnal, migrasi akun, order, atau aktivasi sesi oleh agen. Proses baru berhasil
membaca akun dua kali. Aktivasi sesi baru tetap lewat UI oleh pengguna.

Keterbatasan aktual: pemicu spesifik sesi lama tidak dapat direkonstruksi karena
diagnosis 0.6.1 sudah tertimpa. Mekanisme penghentian akibat error data sementara
terbukti lewat kode dan probe. Data Binance/jaringan masih sesekali terlambat;
patch menangani retry terbatas saat flat dan tetap menolak data usang. Siklus
order/proteksi diuji pada exchange tiruan; penerimaan order nyata dan profit
tidak diuji dalam putaran ini. Tidak membuka putaran penyetelan strategi baru.

Bukti: `~/Workspace/outputs/binance-desk-debug-20260919-lm2Lgj`.
