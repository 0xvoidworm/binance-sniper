# Freshness dan rekonsiliasi posisi — putaran 0.6.5

## Kontrak sebelum implementasi — 19 September 2026, 23:25 WIB

Permintaan pengguna: evaluasi titik patch dan hubungan antarlogika, implementasi,
pengujian serta pembaruan aplikasi. Batas satu putaran: maksimum 90 menit,
satu rancangan perbaikan freshness/reconciliation dan diagnostik, tes regresi
lokal sebelum/sesudah, pipeline Rust penuh, dua snapshot setup GET-only serta
satu probe GET-only maksimum 10 menit. Tidak melakukan tuning strategi.

Bukti awal: sesi pengguna b1a0b9d2ccad94ae berhenti setelah 2 jam 27 menit
karena observasi posisi/harga usang; satu transaksi produksi selesai dengan
net −0.39563580 USDT. Rincian pada
`~/Workspace/outputs/binance-session-review-20260919-6VRSTY/EVALUASI.md`.
Log lama tidak menyimpan umur field/latency; sebab historis spesifik tidak boleh
diklaim sudah terbukti hanya dari reproduksi deterministik.

Rancangan: baca/rekonsiliasi order sebelum mengambil frame akun/harga keputusan;
hasil rekonsiliasi sekali pakai terikat intent dan dibatasi umurnya. Gabungkan
GET order dalam batch bounded dengan satu persiapan waktu; jangan replay POST.
Guard account/quote lima detik, kepemilikan order, risk/Stop/deadline, proteksi,
ledger dua pembacaan dan larangan entry sebelum accounting tetap berlaku.
Tambahkan diagnostik aman umur account/quote serta biaya waktu query/poll dan
alasan terstruktur respons order yang belum pasti; tanpa raw response/key/URL.

Tes harus mencakup latency rekonsiliasi, frame/hasil query usang, Stop/deadline,
gagal query proteksi, uncertain/restart/partial fill, kelanjutan sesudah settlement,
dan diagnosis yang tidak mengungkap credential. Kegagalan data aktual tetap
fail-closed saat exposure; tujuan bukan mengabaikan error agar sesi selalu hidup.

Tidak mengaktifkan live/paper, membuat order, mengubah leverage/akun/risk atau
merestart sesi aktif. Instalasi lokal hanya setelah sesi tersimpan selesai,
tanpa current intent, lock tersedia dan state identik backup. Toolchain/dependency
terpin dipertahankan; tool existing cukup sehingga tidak perlu instalasi baru.

Syarat selesai: regresi dan pipeline terverifikasi, hasil GET termasuk negatif
dicatat, state/order/ledger pengguna tidak berubah, release/runtime terpasang
cocok jika idle, laporan batas bukti dan prioritas sisa. Putaran ini bukan
uji trading nyata 24 jam atau bukti profit.

Bukti putaran:
`~/Workspace/outputs/binance-freshness-065-20260919-H2KhLa`.

## Temuan dan hubungan antarlogika

Bug yang direproduksi: versi0.6.4 mengambil account/quote terlebih dahulu,
kemudian query entry/proteksi satu per satu dengan persiapan DNS/clock berulang.
Pemeriksaan `Intent::next` baru berjalan setelah query tersebut, sehingga frame
yang tadinya segar dapat melewati5detik. Worker memperlakukan error itu sebagai
Stop, menutup exposure, mencocokkan ledger, lalu menyelesaikan sesi. Inilah
mekanisme yang cocok dengan penghentian historis; field/latency spesifik pada
22:46:50 tidak tersimpan, sehingga korelasi bukan kepastian penyebab tunggal.

| Titik patch | Hubungan dan pengaman yang dipertahankan |
|---|---|
| `live`/`supervisor` | Rekonsiliasi mendahului frame baru; recovery juga membuang preflight sebelum keputusan pertama |
| `transport`/`outbox` | Maks16 GET order dengan satu persiapan waktu, bukan batch mutasi; receipt lama tidak dihapus saat error |
| `readiness` | Refresh metadata sebelum query order jika TTL tinggal≤5s; TTL60s tetap, tanpa stale fallback |
| `flow`/`supervisor` | Bukti order, akun dan quote dibatasi5s; bukti hanya sekali, terikat intent; proteksi hilang tidak boleh Watch |
| `outbox`/proteksi | POST tanpa `cumQuote` diikuti satu GET≤5s; fill lengkap baru boleh memicu SL, tidak resend entry |
| Stop/ledger | Stop/deadline menang atas sinyal; cleanup, dua pencocokan wallet/ledger dan larangan reentry prematur tetap |

