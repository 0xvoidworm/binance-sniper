# P1 — keandalan snapshot dan bukti exit, kandidat 0.6.7

## Kontrak sebelum patch — 20 September 2026

Permintaan pengguna: mulai langkah berikutnya dari rancangan Hermes07.
Scope putaran: jendela snapshot order/akun, history di luar jalur pengelolaan
posisi, status settlement saat Stop, serta alasan exit terstruktur. Anggaran
satu kandidat desain dan maksimal dua jam kerja aktif sebelum checkpoint.
Rust/pin/dependency existing; bukan penambahan Hermes atau perubahan strategi.

Tetap guard data5detik, candle terkonfirmasi dua kali, tanpa catch-up entry,
SL/TP/ownership/outbox/no-duplicate, ledger dan batas risiko existing. Uji
regresi offline sebelum/sesudah, seluruh test desktop/engine, fmt/Clippy/doc/
release dan pemeriksaan dependency sesuai ketersediaan tool. Hasil negatif
dan batas cakupan dicatat; tidak menyebut fixture sebagai acceptance24jam.

Tidak install/deploy/restart aplikasi, tidak mengaktifkan live/paper, tidak
mengirim POST/DELETE/ubah akun, tidak membaca key ke output. Binary terpasang
0.6.6 tetap. Source hasil patch disiapkan terpisah untuk review/rilis berikut.
Jika diperlukan probe GET kandidat, lingkup dan batasnya dicatat dahulu;
pengujian awal memakai exchange lokal tanpa jaringan/kredensial.

Backup source awal dan bukti pengujian:
`~/Workspace/outputs/binance-reliability-p1-20260920-owA0xD`.
Worktree sudah memiliki perubahan pengguna; hanya file P1 terkait disentuh.

## Status pengerjaan

**P1 selesai sebagai kandidat software0.6.7; belum deploy/acceptance live.**
Regresi dan implementasi selesai; pipeline software PASS. Kontrak probe
tambahan sebelum dijalankan: satu `--probe-auto --probe-auto-seconds 360`
kandidat0.6.7, maksimal6menit GET-only, dua setup GET sesudahnya. Probe memakai
assembler snapshot/sampler yang sama dengan live tetapi transport ReadOnly,
tanpa Engine/order dispatch atau penulisan state. Tidak membuktikan snapshot
order posisi aktif karena akun yang diharapkan adalah flat; hasil yang berbeda
dilaporkan, tidak diperbaiki dengan transaksi atau memperpanjang sesi.

Hasil: **probe360detik PASS/exit0**,97snapshot,0error,0rewarm/held,1candle
terkonfirmasi pada usia10118ms,1keputusan/0sinyal. Umur akun maksimum3359ms;
gap sehat maksimum10955ms. Ada persiapan DNS/clock lambat hingga4634ms pada
log, tetapi data snapshot tetap lolos guard; bukan bukti batch order posisi
aktif karena probe tidak mengelola intent/order. Strategi menunggu ketika
ATR0.085% di bawah filter, tanpa memaksa entry atau memperpanjang15detik.

Dua invocation `--check-setup` sesudah probe PASS/exit0, masing-masing2snapshot,
akun kompatibel/flat tanpa order awal. ATR aktual0.0851% masih di bawah0.10%;
planner aktual Long/Short juga tertahan cap6ATR. PASS koneksi/setup hipotetis
bukan sinyal entry. Config100/1/200/5, funding0.1, rugi3/sesi/5harian dan maks5
trade tetap. Tidak mengubah batas untuk membuat snapshot ini bisa entry.

Hash10file JSON sesi/outbox/ledger dan binary terpasang identik sebelum/sesudah
pemeriksaan jaringan. Engine-rs dan config juga cocok baseline sebelum desain.
Binary terpasang/proses tetap0.6.6; tidak install/deploy/restart atau transaksi.

## Perubahan yang diimplementasikan

1. **History asinkron khusus publik.** Satu worker dengan antrean request/result
   masing-masing satu slot, satu request outstanding, timeout10detik. Loop
   pengelolaan posisi tidak menunggu GET klines atau dua konfirmasi candle.
   Hanya baseline terkonfirmasi diteruskan; dua read identik≥2detik, revisi,
   jendela sinyal15detik dan recovery-cutoff existing tetap berlaku. Error
   worker diteruskan ke pause-flat/stop-close-only existing, bukan disembunyikan.
   Stop/recovery membuang worker dan hasil terlambat. Worker tidak memegang key.
2. **Satu persiapan DNS/waktu privat untuk satu siklus GET.** Reference di luar
   jendela bukti; kemudian clock/route lease≤5detik → GET order → GET akun/quote.
   Batch akun tidak melakukan persiapan privat kedua. Timestamp order/akun/quote
   tetap waktu asli; lease bukan bukti freshness data. Lease usang ditolak,
   bukan diperpanjang. Mutasi tidak memakai lease; tiap awal/akhir siklus
   membersihkannya, termasuk siklus setelah pembatalan Stop/deadline.
3. **Metadata tidak direfresh di tengah bukti order.** `fetch_prepared` menolak
   reference yang kedaluwarsa, bukan mengambil metadata/fee lambat di sana.
   Tidak ada retry POST atau reacquire tanpa batas. REST tetap tidak atomik.
4. **Guard quote dipasang di pemeriksaan frame awal dan sebelum entry setelah
   pemeriksaan fee.** Quote stale/future tidak boleh diloloskan hanya karena
   akun segar. Batas5detik tidak dinaikkan untuk mengakomodasi kasus5069ms.
