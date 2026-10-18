# Pemulihan candle dan ketahanan sesi — 19 September 2026

## Kontrak putaran (sebelum implementasi)

Permintaan: upayakan mesin beroperasi dengan hasil yang bisa dibuktikan setelah
sesi 60 menit berhenti dini. Lingkup: perbaiki cache candle yang terus menolak
revisi, observabilitas, pengaman entry/pemulihan dan pelaporan sesi terkait.
Tidak mengubah strategi ensemble-1, ambang ATR, modal, risiko atau leverage;
tidak menjalankan order, aktivasi live, atau perubahan akun oleh agen.

Anggaran teknis awal: satu putaran perbaikan maksimum 120 menit, tes regresi
lokal, satu pembanding versi lama GET-only maksimal 10 menit, dan satu observasi
kontinu input produksi GET-only 60 menit. Jika
observasi gagal, simpan hasil negatif dan jelaskan; jangan otomatis mengulang
atau menyetel strategi sampai tampak lulus. Batas ini bukan persetujuan dana.

Syarat selesai: regresi membuktikan pemulihan tanpa entry dari candle revisi,
dua pembacaan konsisten sebelum rebase, tidak melewati Stop/deadline/exposure,
pipeline Rust lulus, observasi 60 menit dilaporkan apa adanya, binary terpasang
dan status runtime diverifikasi jika tidak ada sesi pengguna yang aktif.
State/jurnal/saldo tidak dihapus atau direset. Uji GET bukan uji order/profit.

## Bukti sebelum perubahan

Sesi terakhir mulai 16:05:23 WIB, deadline 17:05:23, selesai 16:25:32
(20 menit 9 detik). Sequence/completed nol, current kosong, net/fee/funding nol.
Hash integritas state cocok. Journal mencatat delapan error candle berubah
pukul 16:25:04–16:25:28, lalu batas retry 30 detik menghentikan sesi.
Keputusan terakhir: ATR 0.094% < minimum 0.10%; tidak ada entry.

`session_input` membandingkan semua bar overlap terhadap cache lama; ketika
ada perbedaan, fungsi keluar sebelum cache diperbarui. Retry atas revisi yang
persisten tidak mungkin pulih. Log lama tidak menyimpan bar/kolom lama-baru;
pemicu upstream persis sesi ini tidak dapat direkonstruksi.

Bukti putaran: `~/Workspace/outputs/binance-desk-candle-recovery-20260919-aejlZY`.

## Perubahan implementasi 0.6.3

- REST candle diberi `endTime` eksplisit, hanya interval selesai; grace dua
  detik menghindari sampel tepat di batas close. Grace bukan jaminan Binance
  tidak merevisi data. Umur maksimal sinyal tetap 15 detik.
- Perbedaan overlap masuk karantina. Dua GET dengan seluruh bar identik,
  dipisahkan ≥2 detik pada jam monotonik DAN waktu server, diperlukan untuk
  membangun ulang baseline. Riwayat yang berubah lagi memulai konfirmasi baru.
- Candle hingga akhir baseline pemulihan dilewati, tanpa keputusan arah,
  perubahan margin/leverage maupun entry. Watermark `last_bar` dipersistenkan
  sebelum kembali menunggu. Pembacaan ulang candle yang sama tidak dapat entry.
- Dengan intent/posisi, Stop, atau deadline, jalur penghentian dan rekonsiliasi
  dipertahankan. Kandidat awal masih memakai timeout data 30 detik; temuan
  lanjutan di bawah memperbaikinya menjadi pause tanpa intent sampai deadline.
  Bukan restart sesi otomatis.
- Candle yang sudah tutup sebelum aktivasi tidak boleh entry setelah grace.
- Log perbedaan memuat open-time dan kolom OHLCV/nilai publik lama-baru, bukan
  isi akun atau kredensial. Gangguan terakhir ikut disimpan bila deadline
  tercapai saat data belum pulih.
- Keputusan per candle masuk journal. UI memisahkan target durasi, durasi aktual,
  dan berhenti dini; hasil nol tetap terlihat pada sesi tanpa transaksi.
- Probe GET-only diperpanjang maksimum 3600 detik, dengan statistik gap sehat,
  revalidasi, candle segar dan sinyal. Exit nonzero bila ada error dipertahankan;
  statistik kemampuan pemulihan tidak mengganti label hasil negatif tersebut.

## Bukti tes lokal

Regresi cache dijalankan sebelum perbaikan dan gagal pada pembacaan konsisten
kedua (`regression-before.log`). Sesudah perbaikan, pengujian meliputi:
revisi persisten, revisi berubah-ubah, baca ulang terlalu cepat, waktu server
meloncat tanpa waktu monotonik, akun usang, grace close, candle tertinggal,
entry/config dilarang pada candle pemulihan, watermark persisten, serta
pemulihan yang tidak melewati Stop/deadline/posisi yang sudah terbuka.