Urutan aktif menjadi: metadata → GET status order → frame akun/open-order/harga
baru → pemeriksaan → keputusan. Persiapan DNS bukan umur data; timestamp
rekonsiliasi diambil sebelum GET pertama setelah persiapan. Bukti lama tidak
diberi timestamp baru hanya agar lolos. Total batch/order evidence yang benar-
benar terlambat tetap ditolak. Jika query terputus, restart mengulang GET,
bukan POST; tidak ada aktivasi/restart sesi otomatis.

Temuan respons market tambahan: contoh New Order pada dokumentasi resmi tidak
menyertakan `cumQuote`, sedangkan Query Order menyertakannya. Parser lama meminta
receipt lengkap dan jatuh ke Uncertain sampai poll berikutnya. Patch tetap
meminta receipt lengkap, tetapi mempercepat konfirmasi dengan satu GET.
Tidak menganggap absennya field sebagai fill nol atau menghitung quote dari
angka perkiraan. Tidak ada raw respons sesi lama, jadi error historical entry
belum dapat diatribusikan pasti ke field ini.
[Sumber resmi: New Order dan Query Order](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade).

Diagnostik baru memisahkan DNS/clock, HTTP, JSON/schema, penolakan API, dispatch
dan rekonsiliasi. Hanya kategori, endpoint statis dan angka latency/umur yang
dicatat; tidak ada URL signed, credential atau raw response akun.

## Pengujian lokal yang selesai

- Regresi sebelum patch gagal persis `Observasi posisi/harga usang`: dua GET
  proteksi masing-masing2200ms menghabiskan umur frame dengan lag awal1000ms.
  Setelah patch kasus yang sama tetap Trading, tanpa Stop/order tambahan.
- **129 passed, 0 failed, 1 network ignored**. Sepuluh tes baru meliputi
  latency, bukti order usang, account/quote stale atau future, bukti intent
  berbeda, proteksi hilang setelah query, pembatalan GET+Stop+restart,
  Stop/deadline sebelum keputusan, receipt tidak lengkap/NotFound, batas batch
  GET dan headroom metadata. Lifecycle partial fill, uncertain, dua arah,
  kelanjutan trade, ledger dan risiko existing tetap lolos.
- `rust-workbench check` penuh lulus: fmt, Clippy semua target/fitur tanpa
  warning, tes, doc, release dan audit489 dependency. Toolchain/dependency
  tidak diubah; versi paket menjadi0.6.5. Engine library digunakan tes desktop,
  tetapi suite engine mandiri tidak dijalankan ulang karena source tidak diubah.
- Semua hasil awal dipertahankan. Selain regresi sengaja gagal, fixture sempat
  memakai jam mundur setelah pemanggilan tick langsung; jam fixture diperbaiki
  agar monotonik, bukan melonggarkan produksi. Penyesuaian return future trait
  sempat gagal compile karena `async move` belum ditambahkan; build akhir lulus.

## Pemeriksaan produksi read-only

`--check-setup`: dua snapshot GET berhasil, keduanya layak bersyarat pada
batas tersimpan180menit/maks5trade. ATR saat itu0.1253%; sinyal lama tidak
dikejar. Ini hanya snapshot kelayakan, bukan sinyal/profit. Satu persiapan
DNS/clock2227ms terukur; tidak dihitung sebagai umur account sebelum GET.

Probe600detik selesai dengan **168 snapshot sehat, 2 penolakan revisi candle,
2 pergantian candle, 1 rewarm, 0 sinyal**, max umur snapshot4040ms dan gap sehat
maksimum10056ms. **Exit1**, bukan PASS tanpa gangguan. Tidak ada penolakan
account/quote stale pada probe ini, tetapi itu bukan bukti masalah sudah mustahil.

Pada detik525 dan528, volume candle tertutup open_ms1789836300000 berubah
195.660→195.836. Dua pembacaan konsisten kemudian memulihkan baseline, candle
hingga1789836899999 dilewati tanpa entry. Ini perilaku pengaman revisi existing,
bukan bug baru yang diselesaikan dengan menerima data berbeda tanpa validasi.
Bar pertama memberi keputusan tidak ada sinyal; bar kedua dilewati karena
revalidasi.19 snapshot memiliki label hold/revalidation, bukan19error tambahan.

