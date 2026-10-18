# Putaran desktop 01 — aplikasi Linux native

Mulai 16 September 2026. Pengguna mengarahkan pekerjaan: bangun aplikasi UI
yang terpasang dan berjalan di Linux, tanpa web lokal. Rencana dashboard HTTP
dihentikan sebelum implementasi; probe publik sebelumnya tidak menjadi feed aktif.

## Lingkup dan batas

- Satu aplikasi `Binance Desk`, Rust + egui/eframe, jendela native Wayland/X11.
- Ruang kerja dengan chart/panel pasar belum terhubung; halaman simulasi interaktif
  menggunakan library `engine-rs`, posisi/order, risiko, jurnal dan pengaturan UI.
- Demo hanya fixture sintetis. Nol backtest/kandidat strategi baru, nol API/account
  adapter, nol transaksi. Tidak ada listener HTTP, browser/webview atau daemon feed.
- Instalasi user-local binary, ikon dan launcher menu aplikasi; tidak perlu root.
- Anggaran satu aplikasi/satu fixture, maksimal 75 menit aktif sebelum evaluasi.

## Penerimaan

- Build/test/fmt/Clippy/doc/release/audit Cargo sesuai dependency yang terpin.
- Uji replay/state UI, kontrol dan ekspor yang memetakan engine sebenarnya;
  tidak menampilkan data dummy sebagai harga/saldo/P&L live.
- Binary release membuka jendela pada Fedora; tampilan utama, demo, ukuran minimum
  dan navigasi diperiksa melalui render/screenshot aktual sejauh tool tersedia.
- Desktop entry valid; pengguna dapat membuka dari menu dan command `binance-desk`.
- Hasil, bukti dan keterbatasan tersimpan di Workspace/outputs; status diperbarui.

LANJUT berarti shell aplikasi native dapat dipakai, bukan mesin live/profit siap.
Jika ada kegagalan build/runtime nyata, laporkan dan perbaiki dalam lingkup ini;
jangan beralih diam-diam ke aplikasi web.

Referensi: [eframe 0.36.2](https://docs.rs/eframe/0.36.2/eframe/),
[App native](https://docs.rs/eframe/0.36.2/eframe/trait.App.html).

## Hasil — LANJUT untuk UI native

Aplikasi `desktop-rs` selesai dalam scope satu app/satu fixture, di bawah anggaran
75 menit. Binary 15 MiB terpasang user-local, launcher/ikon valid dan cache KDE
diperbarui. Aplikasi native berhasil dirender dan dibuka pada Fedora Wayland.

6 tes lulus; format/Clippy/doc/release/audit Cargo (339 dependency) lulus; doctor
lingkungan 52 pemeriksaan lulus. Render workspace/simulation/journal/settings
dan ukuran minimum diperiksa dari screenshot nyata. Kontrol navigasi/demo/export/
settings diperiksa dengan pointer events ke widget egui, bukan hanya fungsi model.

Koreksi selama implementasi: API style/label egui 0.36 disesuaikan, lint nested-if
dirapikan, glyph yang tidak didukung diganti, label status dipaskan, tinggi chart
dibuat responsif agar transport demo tampak di jendela minimum. Tampilan awal
dan log awal dipertahankan. `gtk-launch` mengembalikan 0 tetapi proses tidak
bertahan dari tool; launcher aplikasi langsung dijalankan melalui user unit
sementara tanpa autostart agar jendela tetap dapat dilihat pengguna.

LANJUT hanya untuk shell desktop dan demo interaktif. Integrasi akun/feed,
strategi, order nyata, distribusi paket/distro lain belum tersedia. Tidak ada
riset/bukti performa baru, perubahan akun atau akses kredensial. Engine lama,
Python/web dan riwayat gagal tetap dipertahankan.

[Bukti dan reproduksi](../../../outputs/binance-desktop-round01-20260916/DECISION.md).