5. **Settlement Stop langsung Finished** setelah cleanup dan dua ledger cocok.
   Intent terminal tanpa exposure saat Stop juga tidak kembali Watching.
   Keputusan melanjutkan sesi tanpa Stop tetap mengikuti batas existing.
6. **Alasan dan outcome exit berbeda.** `stop_code` sesi, `exit_decision`
   (primary, semua sebab yang diamati, waktu keputusan, versi aturan) dan
   `exit_outcome` disimpan sebagai metadata opsional. Outcome SL/TP/reduce-only/
   mixed/unknown ditentukan dari order exit yang punya fill nonzero dan telah
   diverifikasi ledger. Algo expired/triggered saja tidak membuktikan fill.
   Exit internal lifecycle yang belum dirinci dilabeli `lifecycle_close`, tidak
   ditebak menjadi timeout. Timestamp keputusan bukan waktu klik UI/fill.
7. **Korelasi diagnosis.** `LIVE_SNAPSHOT_CYCLE` memakai ID siklus dan waktu
   selesai tahap reference/clock/order/frame relatif terhadap awal siklus;
   nol berarti tahap belum selesai atau <1ms. `LIVE_FRAME_HEALTH` dan error
   pemeriksaan membawa ID yang sama. Tidak merekam raw respons/key/wallet.

File inti: `live`, `session_input`, `readiness`, `transport`, `supervisor`,
`session_store`, `ledger`, modul baru `exit`, dan regression tests. Planner,
ensemble, order protocol/flow/outbox, config risiko serta engine-rs tidak
diubah. Cargo.toml/lock hanya versi paket0.6.6→0.6.7; Rust1.98.1 dan seluruh
dependency tetap. Hermes/exporter P2 belum diimplementasikan.

## Bukti pengujian software

- Dua regresi **FAIL pada kode lama, PASS sesudah patch**: pemeriksaan awal
  quote5069ms dan Watching sesudah settlement Stop. Log kegagalan dipertahankan.
- **150 tes desktop PASS, 2 network ignored; 36 tes engine PASS.** Total13tes
  baru mencakup batas lease4999/5000/5001/5069ms, overflow/cooldown, history
  menggantung/timeout/abort, dua konfirmasi/revisi, proteksi ketika history
  pending, entry setelah konfirmasi dan penolakan late/Stop/deadline, prioritas
  sebab, SL/TP/reduce-only/holding/funding, hash state lama dan crash settlement.
- Waktu virtual24jam melintasi hari UTC dan selesai pada deadline asli tanpa
  order. Ini uji deadline, bukan24jam market/posisi aktif. Seluruh regresi
  partial/uncertain/restart/no-duplicate/accounting existing tetap lulus.
- `rust-workbench check` desktop dan engine PASS: fmt, Clippy semua target/
  fitur tanpa warning, test, doc, release dan audit489/87dependency. Tidak ada
  dependency baru. Dua test jaringan ignored bukan hasil lulus jaringan.
- Binary kandidat: `desktop-rs/target/release/binance-desk`, `--version`0.6.7.
  SHA256 `ae0583211aec7a20b16aed8d7cce7df8616a4dafeee5d2e84b2f3205398f930d`.

## Kompatibilitas dan batas penerimaan

Field baru menggunakan default absent + skip-serialization; hash catatan lama
tanpa metadata tetap valid. Arsip lama yang sudah ditulis sebelum crash dapat
dipakai menyelesaikan ringkasan jika seluruh isi finansial/identitas cocok;
metadata yang dulu tidak ada tidak ditulis balik atau dikarang. Perbedaan PnL
tetap ditolak. Arsip baru mencatat diagnosis untuk evaluasi Hermes berikutnya.

Ini kompatibilitas **membaca data lama dengan kandidat baru**, bukan jaminan
binary lama bisa membaca semua hash state baru. Rollback setelah kandidat
pernah dipakai memerlukan pemeriksaan/migrasi metadata; jangan mengembalikan
state/outbox lama di atas transaksi exchange yang lebih baru.

Build belum dipasang/restart. Binary0.6.6 dan proses74909 tetap saat pemeriksaan.
Belum ada penerimaan order/latency posisi aktif0.6.7, keuntungan strategi atau
stabilitas24jam. Data yang benar-benar stale, metadata expired, kegagalan
history atau jaringan tetap dapat menghentikan sesi sesuai pengaman.

## Bukti tersimpan

- [Regresi sebelum patch](../../../outputs/binance-reliability-p1-20260920-owA0xD/regression-before.log)
- [Tes P1](../../../outputs/binance-reliability-p1-20260920-owA0xD/p1-tests-final.log)
- [Pipeline desktop final](../../../outputs/binance-reliability-p1-20260920-owA0xD/desktop-check-final.log)
- [Pipeline engine](../../../outputs/binance-reliability-p1-20260920-owA0xD/engine-check.log)
- [Probe GET360detik](../../../outputs/binance-reliability-p1-20260920-owA0xD/probe-360.log)
- [Setup pertama](../../../outputs/binance-reliability-p1-20260920-owA0xD/setup-get-1.log), [kedua](../../../outputs/binance-reliability-p1-20260920-owA0xD/setup-get-2.log)
- [Binary kandidat, belum dipasang](../../../outputs/binance-reliability-p1-20260920-owA0xD/binance-desk-0.6.7-candidate)

Keputusan: P1 diterima untuk handoff software teruji. Deployment idle dengan
verifikasi ulang akun/state dan instruksi pengguna merupakan langkah terpisah.
Tahap kode berikutnya P2 exporter/validator untuk Hermes; tidak mengaktifkan
P3/model atau eksperimen live otomatis dari hasil probe ini.