Log transport mengukur beberapa persiapan DNS/clock lambat1259–2222ms pada
probe; tidak ada raw data akun. Metrik `within_30s_recovery_budget=true` hanya
pembanding historis, bukan aturan sesi0.6.5 atau bukti kestabilan24jam.
Probe dan UI memakai read-only; tidak ada mutation, order-test, live/paper baru.

## Instalasi dan pemeriksaan native

Binary0.6.5 sudah dipasang dan UI idle dibuka sebagai unit user sementara
`binance-desk-debug-065.service`, menggantikan unit0.6.4 yang idle. Sebelum
stop/install: global session lock berhasil dipegang, sesi finished/currentNone,
seluruh isi folder live identik backup dan binary lama cocok backup. Lock
ditahan sepanjang penggantian agar sesi tidak dapat dimulai di tengah instalasi.
Tidak mengubah launcher, credential, konfigurasi atau riwayat transaksi.

Binary release, terpasang dan prosesPID50931 cocok SHA256:
`808f830df6597941cc8bdd2abd1ce7a108e8221c186c61d0abb5cb2e7b96618b`.
Backup0.6.4/source/state ada pada `rollback/` di folder bukti privat; bukan
folder pemulihan `~/data`. Tidak perlu pemasangan tool/dependency baru.

Capture framebuffer native X110.6.5 berhasil dan diperiksa: halaman Futures,
preflight terbaca, akun baca saja, sesi lama selesai147menit25detik dari target
180menit, label berakhir lebih awal dan footer0.6.5. Capture menyamarkan batas/
nilai akun dan menonaktifkan aktivasi; ini bukan screenshot posisi live baru.
Unit aplikasi normal aktif; tidak ada aktivasi live/paper oleh agen.

## Keputusan putaran — 19 September 2026, 23:57 WIB

**Perbaikan software diterima dan terpasang; acceptance operasi live24jam belum
lulus/belum diuji.** Lingkup selesai dalam anggaran90menit tanpa tuning risiko
atau strategi. Probe aktual yang exit1 dipertahankan sebagai bukti negatif,
bukan diulang sampai mendapat label PASS. Source dan seluruh hasil regresi
tersedia untuk evaluasi berikutnya.

State sesi, outbox dan ledger dibandingkan ulang dengan backup pada akhir
putaran; konfigurasi dan riwayat tetap identik. Sesi baru hanya dimulai pengguna
dari UI setelah pemeriksaan setup baru. Tidak ada kewajiban mengubah batas180
menit untuk memakai patch; memperpanjang ke24jam bukan perbaikan atas data gagal.

## Batas bukti dan langkah berikut

Satu transaksi produksi pengguna pada0.6.4 membuktikan entry/SL/TP/exit dan
ledger pada satu kasus, hasil net−0.39563580USDT. SL/TP berakhir EXPIRED tanpa
child; tidak ada command cancel eksplisit, sehingga cabang cancel nyata belum
terbukti. Semua POST/DELETE pada pengujian patch memakai exchange tiruan;
probe nyata tidak menguji receipt mutation atau posisi terproteksi aktif.

REST tetap berurutan/non-atomik. Ada celah entry→SL exchange, dan gangguan DNS,
jaringan atau quote server yang betul-betul usang masih bisa menghentikan sesi
dengan intent. SL exchange hanya melindungi jika sudah diterima. Ledger memakai
dua pencocokan snapshot/hasil terpisah, bukan transaksi atomik dengan wallet;
perubahan akun asing ditolak sebelum entry berikut. Jangan menyamakan patch
freshness dengan jaminan uptime24jam atau keuntungan.

Setelah pengguna mengaktifkan sesi berikut, periksa umur account/quote/order,
latency query, alasan Stop, interval entry→SL, receipt/ledger dan kesinambungan
antartrade. Tetap gunakan batas pengguna yang tersimpan; tidak menaikkan risiko
untuk memaksa entry. Satu trade selesai boleh dilanjutkan sinyal baru setelah
accounting, selama belum Stop/deadline/batas rugi/maksimum transaksi. Sesi lama
selesai karena Stop data usang, bukan aturan selesai setelah satu trade.