Tes gabungan menggunakan fungsi input produksi dan supervisor produksi dengan
exchange di memori: revisi → tanpa order → dua bacaan cocok → candle terdampak
dilewati → candle berikutnya dapat entry dan langsung SL. Tes waktu sintetis
60 menit melakukan pemulihan di menit ke-20 dan berakhir tepat pada deadline
asli. Ini bukti perilaku kode, bukan transaksi Binance atau profit aktual.

## Pemeriksaan build dan pembanding

Pipeline kandidat awal `rust-workbench check` lulus: fmt, Clippy semua target/fitur
tanpa warning, **103 tes passed / 1 network ignored**, doc, build release,
dan audit 489 dependency. Pin toolchain/dependency tetap; versi aplikasi 0.6.3.
Binary observasi input (sebelum perbaikan kebijakan pause) memiliki SHA-256:
`e986846b84719f33ff441990a45af7d99efe8f9981283a817c7d4bb90f80a1dc`.

Pembanding 0.6.2 selama 600 detik: 170 snapshot sukses, satu error harga
preflight usang/future pada detik 22, pulih pada detik 25; dua rollover.
Umur snapshot sukses maksimum 3483 ms. Exit code 1 tetap dipertahankan.
Tidak ada revisi candle pada jendela pembanding ini; bukan reproduksi pasti
kejadian pengguna. Permintaan kedua proses tidak simultan/atomik, sehingga
perbedaan error/latensi tidak boleh dianggap benchmark kausal patch jaringan.

## Pemulihan nyata teramati selama probe

Pada detik 915, revisi benar-benar muncul pada REST produksi: candle dibuka
17:00:00 WIB (`open_ms=1789812000000`), volume **170.859 → 170.914 BTC**.
OHLC tidak tercatat berubah. Patch menolak input tersebut, melakukan pembacaan
ulang, lalu pada detik **919** berhasil rewarm dari dua respons konsisten.
Candle hingga 17:09:59.999 WIB dilewati dengan `side=None`, `skipped=true`.
Snapshot terus terbaca setelahnya; tidak ada mutasi/order oleh probe.

Candle berikutnya, tutup 17:14:59.999 WIB (`1789812899999`), kembali dievaluasi
dengan `skipped=false`: ATR 0.092% masih di bawah minimum strategi. Dengan
demikian pemulihan tidak terus mengunci evaluasi candle selanjutnya.

Ini membuktikan revisi REST dapat terjadi meskipun pembacaan sudah diberi grace,
dan pemulihan input sekarang bekerja pada kejadian nyata (bukan hanya fixture).
Penyebab internal upstream tidak diketahui; ini bukan bukti bahwa perbedaan
volume yang sama adalah pemicu persis sesi pengguna pukul 16:25.

## Temuan lanjutan yang mengubah keputusan ketahanan

Pada detik 2471 masih ada snapshot valid, koneksi publik gagal pada detik 2494,
dan snapshot valid berikutnya baru pada detik 2502: **gap sekitar 31 detik**.
Kandidat pemulihan candle saja masih akan menghentikan sesi melalui timeout
30 detik. Hasil ini negatif untuk sasaran sesi tetap memantau selama durasi
yang diminta; tidak dilabeli lulus dan tidak dihapus.

Keputusan dalam putaran yang sama: perbaiki kebijakan tanpa intent menjadi
PAUSE, bukan memperbesar angka timeout agar benchmark tampak lulus. Selama
data tidak valid tidak ada dispatch; durasi asli tetap berlaku menggunakan
deadline monotonik yang juga dapat menyela GET menggantung. Setelah pulih,
akun/posisi/order/wallet diperiksa dan candle pemulihan dikonsumsi tanpa entry.
Jika ada intent (termasuk hasil kirim belum pasti), Stop/pemulihan close-only
tetap berlaku. Stop pengguna tidak boleh dihidupkan kembali.

Observasi 60 menit yang sudah berjalan diteruskan sebagai bukti input produksi
yang sama, termasuk hasil negatifnya. Perubahan supervisor diuji dengan replay
gangguan dan waktu sintetis, bukan diklaim sebagai 60 menit sesi trading nyata.
Tidak membuka sesi trading/paper atau mengubah strategi/risiko akun.

Regresi `flat_session_can_pause_beyond_thirty_seconds_without_expanding_deadline`
gagal sebelum perbaikan kedua (`network-regression-before.log`). Versi akhir
lulus simulasi gap 31, 310, dan 631 detik: tanpa dispatch saat putus, tanpa
perpanjangan deadline, tanpa entry pada candle pemulihan (termasuk beberapa
candle terlewat), dan hanya candle baru berikutnya yang boleh entry.
Pemeriksaan akun usang, perubahan wallet, posisi/order asing, dan mode akun
tetap menahan entry. Tes deadline monotonik menyela GET menggantung tepat pada
durasi asli; penyelesaian/penutupan tetap memerlukan data akun terverifikasi.

