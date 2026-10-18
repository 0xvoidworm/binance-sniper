# Mesin forward paper 01 — 16 September 2026

## Lingkup sebelum implementasi

Pengguna meminta mesin segera berjalan dan menyatakan dana Binance khusus
proyek, menyerahkan strategi/rules. Akun read-only terhubung, tetapi engine belum
memiliki adapter order Binance, margin/funding/likuidasi atau strategi tervalidasi.
Putaran ini mengaktifkan simulasi pasar nyata di UI native, bukan transaksi uang.

- Satu implementasi Rust, maksimal 100 menit aktif; nol optimasi/backtest/trial
  strategi pembanding. Maksimal satu sesi pasar 60 menit untuk penerimaan.
- BTCUSDT USD-M, candle 5m, EMA20/50. Entry long bila EMA20>EMA50, close>EMA20
  dan menembus high bar sebelumnya; short simetris. Satu posisi. Bar historis
  hanya warmup, entry hanya pada bar baru setelah sesi dimulai. Exit SL 2×ATR14,
  TP 3×ATR14; semua dihitung desimal, dibulatkan ke filter harga bursa.
- Modal virtual 1000 USDT, risiko maksimum 2.5 USDT/entry, notional maksimum
  250 USDT, loss harian 10/total 30 USDT; tanpa leverage simulasi. Asumsi fee
  6 bps/sisi + adverse slippage 1 bp; bukan fee/alokasi/rekomendasi akun nyata.
- Adapter simulator memakai engine-rs/journal existing; fill virtual langsung
  pada bid/ask publik plus adverse slippage, tanpa klaim likuiditas/partial-fill
  realistis. Filter BTCUSDT dari exchangeInfo publik.
- Tidak memegang posisi melintasi funding: entry ditahan bila funding kurang
  dari 30 menit, exit virtual sebelum 5 menit; bila data hilang melewati batas,
  sesi ditandai tidak lengkap. Belum simulasi margin/likuidasi atau funding penuh.
- Worker terpisah dari render, GET publik saja, harga stale/gap/revisi mencegah
  entry. Tidak melewati sinyal masa outage. Pause/stop menghentikan entry; posisi
  virtual ditutup pada quote segar berikutnya. Sesi berakhir otomatis 60 menit.
- Halaman Mesin paper: aktif/menunggu/stop/error, alasan sinyal/gate, modal/P&L
  virtual, posisi/proteksi, riwayat keputusan dan lokasi jurnal. Akun/keys tidak
  menjadi input sizing atau isi log. Tidak ada strategi campuran tanpa bukti.
- Bukti di `Workspace/outputs/binance-forward-paper-round01-20260916`.

## Penerimaan dan titik berhenti

1. Uji causal warmup/bar baru, EMA/ATR/sisi, dedup/gap/stale, filter, sizing,
   fill/TP/SL/stop, lifecycle jurnal dan batas sesi.
2. Pipeline Rust dan UI native terverifikasi; mode paper dipasang dan sesi
   publik berjalan, state/jurnal bisa diperiksa. Trade hanya bila sinyal terjadi;
   jangan memaksakan entry untuk menghasilkan screenshot/trade count.
3. LANJUT hanya untuk operasi simulasi, bukan bukti profit atau kelayakan live.
   Catat hasil/limit sebenarnya. Tidak mengirim order, transfer atau mengubah
   pengaturan akun/leverage. Tidak menghapus hasil negatif lama.

## Hasil putaran

**LANJUT untuk operasi paper.** Versi 0.4.0 terpasang, 46 tes desktop dan 36 tes
engine lulus. Pipeline penuh/audit489 dependency lulus. Probe harga publik mencapai
≥3 pembaruan dan berhenti flat; replay jurnal sesuai state. Batas metadata publik
exchangeInfo dinaikkan dari1MiB ke4MiB setelah probe awal gagal pada respons nyata
lebih dari1MiB. Endpoint lain tetap1MiB. Tidak ada perubahan parameter strategi.

Satu sesi penerimaan dimulai19:47WIB, otomatis berakhir20:47WIB. Candle selesai
19:50 diproses19:50:03WIB; aturan memilih menunggu, belum ada trade virtual.
Snapshot penerimaan memiliki72 pembaruan/73record; replay prefix jurnal cocok.
UI native1400×900 dan1024×720 diperiksa lewat laporan tersimpan dari sesi tersebut.
Ini bukti runtime awal, belum hasil sesi60menit atau validasi performa strategi.
[Bukti lengkap](../../../outputs/binance-forward-paper-round01-20260916/DECISION.md).
