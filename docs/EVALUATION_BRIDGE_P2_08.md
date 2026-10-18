# P2 — jembatan evaluasi Rust, kandidat 0.6.8

## Kontrak putaran — 20 September 2026

Pengguna meminta melanjutkan sampai siap live, setelah penjelasan bahwa P2 dan
integrasi Hermes dapat diuji dari sesi arsip tanpa membuka sesi live baru.
Putaran ini menyelesaikan exporter/validator lokal, pengujian keamanan data,
dan persiapan rilis. Panggilan provider membutuhkan pilihan model, izin data
keluar dan plafon biaya; pertanyaan sudah disampaikan. Tidak mengambil key dari
konfigurasi aplikasi lain, tidak mengaktifkan trading, tidak menaikkan risiko.

Satu schema v1, satu sesi acuan selesai, fixture positif/negatif; checkpoint
maksimal dua jam kerja aktif. Rust/pin existing. `rustix=1.1.4` yang sudah
terkunci transitif dijadikan dependency langsung (`fs`) untuk `openat2` tanpa
unsafe custom: kernel menolak symlink seluruh komponen path dan FIFO tidak
menggantung pembacaan. Tidak memperbarui versi dependency atau toolchain.
Implementasi dalam
crate desktop agar dapat memakai tipe arsip tanpa menyalin formula trading.
CLI evaluasi berhenti sebelum membuka GUI, KWallet, transport atau worker live.
Scope awal: sesi tersimpan terakhir yang sudah final; sesi aktif/incomplete
ditolak secara eksplisit. Tidak membuat seolah seluruh histori telah direkam.

Paket memakai allowlist bertipe, angka Decimal eksak, ID lokal, kualitas bukti,
checksum dan batas ukuran. Teks alasan bebas dan identitas exchange tidak
diteruskan. Validator output tidak mengeksekusi proposal atau mengubah setting.
Keterbatasan log lama ditandai, bukan direkonstruksi menjadi fakta.

Verifikasi: fixture kebocoran sintetis, data rusak/asing, mismatch total, ID
ganda, symlink/traversal, lock/aktif, izin file, manifest parsial dan serialisasi;
pipeline desktop/engine. Ekspor sesi acuan harus cocok ledger dan tidak mengubah
file sumber. Probe GET maksimal 10 menit dan dua setup jika mencapai tahap
kesiapan rilis; bukan uji order/24 jam/profit.

Artefak: `~/Workspace/outputs/binance-hermes-p2-20260920-q1ZstL`.
Source awal dan pin disalin ke `before`; state asli hanya diperiksa/hashing.
Worktree lama pengguna dipertahankan. Deployment hanya setelah akun idle/flat,
lock dan binary terverifikasi. Hermes tidak menjadi otoritas order.

## Hasil

**P2 diterima sebagai komponen lokal teruji; integrasi Hermes/live-ready belum
selesai dan menunggu pilihan provider/biaya/izin data.**
Source/build kandidat0.6.8; binary/proses terpasang tetap0.6.6. Tidak memasang
Hermes, mengambil kredensial model, menjalankan model atau mengaktifkan order.

### Implementasi

- CLI `evaluation export/validate/validate-proposal` berjalan sebelum GUI,
  KWallet dan transport. Parsing melarang dicampur dengan flag sesi/probe.
- Snapshot read-only dari sesi final terbaru di bawah lock existing. Tidak
  memakai `Store::open`, tidak mengubah source/lock/config. Decoder hash existing
  dan tipe arsip dipakai ulang; perubahan store hanya visibility + ekstraksi
  decoder, tanpa perubahan format state atau aturan pembukuan.
- Allowlist DTO, pseudonim lokal, Decimal kanonik, total eksak, verifikasi
  relasi dan unique fill/funding IDs sebelum IDs dibuang. Tidak ada wallet,
  binding, client/exchange ID, free-text alasan atau jawaban laporan acuan.
- Paket 0700/0600, jumlah/ukuran terbatas, no-overwrite, manifest terakhir
  setelah fsync. Rehash tidak menyembunyikan mismatch total/schema/kelengkapan.
  Kernel `openat2` menolak symlink termasuk ancestor; nonblocking file open dan
  inode/metadata checks menolak perubahan file yang teramati. Unsupported kernel
  gagal tertutup pada CLI evaluasi; tidak memengaruhi worker trading.