Pipeline akhir `rust-workbench check` lulus seluruh tahap: fmt, Clippy, **106
tes passed / 1 network ignored**, doc, release, dan audit dependency.
Log: `workbench-final-pause.log`. SHA-256 binary akhir 0.6.3:
`e43f49f57d2cc4c75e7a90256f17b8e60a07577b5f5973f647facaf87f9d397d`.
Binary ini berbeda dari binary probe satu jam pada kebijakan supervisor;
jalur input candle yang diamati satu jam tidak berubah.

## Pemasangan dan smoke test build akhir

Probe GET-only tambahan 60 detik pada binary akhir: **17 snapshot sukses,
0 error**, satu candle segar, nol sinyal, umur snapshot maksimum 1706 ms,
gap sehat maksimum 3802 ms; exit 0 (`probe-final-60sec.log`). Ini smoke test,
bukan pengganti hasil negatif observasi satu jam.

0.6.3 terpasang di `~/.local/bin/binance-desk` dan berjalan melalui
`binance-desk-debug-063.service` (PID awal 22743). SHA-256 executable proses,
binary terpasang, dan release cocok dengan hash akhir di atas. Journal
memverifikasi `ACCOUNT_READ_ONLY_CONNECTED; snapshots=2; ...; mutations=0`.
Restart hanya dilakukan setelah memperoleh lock sesi dan memastikan state
finished/stop/current kosong; tidak mengaktifkan sesi trading.

File `session.json` sebelum/sesudah identik. Backup binary 0.6.2 dan salinan
state ada di `rollback/` dalam folder bukti (akses 0700). Launcher desktop
tetap menunjuk binary yang sama. Tidak mengubah NetworkManager, strategi,
konfigurasi risiko, API key, saldo, atau state sesi untuk membuat hasil lulus.

Screenshot kandidat `futures-063-offline.png` dan `futures-063-online.png`
sudah diperiksa: status berhenti dini/durasi aktual terlihat dan nilai akun
disamarkan. Capture tambahan dari binary terpasang timeout setelah 40 detik
(exit 124, tidak menghasilkan PNG); pemeriksaan KDE ScreenSaver mengembalikan
`true` (layar terkunci). Keterkaitan timeout dengan compositor terkunci adalah
inferensi, bukan bukti bug aplikasi. Tidak membuka kunci layar. Verifikasi
pasca-install dibatasi ke hash proses, layanan aktif, dan koneksi GET akun;
tidak mengklaim sudah memeriksa tampilan baru pada layar pengguna yang terkunci.

## Hasil final observasi 60 menit

`probe-60min.log` berakhir setelah **3600/3600 detik**, exit **1** (hasil
ketat stabilitas input **gagal**, tidak diubah menjadi lulus):

- 986 snapshot sukses, 18 pembacaan gagal, 12 pergantian candle.
- 18 error terdiri dari tiga penolakan revisi candle, delapan harga preflight
  usang/future, empat snapshot akun usang, dua kegagalan koneksi, dan satu DNS.
- Dua episode revisi pulih; tiga candidate baseline karena konfirmasi sempat
  berubah, bukan tiga sesi. Ada 166 snapshot dengan candle pemulihan ditahan.
- Gap antarsnapshot sehat maksimum **31.188 detik**; umur snapshot sukses
  maksimum **4550 ms**, masih di bawah guard lima detik.
- Dua belas candle segar teramati, **nol sinyal arah**, **nol mutasi/order**.
  Candle di luar karantina ditolak strategi karena ATR di bawah minimum 0.10%;
  keputusan terakhir ATR 0.079%. Tidak melonggarkan strategi agar ada transaksi.

Kesimpulan dibatasi: bug baseline revisi punya reproduksi sebelum patch dan
pemulihan input nyata; batas reconnect lama terbukti tidak cukup untuk jaringan
teramati. Kebijakan pause akhir, Stop, pemeriksaan akun, skip candle pemulihan,
dan deadline telah lolos tes regresi deterministik. Ini belum membuktikan sesi
supervisor akhir 60 menit dengan order nyata, profit, atau koneksi bebas gangguan.
Tidak otomatis mengulang soak atau mengaktifkan live. Sesi berikutnya tetap
memerlukan aktivasi eksplisit pengguna dari UI, dengan batas risiko semula.

## Referensi kontrak data

[Binance USD-M Market Data](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data)
mendokumentasikan waktu buka/tutup dan parameter batas REST kline. Dua bacaan
konsisten dan grace adalah kebijakan aplikasi, bukan klaim finalitas dari API.
