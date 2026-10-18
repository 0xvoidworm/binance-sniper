# Pemulihan transport terbatas — 0.6.10

21 September 2026 WIB. Pengguna meminta langsung patch dan pembaruan setelah
[audit sesi 34 menit](../../../outputs/binance-session-early-20260921-IKfoPf/LAPORAN.md).
Scope satu kandidat, checkpoint dua jam, probe GET maksimal 600 detik. Tidak
mengaktifkan sesi, mengirim order, mengubah akun/jaringan, menaikkan risiko,
atau memasang model/dependency/toolchain. Hermes tetap ditunda pengguna.

## Penyebab yang diperbaiki

Sesi produksi 0.6.9 selesai 34 menit 22,750 detik dari target 180 menit:
persiapan route/time gagal setelah 15099 ms. Frame sehat berikutnya muncul
4386 ms setelah error, atau 21521 ms setelah frame sehat sebelumnya. Bukan
bukti outage hanya empat detik, bukan crash atau batch akun serial lagi.
Satu Long 0.002 BTC: gross +0.08560000, fee 0.16196919, funding 0, net
−0.07636919 USDT. Guard lama melatch Stop seluruh sesi ketika ada intent.
Koneksi membaik tidak membatalkan Stop tersebut; trade ditutup lalu sesi final.

Reproduksi pada supervisor produksi + exchange sintetis gagal sebelum patch
dan lulus setelah patch. Kegagalan persiapan sebelumnya juga semuanya
dipetakan ke Network; klasifikasi itu tidak cukup aman untuk auto-recovery.
Letak gangguan fisik (Wi-Fi/ISP/resolver/Binance) belum terbukti.

## Kebijakan baru

| Kondisi | Tindakan |
|---|---|
| Transport bertipe sementara, belum ada intent | Pause lama tetap; lewatkan candle pemulihan, tanpa membuka posisi |
| Transport bertipe sementara, ada intent | Tutup/reconcile intent; jangan latch Stop seluruh sesi selama anggaran pemulihan tersedia |
| Posisi/order kosong, ledger cocok dua kali | Mulai pemeriksaan sehat 60 detik; tidak ada entry selama pemeriksaan |
| Bukti sehat lengkap dan candle terbaru terkonfirmasi | Kembali menunggu, konsumsi candle pemulihan; entry hanya pada candle baru yang lolos seluruh aturan |
| Episode ketiga dengan intent | Stop terminal, tutup/accounting; tidak kembali menunggu |
| Stop pengguna/jendela, deadline, batas uang/trade, auth/schema/integritas/proteksi tidak pasti | Stop terminal selalu menang; tidak dibatalkan oleh koneksi sehat |
| Restart proses | Tidak auto-resume; tombol pemulihan lama tetap close-only |

Maksimal **dua episode pemulihan dengan intent per sesi**. Error berulang
selama episode yang sama tidak menambah hitungan, tetapi mengulang bukti
sehat. Ini batas rekayasa konservatif, bukan angka optimum strategi.

Health memakai waktu monotonic dan waktu server yang sama-sama maju minimal
60 detik, minimal dua sampel, jarak antarsampel maksimal 10 detik pada kedua
jam. Replay/clock jump/gap mengulang proof; proof tidak disimpan ke disk.
Freshness akun, quote dan bukti order tetap maksimal 5 detik. Wallet, mode
akun, margin/leverage, posisi/order asing, deadline dan risiko diperiksa tiap
frame. Revalidasi history saat flat me-reset proof; dua pembacaan candle
konsisten tetap wajib. Sisa risiko dan biaya juga diuji planner sebelum entry.

Penutupan tetap menunggu bukti baru yang aman; tidak ada klaim dapat mengirim
exit saat koneksi mati. Order proteksi yang sudah diakui exchange tidak
dicabut hanya karena masuk pemulihan. Outbox tetap menyimpan intent sebelum
POST, hasil pengiriman tak pasti diperiksa GET dan tidak dikirim ulang buta.
Gangguan baca ledger tidak menghapus fill/fee atau melewati accounting.
Ketidakpastian proteksi/order tetap terminal, bukan alasan melonggarkan guard.

## Perubahan implementasi

- `recovery.rs`: tipe state episode dan proof kesehatan ephemeral; allowlist
  error transport bertipe, bukan pencocokan string error.
- `supervisor.rs`/`live.rs`: bedakan close trade dari Stop sesi, mode close-only
  selama ada intent, lalu history/snapshot sehat tanpa entry. Exit trade
  tetap mencatat DataFailure dan outcome berdasarkan ledger.
- Transport: pertahankan kategori persiapan auth/rate/clock/schema, tolak
  menganggap HTTP baca 401/403/400/404 sebagai gangguan yang boleh pulih.
  HTTP 5xx/408 dan timeout eksekusi mutation tetap Unknown; aturan POST
  tidak dilonggarkan. Selaras dengan [panduan resmi Binance](https://developers.binance.com/en/docs/products/derivatives-trading-usds-futures/general-info).
- Persiapan privat time/DNS/fallback GET dibatasi total 20 detik, maksimum
  empat alamat cadangan seperti sebelumnya; timeout membatalkan future.
  Log tahap/attempt/elapsed/timeout/connect tanpa key/URL bertanda tangan/body.
  Timeout history tetap 10 detik, batas siklus 40 detik tetap; bukan retry POST.
- UI membedakan pemulihan trade dari penghentian sesi, menampilkan anggaran
  episode dan ketentuan baru. Capture menyamarkan metadata privat.
- Metadata opsional state/ekspor untuk sesi baru, build 0.6.10. Sesi lama
  tidak mendapat recovery authority/counter buatan. Validator menerima paket
  0.6.8/0.6.9/0.6.10, menolak metadata aktif pada paket final atau versi lama.

Tidak mengganti ensemble-2-cost-aware, ATR 0.10–3%, SL 2 ATR, TP maksimum
6 ATR, net RR minimum 1.1, fee, sizing, leverage, batas uang atau durasi.
Reentry setelah outage dapat menambah biaya dibanding sesi yang langsung
berakhir; dua episode, batas trade/uang dan cooldown membatasi, bukan
menghilangkan risiko tersebut. Tidak mengklaim patch meningkatkan profit.

## Verifikasi dan penerimaan

Tes menggunakan supervisor/outbox/ledger produksi, exchange sintetis dan
waktu virtual: outage, 60 detik/candle baru, episode berulang/ketiga, Stop dan
deadline saat close/cooldown, restart/legacy, partial/residual/uncertain receipt,
ledger gagal, proteksi tak pasti, foreign order/wallet/leverage, data basi,
revalidasi history, metadata/ekspor/UI serta pembatalan persiapan 20 detik.

Hasil pipeline, probe, setup dan status pemasangan dicatat pada
[laporan putaran](../../../outputs/binance-recovery-010-20260921-1JfFpw/LAPORAN.md).
Tes dan GET flat bukan bukti recovery posisi nyata, edge strategi atau 24 jam.
Stale snapshot, error tak terklasifikasi dan pelanggaran keselamatan masih
dapat mengakhiri sesi lebih awal. Durasi adalah batas maksimum, bukan jaminan.
Sesi berikutnya tetap diaktifkan pengguna; batas tersimpan tidak diubah.

Rollback: salinan binary 0.6.9 disimpan pada folder bukti. Setelah state baru
0.6.10 dibuat, jangan menjalankan binary lama tanpa cek kompatibilitas/hash.
Jangan menimpa state/outbox/ledger baru dengan backup atau rollback saat live.