- Proposal hanya struktur data untuk review: value/fact reference dibandingkan
  sumber, missing flags wajib, candidate enum terbatas, tanpa risk override,
  executable, path runtime atau auto-apply. Deklarasi usage bukan meter biaya.
- Profil v1 khusus arsip final. Full decisions/health/price path/raw fills
  memang belum tersedia dari sumber ini dan ditandai missing, bukan dibuat nol.
  [Kontrak executable](EVALUATION_PACKET_V1.md),
  [draft prompt pilot yang belum dijalankan](HERMES_EVALUATION_PROMPT_V1.md).

### Verifikasi lokal

**166 tes desktop PASS, 2 network ignored; 36 tes engine PASS.** Tambahan16tes
P2 mencakup angka string, ID/relasi, source/ledger mismatch, canary free-text,
lock/aktif/hash rusak, unknown fields, duplicate keys, symlink/hardlink/izin,
path traversal, oversize, no-overwrite, metadata legacy/baru, no-trade session,
reproducibility, injection gagal tulis di setiap5titik, proposal fiktif/risk/code,
dan konflik CLI. Gagal tulis memakai fixture IO, bukan mengisi disk host.
Seluruh regresi order/SL/TP/Stop/uncertain/restart/accounting existing tetap PASS.

`rust-workbench check` kedua crate PASS: fmt, Clippy all-targets/features tanpa
warning, test, doc, release, audit489/87 dependency (1251advisories saat run).
Tidak ada versi dependency atau Rust yang diperbarui. Lockfile berubah hanya
versi aplikasi0.6.7→0.6.8 dan direct edge ke rustix1.1.4 yang sudah ada.
Doctor baseline52pemeriksaan PASS. Whitespace seluruh worktree lama mengandung
CRLF pre-existing di Python/config; tidak dinormalisasi atau diubah. Pemeriksaan
file terkait putaran dilakukan terpisah, bukan mengklaim seluruh repo bersih.

Ekspor **sesi acuan asli** menghasilkan3trade; gross0.14079999, fee0.48235739,
funding aktual0, net−0.34155740USDT. Durasi11.645.332ms (194m5,332s) dari300m;
early-stop5069ms terhadap5000ms dipetakan hanya dari exact legacy template.
Alasan/outcome exit tiga trade tetap missing. Nilai sesuai laporan audit;
tidak ada data jawaban audit yang disuplai ke suatu model.

Paket release [session-packet](../../../outputs/binance-hermes-p2-20260920-q1ZstL/session-packet)
lulus validator; manifestSHA256
`4f077a6b9a5edcbfb59d1a3624d0d5d20c05575188f41bdd1116a424378ed4ae`.
Gitleaks pada paket:0temuan (tambahan, bukan pengganti allowlist/canary tests).
Ekspor ulang sukses dalam user/network namespace tanpa koneksi jaringan dan
environment kosong; ini membuktikan jalur exporter tidak perlu credential
environment/network, **bukan uji isolasi Hermes yang belum terpasang**.

Binary kandidatSHA256:
`42fa3ee61498bf5db853f91caf5b3db86c17d702add4bdbaa19f869f430616ad`.
Salinan kandidat di folder artefak. Modul strategi/planner/margin/transport/
supervisor/flow/outbox/ledger/P1 dibanding backup dan tidak berubah.

### Hermes: gerbang yang belum selesai

Host mempunyai Podman5.8.4 rootless, seccomp/SELinux aktif, cgroupv2. Tidak ada
perubahan engine container, group host, firewall atau SELinux. Belum menarik
image, pin/memasang vendor, menjalankan isolasi Hermes atau menyambung provider.
Ketersediaan tool bukan kelulusan boundary/evaluator.

