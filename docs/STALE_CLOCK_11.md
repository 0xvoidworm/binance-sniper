# Data usang dan clock snapshot — 0.6.11

21 September 2026 WIB. Pengguna meminta patch setelah sesi 0.6.10 berakhir
77 menit 55,681 detik dari target 180 menit. Scope satu kandidat, dua jam,
probe GET maksimal 600 detik dan dua pemeriksaan setup. Bukan aktivasi live,
perubahan akun/jaringan, peningkatan risiko atau pemasangan Hermes/model.

## Akar masalah dan perbaikan

[Audit sebelumnya](../../../outputs/binance-session-77m-20260921-3ivGTg/LAPORAN.md)
membuktikan recovery posisi telah selesai, kemudian depth berumur 7.267 ms
memicu Stop saat flat. Kode lama menggabungkan stale/future menjadi error
generik, sementara handler 0.6.10 hanya mengizinkan Network untuk pemulihan.
GET waktu publik setelah depth/mark juga bisa membuat data yang sudah
dikumpulkan menjadi basi. Bukan timer 77 menit atau input risiko yang salah.

- `quotes`: periksa schema, timestamp future/terbalik, urutan level, spread,
  replay ID dan perubahan isi terlebih dahulu. Hanya umur yang melampaui
  5.000 ms menjadi `StaleRead` bertipe; error string tidak memberi izin pulih.
- `readiness`/`transport`: gunakan clock server monotonic dari lease GET
  yang disiapkan sebelum pembacaan order/akun/harga. Tidak ada GET `time`
  tambahan setelah depth/premiumIndex. `snapshot_now` sinkron dan tanpa
  network; lease lebih dari 5 detik dibuang (`ReadWindowExpired`), bukan
  diperpanjang. Lease kosong/waktu invalid tetap `Clock`, cooldown tetap
  diperiksa. Jalur preflight mandiri membuka/menutup lease yang sama.
- Umur account/quote dalam frame dan umur mark memakai jenis stale eksplisit;
  future timestamp tetap terminal. Batas quote/account/order 5 detik dan
  mark 10 detik tidak dinaikkan. Timestamp harga tetap timestamp exchange,
  account age tetap dihitung sejak awal batch asli, bukan cap waktu baru.
- Supervisor memperlakukan Network, StaleRead dan expired GET lease sebagai
  gangguan baca yang dapat dipulihkan. Saat flat, jeda entry lalu konsumsi
  candle pemulihan; durasi tidak diperpanjang dan kuota episode tidak terpakai.
  Saat ada intent, close-only/accounting lalu proof sehat 60 detik dengan
  gap maksimal 10 detik, maksimal dua episode, seperti kontrak 0.6.10.
  Gangguan saat cooldown mengulang proof, bukan melatch Stop.
- Stop pengguna, deadline, batas rugi/trade, restart, schema/auth/rate/future,
  error tak terklasifikasi dan ketidakpastian proteksi/order tetap konservatif.
  Bukti order usang tidak dimasukkan ke kelas data usang yang boleh pulih.
  Tidak ada retry POST buta atau order berdasarkan snapshot yang ditolak.
- UI memperjelas kebijakan stale saat flat/intent; metadata sesi baru memakai
  build 0.6.11. Validator menerima paket 0.6.8–0.6.11 sesuai versi sumber,
  menolak klaim metadata baru dari exporter lama. Arsip asli tidak diubah.

Strategi, sizing, parameter pengguna, biaya, Stop dan desain Hermes tidak
diubah. Cargo.lock hanya berubah pada versi aplikasi; dependency/toolchain
tetap. Koneksi fisik Wi-Fi/ISP/resolver belum terbukti sebagai akar spesifik.

## Verifikasi

Tes regresi actual Book parser → supervisor → fresh frame gagal sebelum
patch, kemudian lulus. Tambahan enam tes meliputi stale 7.267 ms saat flat,
entry hanya pada candle baru, stale saat intent/uncertain receipt/residual,
accounting/cooldown, gangguan lagi sesudah recovery, 200 stale flat tidak
memperpanjang deadline, expired lease, stale account/quote, future/schema/
replay, clock monotonic/batas/cooldown. Matrix Stop lama juga diberi stale.

204 tes desktop lulus, dua tes jaringan manual diabaikan; 36 tes engine
lulus. Pipeline fmt/Clippy/test/doc/release/audit kedua crate lulus. Satu tes
lama sempat gagal karena format error berubah; assertion diperbarui untuk
memeriksa kelas error bertipe dan larangan order, bukan melemahkan guard.
Log kegagalan awal dipertahankan.

[Laporan putaran dan bukti](../../../outputs/binance-stale-clock-011-20260921-mhBot6/LAPORAN.md)
menjadi acuan hasil probe, setup dan status pemasangan aktual. Uji GET tidak
menguji order produksi. Patch bukan bukti sesi 24 jam, profit, atau jaminan
selalu mencapai durasi: Stop keselamatan masih dapat mengakhiri sesi.
