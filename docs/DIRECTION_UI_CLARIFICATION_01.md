# Klarifikasi arah otomatis — 16 September 2026

## Lingkup sebelum perubahan

Pengguna menegaskan strategi harus memilih Long atau Short secara otomatis
sesuai kondisi pasar, bukan arah tetap yang dipilih pengguna. Form saat ini
adalah kalkulator satu posisi tetapi berjudul "Rancang batas sesi", sehingga
memberi kesan sebagai pengaturan strategi live.

- Satu patch UI Rust dan dokumentasi, maksimal 15 menit aktif.
- Jelaskan pilihan Long/Short, SL/TP dan budget sebagai input contoh satu posisi.
- Catat desain arah otomatis: Long/Short/menunggu, penanganan konflik sinyal
  dan satu posisi yang dikelola sebelum perpindahan arah.
- Tidak menambahkan strategi baru, tombol Auto semu, sesi paper atau aktivasi live.
- Verifikasi suite/pipeline desktop, build/install, serta versi binary.
- Bukti: `Workspace/outputs/binance-direction-ui-20260916`.

Perubahan label bukan penyelesaian supervisor, strategi campuran atau trading
otonom live. Nyatakan batas itu pada hasil akhir.

## Hasil

**TERIMA patch klarifikasi UI0.5.1**, selesai dalam batas15menit. Form berjudul
"Kalkulator satu posisi — contoh manual", selector "Arah contoh", label budget/
risiko menyebut posisi. Desain Long/Short/menunggu otomatis dan SL/TP strategi
dijelaskan dengan status integrasi live yang belum selesai.

67 tes desktop lulus, 1 public-network ignored; fmt/Clippy/doc/release/audit489
lulus. Binary terpasang dan jendela baru dibuka tanpa menutup input versi lama.
Capture native offline1024×720 diperiksa. Kode margin/protokol/transport/outbox/
flow/strategi tetap cocok checksum0.5.0; tidak ada aktivasi trading atau paper.

[Bukti](../../../outputs/binance-direction-ui-20260916/DECISION.md).