Dokumentasi upstream dicek ulang. [Security Hermes](https://hermes-agent.nousresearch.com/docs/user-guide/security)
menegaskan guard file bukan sandbox proses; kemampuan adopsi login CLI juga
perlu dinonaktifkan dan auth host tidak boleh di-mount. Isolasi penuh mengikuti
[panduan Docker](https://hermes-agent.nousresearch.com/docs/user-guide/docker),
bukan sekadar backend terminal dalam container. Pin/config yang benar harus
diperiksa lagi terhadap versi vendor yang benar-benar dipasang pada P3.

Pertanyaan provider/model, izin kirim paket tersanitasi, serta plafon biaya
disampaikan kepada pengguna. Belum ada jawaban saat implementasi ini. Tidak
menganggap autentikasi agent coding atau izin pengembangan sebagai izin memakai
layanan model lain. Setelah pilihan tersedia: pin vendor + install terisolasi,
proxy/meter batas budget, uji akses terlarang dan pilot arsip (maks2run sesuai
kontrak07), review hasil, baru keputusan deploy idle. Tidak butuh sesi live baru.

Tidak melonggarkan filter/SL/TP/risiko untuk memaksa entry. Belum klaim bahwa
Hermes siap, strategi profit, atau mesin lolos penerimaan24jam.

### Verifikasi akhir jaringan dan state

Probe kandidat360detik **PASS/exit0**:92snapshot sehat,0failure,1rollover,
0revision/rewarm/held. Umur akun maksimum4082ms, gap sehat maksimum7313ms.
Satu candle confirmed_age_ms11689; saat evaluasi sudah tidak masuk jendela
15detik, sehingga fresh_decision_bars=0 dan signal_bars=0. Tidak menyebut probe
ini bukti kesempatan entry segar atau batch order saat posisi aktif.

Setup invocation pertama **FAIL/exit1**: `exchangeInfo` gagal setelah4004ms,
pesan `Resolusi DNS publik gagal`. Source menunjukkan request ke resolver
publik gagal; detail transportnya tidak tersimpan, sehingga belum membedakan
DNS lokal/TLS/connect/timeout sebagai penyebab eksklusif. Tidak mengganti DNS
sistem, mematikan TLS atau melonggarkan guard. Invocation kedua **PASS/exit0**,
dua snapshot; route DNS publik pulih. Ini bukan dua invocation PASS atau bukti
gangguan permanen sudah diperbaiki. Error pertama tetap disimpan.

Dua snapshot terakhir sekitar19:04:37–19:04:44WIB: akun kompatibel/flat sesuai
preflight, ATR0.0816% <0.10%; kedua planner aktual juga tertahan TP cap6ATR.
Contoh hipotetis layak bersyarat bukan entry/profit. Batas300menit/margin100/
risk1/notional200/leverage5/fundingreserve0.1/rugi3per-sesi/5harian/maks5trade
tidak diubah. Tidak ada aktivasi paper/live, POST/DELETE atau perubahan akun.

Sesudah ekspor/probe/kedua setup, hash seluruh10file JSONstate/outbox/ledger
dan binary terpasang identik baseline. Service066 tetapPID74909 aktif, versi
0.6.6. Candidate0.6.8 hanya build dan salinan artefak; tidak restart/deploy.
161tautan lokal dokumentasi diperiksa tanpa target hilang sebelum penambahan
dua tautan setup; pemeriksaan whitespace file terkait PASS. Warning CRLF di
worktree lain tidak disentuh.

Keputusan akhir putaran: P2 software diterima. Permintaan menyeluruh "siap live
dengan Hermes" **belum tercapai**. Menunggu provider/model + plafonbiaya + izin
data untuk P3; sesudah pilot lolos, readiness jaringan/akun perlu diperiksa ulang
sebelum deployment idle. Tidak memulai sesi trading sebagai pengganti pengujian.

### Log

- [Pipeline desktop final](../../../outputs/binance-hermes-p2-20260920-q1ZstL/desktop-check-final.log)
- [Pipeline engine](../../../outputs/binance-hermes-p2-20260920-q1ZstL/engine-check.log)
- [Exporter tanpa network/env](../../../outputs/binance-hermes-p2-20260920-q1ZstL/export-no-network.log)
- [Gitleaks paket](../../../outputs/binance-hermes-p2-20260920-q1ZstL/package-gitleaks.log)
- [Probe GET360detik](../../../outputs/binance-hermes-p2-20260920-q1ZstL/probe-360.log)
- [Setup pertama FAIL](../../../outputs/binance-hermes-p2-20260920-q1ZstL/setup-get-1.log)
- [Setup kedua PASS](../../../outputs/binance-hermes-p2-20260920-q1ZstL/setup-get-2.log)
