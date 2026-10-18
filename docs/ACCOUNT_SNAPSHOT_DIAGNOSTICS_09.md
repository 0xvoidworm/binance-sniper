# Investigasi sesi 78 menit dan patch 0.6.9

20 September 2026. Scope: satu kandidat, maksimal dua jam kerja aktif,
uji GET maksimal 600 detik dan dua invocation setup. Pengguna mengizinkan
pemasangan/restart idle setelah tes dan akun aman. Tidak mengaktifkan sesi,
mengirim order, mengubah akun atau batas uang. Hermes/model ditunda; bukan
penghalang perbaikan inti. Tidak ada dependency/toolchain baru.

## Temuan sesi sebenarnya

Sesi produksi masih menggunakan **0.6.6**, bukan kandidat P1/P2 0.6.7/0.6.8
yang belum dipasang. Proses tetap hidup sejak 10:52:55 WIB; bukan app crash.
Session mulai 20:23:46.998, target 23:23:46.998, final 21:42:19.456 WIB:
**78 menit 32,458 detik dari 180 menit**. State final, stop aktif, intent kosong.

| Bukti | Hasil |
|---|---|
| GET /fapi/v1/symbolConfig sebelum Stop | 4310 ms |
| Umur snapshot akun/posisi saat ditolak | 5407 ms, melewati guard 5000 ms |
| Umur quote saat itu | 201 ms, masih segar |
| Alasan final | Koneksi/data gagal; snapshot account/posisi usang |
| Posisi selesai | 1 Long BTCUSDT, 0.002 BTC, pola trend pullback |
| Waktu intent / last fill | 21:35:09.301 / 21:41:58.344 WIB |
| Gross / fee / funding / net USDT | +0.1292 / 0.16110040 / 0 / **−0.03190040** |

Enam GET akun pada kode lama berjalan berurutan. Respons symbol yang lambat
ditambah latensi GET lain membuat snapshot pertama kedaluwarsa. Pada posisi
yang mungkin aktif, data failure melatch penghentian, close-only dan accounting.
Ini bukan aturan satu order selesai mengakhiri sesi, batas rugi yang habis,
atau holding 30 menit. Dari intent ke last fill sekitar 6 menit 49 detik;
intent bukan timestamp fill entry. Fill terakhir bukan timestamp final sesi.

Journal mencatat 14 evaluasi candle: 11 tertahan ATR, 2 tanpa pola, 1 Long.
Sebelum entry ada tiga error pembacaan saat flat yang sempat pulih; error
5407 ms saat posisi aktif yang memicu penghentian. Ada dua persoalan berbeda:
kesempatan sinyal memang jarang, dan jalur snapshot kurang efisien hingga
sesi harus dihentikan. Menurunkan ATR/memperbesar leverage tidak memperbaiki
penyebab penghentian ini.

Respons entry dan exit tidak cocok schema receipt, lalu berhasil dikonfirmasi
lewat GET masing-masing 202/216 ms. Tidak ada bukti resend pada alur ini.
Respons privat mentah tidak tersedia untuk diagnosis schema spesifik; jangan
mengarang penyebab atau membuat parser permisif. Guard uncertain tetap ada.
Letak latensi 4310 ms di jaringan lokal, ISP atau server Binance belum dapat
dipisahkan oleh telemetry yang tersedia.

## Perubahan dan hubungan dengan logika lain

1. Enam GET akun independen kini serentak, maksimum enam, memakai
   `tokio::try_join!`; hanya batch exact allowlist. Tidak memparalelkan
   POST/DELETE atau rekonsiliasi order. Satu waktu mulai konservatif tetap
   dipakai; tidak merestamp data atau menaikkan guard.
2. Error/cancel/deadline menjatuhkan semua future GET yang masih menunggu;
   tidak ada job snapshot terlepas. Cooldown bersama mempertahankan batas
   terpanjang, termasuk HTTP 429/418 dengan body rusak. Jeda loop minimal
   tetap dua detik. Jumlah GET per batch sama; batch lebih cepat dapat
   meningkatkan siklus per menit, bukan klaim laju total identik.
3. Log `LIVE_ACCOUNT_BATCH` berisi durasi enam endpoint tanpa key/body/saldo.
   Nol saat error dapat berarti belum selesai atau kurang dari satu ms,
   bukan bukti endpoint sukses tanpa latensi.
4. Counter bertipe untuk sesi baru: no-pattern, late, ATR, konflik, sinyal,
   funding, ekonomi, intent, recovery-skip, data failure dan safety failure.
   Lima pertama adalah cabang evaluasi candle; polling bukan keputusan.
   Signal ≠ intent ≠ fill ≠ trade selesai. Counter hanya diagnosis.
5. UI menampilkan versi runtime, Hermes tidak aktif dan panel counter.
   Versi pembuat sesi disimpan; sesi lama tetap unknown, bukan nol/versi
   terbaru. Capture audit tetap menyamarkan data sesi.
6. Paket evaluasi v1 mendapat diagnostic summary opsional. Counter dan versi
   sumber dicocokkan validator; full decision/health time series tetap
   missing. Bug validator yang hanya menerima versi exporter saat ini
   direproduksi dan diperbaiki: paket 0.6.8 tetap diterima 0.6.9, versi tidak
   dikenal ditolak. Tidak ada model/auto-apply/auto-export.

P1/P2 ikut rilis: history di luar jalur snapshot aktif, lease DNS/waktu
terbatas, Stop settlement langsung Finished, exit metadata opsional, exporter
lokal final dan validator proposal tanpa kewenangan trading. Strategi
ensemble-2, ATR 0.10–3%, SL 2 ATR, TP maksimum 6 ATR, net RR minimum 1.1,
biaya, sizing, durable outbox, dua pemeriksaan accounting dan batas pengguna
tidak diubah.

## Yang diambil dari video

[Analisis lima video](../../../outputs/ai-trading-video-review-20260920-c4xEWq/LAPORAN.md)
membedakan dashboard, simulasi/replay, Polymarket dan Futures. Yang diterapkan
adalah jejak keputusan nyata dan pemisahan analisis/risiko/eksekusi, bukan
animasi enam AI atau label win-rate. Counter dari cabang kode aktual dapat
diaudit bersama fee dan ledger. Pair arbitrage Polymarket tidak dipindahkan
menjadi janji edge BTC Futures. Tidak memasang agent/provider/model berbayar.

## Verifikasi dan batas

Reproduksi waktu virtual pada pengumpul snapshot yang sama: serial FAIL
5407 ms, serentak PASS 4598 ms. Latensi symbol 4310 ms berasal dari journal;
pembagian latensi endpoint lainnya sintetis. Ini bukan pembuktian bahwa
respons Binance berikutnya selalu selesai sebelum lima detik.

Tes tambahan mencakup cancel/deadline, error/schema/rate limit, counter pada
entry/restart/data-failure/settlement, recovery dedup, state/paket lama,
tamper counter dan batas UI/capture. Hasil pipeline, probe, pemeriksaan
setup dan pemasangan dicatat dalam laporan hasil di bawah.

REST belum atomik. Outage atau endpoint tunggal terlalu lambat tetap dapat
menghentikan sesi saat exposure aktif. Belum ada penerimaan posisi aktif
0.6.9 atau soak 24 jam. Target durasi adalah maksimum, bukan jaminan mesin
terus trading walau guard gagal. Satu trade, tes dan probe bukan bukti edge
atau profit. Hermes tetap di luar jalur order, menunggu keputusan provider,
biaya dan izin data tersendiri.

Artefak: `~/Workspace/outputs/binance-session-78m-fix-20260920-MRMfwz`.
[Hasil akhir](../../../outputs/binance-session-78m-fix-20260920-MRMfwz/LAPORAN.md).
