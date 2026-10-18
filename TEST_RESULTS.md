# Test Results

Updated: 2026-09-21 WIB

## 0.6.11 — terpasang idle 03:38:13 WIB

- 204 desktop PASS/0 failed/2 network ignored, 36 engine PASS. Pipeline
  fmt/Clippy/test/doc/release/audit489/87 PASS; dependency/pin tidak berubah.
- Enam regresi baru PASS; parser actual stale7267ms → supervisor flat
  FAIL sebelum patch dan PASS sesudah. Exposure uncertain/residual, ledger,
  health reset, gangguan setelah recovery, deadline/Stop, future/schema/replay,
  lease expiry/no-renewal/cooldown diuji. Satu assertion pesan lama sempat
  gagal; diperbarui menjadi pemeriksaan tipe error dan larangan order.
- Paket asli0.6.8/0.6.9/0.6.10 valid; export latest1trade/net−0.21681479 valid,
  sourcebuild0.6.10 tetap, exporter0.6.11. Gitleaks paket0, source satu fixture
  palsu existing identik baseline. UI native offline/privacy diperiksa.
- Probe600s PASS215snapshot/0failure/accountmaks2449ms/quote88–379ms/
  gap6808ms/2confirmedbar/2freshdecision/0signal.22primaryerror dipulihkan
  fallback. Dua setup PASS4snapshot, terakhir di bawahlock sebeluminstall.
  Source/release/terpasang/proses identik,16JSONstate/filelistunchanged.
- [Kontrak](docs/STALE_CLOCK_11.md),
  [laporan dan log](../../outputs/binance-stale-clock-011-20260921-mhBot6/LAPORAN.md).
  Bukan bukti posisiaktif0.6.11/24jam/profit; tidak aktivasisesi/order/model.

## Riwayat: 0.6.10 — terpasang idle21September01:20:43WIB

- **198desktopPASS/0failed/2networkignored,36enginePASS**; tambahan20tes.
  Fmt/Clippy semuatarget/fitur/doc/release/audit489/87PASS. Tidak dependencybaru.
  Dua reproduksi FAIL→PASS: Stop setelah outage15,099s dan HTTP401 salahNetwork.
- Faulttest: health60s keduaclock/replay/gap, repeat/third episode, entryonlynewbar,
  Stop/deadline/limit saatclose/cooldown, restart/legacy, partial/residual/uncertain,
  ledgernetwork/accountingexact, protectionunknown, foreignorder/wallet/leverage/
  stale, historyrevision, metadata/export/UIprivacy, timeout20s/dropfuture.
- Probe600sGET PASS201snapshot/0snapshotfailure, akunmaks2705ms, quote192–2281ms,
  gapsehat6882ms,1confirmedbar5644ms/2freshdecisions termasukwarmup/0signal,
  0revision/rewarm/held.22errorprimary dipulihkanfallback; bukan 0networkerror.
  Tidak ada order/posisiaktif, bukan benchmark trafik/jaringan terisolasi.
- Dua invocationsetupPASS4snapshot, kedua di bawahlock. Versi0.6.10 dan hash
  release/probe/terpasang/proses identik.14JSONstate unchanged. Nativecapture
  offline diperiksa; batas/akun/episode disamarkan. Paket0.6.8/0.6.9 nyata
  valid; eksportlatest1trade/net−0.07636919 valid. Gitleakspaket0temuan;
  source1fixturefakeexisting identikbaseline, bukan secretbaru.
- [Laporan/log](../../outputs/binance-recovery-010-20260921-1JfFpw/LAPORAN.md),
  [kontrakpatch](docs/TRANSPORT_RECOVERY_10.md). Tidak aktivasilive/paper/order,
  perubahan akun/risiko, modelcall; belum bukti recoveryreal/24jam/profit.

## Riwayat: 0.6.9 — terpasang idle 20 September 22:41 WIB

- **178desktop PASS/0failed/2network ignored,36engine PASS**. Tambahan12tes;
  fmt/Clippy semua target/fitur/doc/release/audit489/87 PASS. Tanpa dependency
  atau toolchain baru. Regresi serial5407msFAIL→parallel4598msPASS virtual;
  latensi endpoint lain sintetis. Clippy awal posisi test module sudah dibetulkan.
- Cancel/error/deadline keenamGET, exactbatch, sharedcooldown429/418bodyrusak,
  schema/503/APIerror, counterpersist/restart/datafailure/settlement/recoverydedup,
  legacy/countertamperexport dan UIlegacy/redaction diuji. Paket0.6.8 real
  diterima; latest1trade diekspor/validasi, Gitleaks0temuan. Counter bukanfill.
- Probe600s PASS203snapshot/0error/accountmaks2726ms/quote191–791ms/gap7917ms.
  1candleconfirmed6603ms,2freshdecisiontermasukwarmup/0signal,0rewarm/held.
  Batch6GET median126/p95348/maks737ms. DNS lambat tetap ada. Bukan benchmark
  terkontrol melawan sesi lama; bukan uji posisi aktif. Perbedaan hash probe
  dan final hanya tambahan testUI; [provenance](../../outputs/binance-session-78m-fix-20260920-MRMfwz/BUILD_PROVENANCE.md).
- Dua setupfinal PASS4snapshot, kedua di bawahlocksebelumrestart; ATR0.1300%
  kedua arah0.002BTC lolos hitungan, tanpaorder. Binaryterpasang/proses/release
  identik;12JSONstate unchanged. Nativecaptureofflinev0.6.9 diperiksa.
  [Laporan dan bukti](../../outputs/binance-session-78m-fix-20260920-MRMfwz/LAPORAN.md).
- Tidak ada aktivasi live/paper, perubahan akun/risiko atau model call.
  Hermes ditunda; belumacceptanceposisiaktif/24jam/profit.

## Riwayat: P2 kandidat0.6.8, belum dipasang pada putaran P2

- 166desktop PASS/0failed/2network ignored,36engine PASS; tambahan16tes P2.
  fmt/Clippy semua target/fitur/test/doc/release/audit489/87 PASS. Doctor52PASS.
  rustix1.1.4 existing dipakai langsung; tidak mengubah versi dependency/pin.
- Export session asli3trade/net−0.34155740USDT cocok total, paket release valid;
  canary/symlink/path/duplicate/unknown/mismatch/oversize/izin ditolak dalam
  fixture. Simulasi IO gagal di5titik tidak menerbitkan manifest valid.
  Sesi aktif/lock writer ditolak. Proposal angka fiktif/risk/executable ditolak.
- Export di namespace tanpa network + env kosong PASS; Gitleaks paket0temuan.
  Bukan uji runtime Hermes; provider/model/biaya/dataegress belum dipilih, tidak
  ada model run/isolasi Hermes/pilot/live acceptance24jam. Terpasang tetap0.6.6.
  [Laporan dan seluruh log](docs/EVALUATION_BRIDGE_P2_08.md).
- Probe360s PASS92snapshot/0error,accountmaks4082ms/gap7313ms/1confirmedbar,
  0freshdecision/0signal. Setuppertama FAILresolverpublik; kedua PASS2snapshot.
  ATR0.0816%/TPcap tetap menahan.10JSONstate+binaryterpasang identik. Tidak ada
  order/deploy atau klaim semua pemeriksaan jaringan lulus.

## Riwayat: P1 kandidat0.6.7, belum dipasang

- 150desktop PASS/0failed/2network ignored;36engine PASS. Tambahan13tes P1;
  fmt/Clippy semua target/fitur/doc/release/audit489/87 PASS. Dependency/pin tetap.
- Dua reproduksi FAIL→PASS: guard frame awal quote5069ms dan Watching setelah
  settlement Stop. Hasil negatif disimpan. History timeout/abort/konfirmasi,
  proteksi saat pending, entry fresh versus late/Stop/deadline, lease/cooldown,
  exit evidence, legacy hash/crash settlement dan deadline24jam virtual diuji.
- Probe GET360detik PASS/exit0:97snapshot/0error/0rewarm/1candle terkonfirmasi
  10118ms; accountmaks3359ms, gapsehat10955ms,0sinyal. Dua setup PASS/exit0,
  masing-masing2snapshot; ATR0.0851% tertahan. Hash10file JSONstate/outbox/ledger
  dan binary terpasang identik. Bukan order/aktivasi live, bukan bukti profit
  atau kestabilan24jam. Binary terpasang tetap0.6.6.
  [Laporan/bukti](docs/RELIABILITY_P1_07.md).

## Riwayat: input/plan0.6.6

- **137passed/0failed/2network ignored**,36tes engine PASS; fmt/Clippy semua
  target/fitur/doc/release/audit489 lulus. Pin dependency/toolchain tetap.
- Regresi candle provisional FAIL sebelum patch; konfirmasi setelah dua read
  identik PASS sesudah. Bug recovery dengan candle pending juga FAIL→PASS.
  Pipeline nyata input→supervisor: entry hanya setelah konfirmasi; late>15s,
  userStop, deadline, recovery tetap tidak entry. Revisi baseline tetap skip.
- Depth: stale/future/invalidtimestamp/IDmundur/isi berubah tanpaID, duplikat/
  levelzero/crossed/urutan salah ditolak. Endpoint depth non-BTC/Spot/limitlain
  ditolak sebelum jaringan. Quote tidak memakai timestamp lokal.
- Oracle15.000kombinasi biaya/lot/ATR/mark pada kode lama sudah PASS; tidak
  mengarang diagnosis salah planner. Review dan replay JSON memakai hasil
  inti identik,≤8percobaan tanpa wallet/credential. Penolakan TP kini berangka.
- Perbandingan20GET publik PASS: bookTicker232–2903ms, depthT101–118ms.
  Sampel berurutan/pendek, bukan benchmark serentak atau bukti stabilitas10jam.
- Dua setup GET10:41WIB PASS, ATR0.1757%; kedua arah layak matematika dengan
  batas pengguna. Capture nativeX11 diperiksa: v0.6.6, baca saja, sesi600menit
  selesai; nilai akun/cap disamarkan. Probe600detik PASS/exit0:168snapshot,
  0error/rewarm/held,2rollover/konfirmasi/keputusan,0sinyal. Quote193–686ms,
  accountmaks3916ms, gapsehat7598ms. Konfirmasi candle12486/7536ms, tetap≤15s.
- Instalasi idle di bawah lock, unit066 aktif baca saja; hash release/binary/
  proses identik.14file state/order/ledger identik baseline. Tidak ada aktivasi
  live/paper/POST/DELETE atau perubahan akun. [Laporan putaran](docs/INPUT_EXECUTION_PLAN_06.md).

## Riwayat: freshness/reconciliation0.6.5

- **129 passed, 0 failed, 1 network ignored**; fmt, Clippy semua target/fitur
  tanpa warning, doc, release dan audit489 dependency lulus. Lockfile hanya
  mengganti versi paket0.6.4→0.6.5, tidak dependency.
- Regresi old-code: frame berumur awal1000ms + dua query proteksi2200ms
  mereproduksi stale dan FAIL. New-code: rekonsiliasi sebelum frame, PASS,
  tidak Stop atau membuat order tambahan. Bukti order yang benar-benar>5s
  tetap ditolak meskipun account segar.
- Tes baru: stale/future account dan quote, bukti beda intent, proteksi hilang
  setelah query, GET dibatalkan+Stop+restart, Stop/deadline mengalahkan sinyal,
  market tanpa cumQuote dengan satu GET dan SL langsung, NotFound tidak resend,
  validasi batch sebelum jaringan dan headroom TTL metadata. Lifecycle lama
  termasuk dua trade dalam satu sesi dan partial/uncertain/ledger tetap lulus.
- Dua snapshot `--check-setup` nyata berhasil, mutations0. Probe600detik:
  168snapshot sehat/2penolakan revisi candle/1rewarm, **exit1**; dua rollover,
  nolsinyal, umur account maksimum4040ms dan gap sehat maksimum10056ms.
  Volume berubah195.660→195.836, pulih setelah dua pembacaan konsisten;
  candle revalidasi dilewati. Bukan uji posisi aktif/order nyata atau24jam.
- Capture native X110.6.5 diperiksa, akun baca saja dan sesi lama selesai.
  Binary release/terpasang/proses identik. Riwayat/state/config tidak dimigrasi.
- Kegagalan awal (regresi, jam fixture mundur, future trait belum async) disimpan
  bersama log final; tidak menghapus hasil negatif atau menurunkan guard.

[Kontrak, hasil dan keterbatasan](docs/FRESHNESS_RECONCILIATION_05.md).

## Riwayat: setup cost-aware 0.6.4

- Pipeline penuh: **119 passed, 0 failed, 1 ignored**. Format, Clippy, doc,
  release dan audit489 lulus; engine existing tidak diubah/diuji mandiri ulang.
- Regresi mencakup benturan biaya/lot/risiko, TP cap6ATR, slippage TP, presisi
  ATR, budget margin sebagai plafon, penolakan sebelum penulisan sesi, UI
  preview tanpa aktivasi dan durasi historis tidak berubah oleh analisis baru.
- Supervisor/exchange tiruan menguji kedua arah dengan batas pengguna, harga
  SL/TP final sampai command proteksi dan ledger. Simulasi240 menit dengan dua
  gangguan multi-menit mempertahankan deadline dan tidak restart/reentry lama.
- GET setup: snapshot pertama layak bersyarat, kedua gagal harga usang;
  **exit1**, bukan dua PASS. Capture native0.6.4 diperiksa, akun baca saja.
  Binary/proses terpasang cocok release; session.json identik backup.
- Probe GET300 detik:79 snapshot sehat/3 error harga usang, gap sehat maksimum
  6964ms, satu candle baru dan nol sinyal (ATR0.069%). **Exit1**, bukan PASS;
  pembacaan pulih tetapi penyebab timestamp belum dipastikan.
- Tidak ada order/mutasi Binance atau bukti profit. Hasil probe5menit dan
  kegagalan awal disimpan pada [laporan putaran](docs/FEASIBILITY_LOGIC_04.md).

## Riwayat: perbaikan recovery 0.6.1

- `rust-workbench check`: **89 passed, 1 network ignored**; fmt, Clippy seluruh
  target/fitur `-D warnings`, doc, release dan audit489 dependency lulus.
- Regresi: revisi candle menolak sinyal tetapi tidak menghalangi recovery;
  snapshot akun stale/gagal tetap ditolak. Stop tanpa history bisa selesai
  flat atau menjalankan penutupan/accounting pada exchange tiruan, tanpa reentry.
- Pemulihan catatan kosong menolak sesi berjalan, data stale, posisi/order/algo,
  wallet berubah, intent dan riwayat accounting; state tidak berubah saat ditolak.
  Form memulihkan config persis tanpa persetujuan dana nyata.
- Binary terpasang 0.6.1 identik dengan release. Recovery aktual dua GET segar
  berhasil; state `finished=true`, sequence/completed0, currentNone. Binding,
  config, daily dan angka accounting identik dengan backup. Tidak ada mutasi Binance.
- Percobaan recovery pertama gagal resolusi DNS dan tidak mengubah state.
  Percobaan berikut berhasil; diagnosis akun seluruh10 checks PASS dua kali.
  `--probe-auto` terpasang PASS dua snapshot, usia maksimum2209ms, mutations0.
- Unit aplikasi aktif. Screenshot akhir hanya memperlihatkan lock screen;
  bukan bukti visual UI setelah patch. Error spesifik sesi lama tidak tersimpan.
- Tidak mengaktifkan trading, menjalankan order-test, mengubah strategi/risiko,
  atau mengklaim acceptance order/profit. Engine existing tidak diuji ulang.

[Lingkup/hasil](docs/SESSION_RECOVERY_FIX_01.md),
[log dan keputusan](../../outputs/binance-recovery-fix-20260917-lzZWCi/DECISION.md).

## Riwayat: sesi otomatis 0.6.0

- **84 passed, 1 public-network ignored** desktop; **36 passed** engine.
  fmt, Clippy seluruh target/fitur `-D warnings`, doc, release dan audit489
  dependency lulus. Source engine cocok checksum baseline.
- Uji mencakup breakout/pullback/range simetris, konflik/no-signal/late, satu
  sesi Long→flat→Short tanpa input per order, SL segera setelah fill, TP/child
  cleanup, biaya aktual model exchange tiruan dan perubahan wallet.
- Respons entry/SL hilang + restart, partial fill, SL reject, residual exit,
  data stale, order asing, fee BNB, stop sebelum konfigurasi, batas rugi/durasi/
  jumlah posisi, pagination2002 fill, duplicate/missing/foreign fills, funding
  tertunda, IO/arsip crash, daily persistence dan locking diuji.
- Klik pointer UI mengirim Start/Stop hanya ke worker fixture; checkbox/budget
  wajib, Start ganda ditolak, offline/capture tidak dapat aktivasi. Tidak ada
  kredensial/network pada pengujian order/UI.
- `--probe-auto` binary terpasang membaca akun/pasar/feeBurn GET-only dua kali;
  PASS, usia snapshot maksimum1793ms, tanpa nilai akun/credential di output.
- Kegagalan awal dipertahankan: assertion UI kalkulator lama setelah collapse;
  fixture short dilewati oleh filter biaya; fixture EMA berpresisi>12 digit;
  assertion serialized Side memakai kapital; Clippy digit-grouping. Fixture/
  assertion diperbaiki tanpa menurunkan aturan biaya/validasi produksi.
- Tidak menjalankan POST/DELETE/order-test Binance, sesi paper, optimasi strategi
  atau transaksi dana nyata. Hasil bukan evaluasi profit atau acceptance order.

[Bukti dan keputusan](../../outputs/binance-autonomous-session01-20260916/DECISION.md).

## Riwayat: klarifikasi UI arah 0.5.1

Patch label manual kalkulator vs arah strategi otomatis; **67 tes desktop lulus,
1 public-network ignored**, fmt/Clippy/doc/release/audit489 lulus. Tidak menambah
tes yang hanya mencocokkan kata label. Engine dan strategi tidak diubah atau
dijalankan ulang sebagai sesi trading. [Bukti](../../outputs/binance-direction-ui-20260916/DECISION.md).

## Margin/konektor Futures 0.5.0

- **67 passed, 1 public-network ignored** desktop; **36 passed** engine.
  fmt/Clippy `-D warnings`, doc, release serta audit **489 dependency** lulus.
  Source engine tetap cocok seluruh checksum dari putaran engine sebelumnya.
- Uji desimal/sizing/tier/maintenance/likuidasi long-short, biaya/funding reserve,
  filter/stale/stop/guard, HMAC dan request contract, error unknown/429/5xx,
  durable reserve/duplicate/restart/corruption, partial fill, proteksi ditolak,
  entry/SL response hilang, cancel/fill race, cleanup/child fill accounting.
  Regresi receipt tidak boleh menurunkan fill atau membangkitkan order terminal.
  Estimasi likuidasi short tidak menganggap reserve saldo sebagai tambahan
  isolated collateral; persamaan equity=maintenance diverifikasi pada plan.
- Worker session credential tanpa fetch, disconnect, UI form/kalkulasi/stale,
  minimum1024×720 dan capture-redaction diuji. Tidak ada real order pada tes.
- Probe bracket awal gagal karena batas parser metadata leverage125. Metadata
  resmi akun melampaui batas itu; parser dipisah dari batas request≤125.
  Probe aktual berikutnya **PASS dua snapshot GET-only**. Tidak menyimpan nilai
  akun, signature, token atau raw response dalam bukti.
- Clippy menemukan nested-if dan letak test module; keduanya diperbaiki sebelum
  pipeline final lulus. Instalasi pertama harus dipanggil melalui `bash` karena
  script tidak executable; versi0.5.0 kemudian terpasang. Log gagal dipertahankan.
- Native capture Wayland tidak menghasilkan PNG pada sesi ini; capture melalui
  X11 berhasil pada 1400×900 dan 1024×720. Nilai akun disamarkan; badge preflight
  menunjukkan pembacaan nyata berhasil. Tidak menangkap desktop/aplikasi lain.
- Tidak menjalankan POST/DELETE Binance, order-test, perubahan akun, sesi paper
  baru, evaluasi strategi atau pengujian Python/JS pada putaran ini.

[Bukti lengkap dan batas penerimaan](../../outputs/binance-futures-execution-round01-20260916/DECISION.md).

## Riwayat: forward paper 0.4.0

- **46 passed, 1 public-network ignored** pada desktop; **36 passed** engine.
  fmt/Clippy `-D warnings`, doc, release dan audit **489 dependency** lulus.
- Regresi: warmup/no pre-start entry, indikator kausal, long/short/sizing,
  TP/SL/fill/fee/cleanup, dedup, stop manual/deadline, funding gate/exit/missed
  cost flag, gap/revisi/filter/stale/future, outage dan replay state/hash jurnal.
  Metadata BTC resmi sebagai fixture; transport menolak order/account/URL bebas.
  Kontrol stop UI sampai channel worker; offline/report tidak bisa start.
- Probe pertama gagal karena respons exchangeInfo >1MiB; batas khusus dinaikkan
  menjadi4MiB, lainnya tetap1MiB. Probe berikutnya **PASS**, ≥3 snapshot harga
  publik, selesai flat, jurnal diverifikasi oleh CLI engine unchanged.
- Aplikasi terpasang menjalankan sesi paper60menit pada feed nyata. Capture native
  laporan virtual diperiksa; tidak menangkap data akun/secret. Ini bukti operasi
  software, bukan evaluasi profit atau keberhasilan real exchange fill.
- [Hasil dan bukti](../../outputs/binance-forward-paper-round01-20260916/DECISION.md).

## Terbaru: patch parser akun 0.3.1

- **34 tes lulus, 1 tes jaringan ignored** dalam pipeline; fmt/Clippy/doc/release
  dan audit 489 dependency lulus. Empat regresi baru: seluruh 897 nama simbol
  publik termasuk lima Unicode, control/bidi/zero-width/duplikat tetap ditolak,
  diagnosis endpoint tanpa isi respons, dan transisi dua snapshot tanpa data akun.
- Tes daftar simbol resmi **gagal sebelum perbaikan**, lulus setelah validator
  Unicode diterapkan. Angka saldo/konfigurasi fixture sintetis; daftar namanya
  berasal dari GET exchangeInfo publik resmi tanggal 16 September 2026.
- **Akun nyata berhasil:** worker UI dan probe CLI terpasang masing-masing
  mencapai dua snapshot read-only. Output hanya status. KWallet kini ready;
  tidak ada secret atau nilai akun di log. Native framebuffer memperlihatkan
  TERHUBUNG dengan seluruh detail akun disamarkan pada capture audit.
- Patch terpasang, unit sementara aktif. Engine seluruh checksum tetap cocok.
  Tanpa order/mutasi, strategi/profit baru, atau rerun Python/JS.
- [Bukti dan hasil](../../outputs/binance-account-parser-fix01-20260916/DECISION.md).

Bagian berikut mempertahankan keadaan sebelum patch parser.

## Akun Futures desktop 0.3.0

- **30 tes lulus, 1 tes jaringan dipisahkan** dari suite offline. fmt, Clippy
  `-D warnings`, doc, release dan audit **489 dependency** lulus.
  [Pipeline](../../outputs/binance-account-desktop-round01-20260916/check-final.log).
- Cakupan baru: HMAC RFC 4231, fixed GET/host/path/query dan sensitive header,
  exact decimal, payload/duplikat/mode posisi invalid, Hedge long/short tidak
  di-net, liquidation zero/missing config, auth/rate-error redaction, Retry-After,
  stale/recovery, cancel disconnect dan private state tidak muncul kembali.
  Tes pointer memeriksa input password tersamarkan dan buffer kosong setelah
  submit; capture menyamarkan akun. Semua halaman/kontrol minimum diuji.
- Tes publik yang di-ignore dijalankan terpisah: **1 passed**, memakai transport
  akun ke endpoint waktu Binance resmi tanpa kredensial/signed request.
  KWallet aktual berhasil tulis/baca/hapus entry dummy terisolasi.
- Native framebuffer halaman akun 1400×900 dan 1024×720, serta ruang kerja pasar
  pada binary terpasang diperiksa. Form aktif ketika wallet kosong; status
  **BELUM DIKONFIGURASI**, data akun belum tersedia. Launcher valid, binary
  release identik dengan terpasang, unit user sementara aktif.
- Compile awal menemukan import HMAC/borrow UI; Clippy menemukan format borrow.
  Capture awal menemukan status Loading tidak selesai pada vault kosong.
  Semuanya diperbaiki; missing-vault regression ditambahkan, log awal dipertahankan.
- **Autentikasi live belum diuji:** entry KWallet aplikasi tidak ada; probe
  berhenti dengan `ACCOUNT_CREDENTIAL_MISSING`. Jangan mengartikan tes publik,
  keyring atau contoh sintetis sebagai saldo/posisi akun nyata.
- Engine-rs seluruh checksum cocok. Tidak ada rerun seluruh suite engine/
  Python/JS, riset strategi, order atau perubahan akun. [Keputusan](../../outputs/binance-account-desktop-round01-20260916/DECISION.md).

Bagian berikut adalah riwayat sebelum integrasi akun desktop.

## Integrasi pasar desktop 0.2.0

- **16 tes lulus**, fmt/Clippy/doc/release dan audit 422 dependency lulus.
  [Pipeline final](../../outputs/binance-market-desktop-round01-20260916/check-verified.log).
- Parser menolak gap/duplikat/timeframe salah, OHLCV invalid/nonfinite; DNS
  menolak IP privat/reserved dan menghormati TTL; retry seconds/HTTP-date diuji.
- Worker dengan sumber uji mengalami putus/pulih tanpa duplikat, membatalkan
  permintaan lambat saat pilihan berganti, menjaga status stale/jeda. UI pointer
  menguji selector, zoom, drag history/reset dan kontrol demo lama.
- Native framebuffer live Futures 1400×900 dan Spot 1024×720 diperiksa; chart,
  volume, pilihan pasar/timeframe dan umur data terlihat. Probe publik memakai
  worker produksi untuk BTC/ETH × dua pasar × empat timeframe, lalu pause/resume.
- Tes awal 12/13 lulus: CTA simulasi berada di luar viewport; dipindahkan ke
  header ruang kerja, dan kontrol pasar minimum serta drag ditambahkan ke regresi.
  Capture awal merekam kegagalan koneksi; fallback kini mencoba hingga empat
  alamat publik secara eksplisit. Log/capture awal tetap disimpan.
- Semua checksum engine-rs dari putaran sebelumnya tetap cocok. Tidak ada
  pengujian strategi/profit, order nyata, akses API privat atau rerun Python/JS.
- [Paket bukti dan batas](../../outputs/binance-market-desktop-round01-20260916/DECISION.md).

Bagian berikut mempertahankan hasil tahap sebelumnya.

## Terbaru: aplikasi desktop Linux native

- **6 tes lulus**: replay/control sampai cleanup, export partial state + replay
  journal, validasi/persistensi settings, pointer-click navigation/play/pause/
  next/back/reset, pointer export/save settings, serta layout semua halaman dan
  visibilitas kontrol demo pada 1024×720.
- `rust-workbench check`: fmt, Clippy `-D warnings`, tests, doc, release dan Cargo
  audit lulus (339 dependency dalam lockfile). [Log](../../outputs/binance-desktop-round01-20260916/check-release.log).
- Render nyata eframe/Glow pada KDE/Wayland: workspace, simulasi langkah 35,
  jurnal langkah 50 dan settings diperiksa lewat screenshot framebuffer aplikasi.
  Ukuran 1400×900 dan 1024×720 diperiksa. Koreksi visual: glyph panah/dot,
  label status yang terbungkus dan tinggi chart agar tombol minimum terlihat.
- Installer user-local berhasil, `.desktop` lolos validator, cache KDE diperbarui.
  Binary terpasang membuka frame native; unit user sementara mempertahankan
  jendela sesi setelah launcher dari tool tidak bertahan. Tidak ada autostart.
- 52 pemeriksaan lingkungan lulus. Engine Rust tetap memakai kode sebelumnya;
  36 tes engine dan hasil Python/JS di bawah tidak diklaim sebagai run baru fase UI.
- [Keputusan dan tangkapan layar](../../outputs/binance-desktop-round01-20260916/DECISION.md).
  Nol feed/account/order nyata, backtest atau bukti profit baru.

## Terbaru: inti eksekusi Rust paper

- `rust-workbench check`: **36 passed** (1 unit lock, 24 execution/risk,
  11 persistence/CLI); format, Clippy `-D warnings`, doc, build release dan
  Cargo audit lulus. Audit memeriksa 87 dependency pada lockfile tanpa temuan.
- Eksekusi: long/short, partial entry/exit, arah stop/TP, biaya/P&L desimal,
  cancel-before-exit dengan late fill sebelum cancel, ambiguous submit/query,
  acknowledgement timeout, stale quote, gap/daily/total loss, halt persisten,
  invalid filter/config/report, duplicate report dan status cleanup.
- Persistensi: state/hash replay identik, setiap prefix demo direstart, dua demo
  byte-identik, duplicate/collision, rejected events, single-writer lintas proses,
  checksum/replay-outcome mismatch, missing record, torn tail dan batas ukuran.
- Pipeline gabungan awal: 33 lulus/2 gagal karena descriptor proses anak menahan
  lock sesaat setelah writer ditutup. Explicit unlock pada Drop dan tes regresi
  menyelesaikannya. [Log awal](../../outputs/binance-execution-round01-20260916/check.log)
  tetap disimpan; [log final](../../outputs/binance-execution-round01-20260916/check-final.log).
- Binary release menjalankan demo 11 event/12 record. CLI status dari proses lain
  menghasilkan state/head hash identik. Net contoh +16.258140900 pada simbol
  `SYNTHUSDT` adalah hasil fixture yang sengaja ditentukan, **bukan P&L strategi**.
  [Demo dan batas](../../outputs/binance-execution-round01-20260916/demo/report.md).
- `codex-operational-baseline/scripts/doctor.sh` lulus. Tidak memasang tool global.
  Python/JS existing tidak diubah dan tidak diuji ulang; angka di bawah historis.

## Terbaru: aktivasi observer kanal H4

- `python -m pytest -q`: **283 passed**, satu warning Starlette/AnyIO yang sudah ada. Seluruh `tests/*.test.cjs`: **23 passed**. `pip check`: no broken requirements.
- 16 kasus Python baru: warmup nontrading, sinyal/entry/stop model, restart/idempotensi, preview exclusion, stale/gap/revision, delayed provenance, code identity, GET-only API, quote freshness, public host/path/method/credential allowlist, DNS cache/global-IP guard, original hostname dan rate-limit backoff. Empat kasus JS baru: countdown, offline/stale, reason, null result.
- Tes awal integrasi: 18 passed / 1 failed karena penutupan response stream tanpa `aclose` pada stream sederhana. Cleanup diperbaiki sesuai kontrak stream opsional; regresi lengkap lolos. Tidak ada hasil P&L historis baru.
- Probe publik nyata: system DNS fapi/fstream -> 103.133.62.251, connect timeout; Google HTTPS bisa diakses. DNS-over-HTTPS mengembalikan alamat berbeda; curl `--resolve` dengan TLS hostname utuh dan transport aplikasi keduanya menerima respons resmi /time dan /klines. Full warmup 24 feed, max lag 0, exchangeInfo enam instrumen, weight 50/2400 pada probe (snapshot, bukan jaminan konstan).
- Runtime: 24/24 LIVE_REST_FALLBACK, 29/29 tugas RUNNING, observer tanpa restart internal saat diperiksa. WebSocket tidak pulih; account AUTH_UNAVAILABLE dan counts null. Tidak ada klaim akses akun berhasil.
- Browser: harga BTC berubah 77,894.80 -> 77,910.40 -> 77,921.50 selama verifikasi; bukan P&L. Keempat model/level kanal UI sama dengan API. Desktop 1440px dan mobile 390px tanpa overflow dokumen. Screenshot dilihat inline, tidak diklaim tersimpan.
- Restart nyata: PID 22728 dihentikan dengan ownership check; browser menunjukkan KONEKSI DASHBOARD TERPUTUS/stale; runner diluncurkan ulang. Meta session/input dan kondisi engine tetap identik (220 warmup BTC + 220 ETH, tanpa duplikasi); browser pulih otomatis. PID dapat berubah, gunakan status.ps1.
- Endpoint observer aktif tanpa order routing. Funding belum dicatat: net lengkap null. Sinyal/entry/exit diuji pada fixture sintetis, belum ada trade pasar baru dalam sesi singkat ini.

## Latest: MT5 evidence pipeline

- Official v5.20 demo installed after manual user login; one preregistered visual test completed. FBS 2026-09-01–2026-09-13, 10000 USD, 1:100, 100 ms, real-tick model requested, no optimization. 22 trades / 44 deals + initial deposit; net +671.44 USD; final 10671.44 USD; reported PF 1.98, equity DD 406.57 USD (3.70%), history quality 83%. All balance movements and ledger net reconciled. Report, vendor defaults, tester INI, agent log and hashes retained; API and actual browser balance chart/metrics/45-row ledger verified. [Evidence and limitations](docs/DEMO_RUN_01.md).

- **130 Python / 19 JavaScript tests passed**, clean `pip check`; one pre-existing Starlette/AnyIO warning, no dependency change.
- Eight actual legacy report format checks reconciled; one unit-harness report rejected for unsupported period/currency. One old EURUSD report imported idempotently: 23 ledger rows, 22 trading deals, net -219.09 USD. **Not a new candidate backtest.**
- New coverage includes encoding, truncation/nonfinite data, ledger/count/time mismatch, secret exclusion, source identity, concurrency, restart/immutability, corrupt-store fail-closed, read-only API and repeat gates.
- Browser: 1440px desktop; 390px mobile with no panel overflow; real archive metrics/ledger/balance chart; candidate empty state; offline refresh clears results. No online console messages after recovery. Screenshots inspected inline only.
- Service restarted using verified project PID/port ownership; 28/28 tasks, 24/24 REST fallback, lag 0, account connected and all order counts 0 at check. Current PID: use `scripts/status.ps1`.
- This demo follow-up changed no application code or dependencies. The 130 Python / 19 JS result above is from the preceding implementation verification, not a newly rerun suite. No live orders, purchase, VPS or Algo Trading activation. Full pipeline limitations: [TESTER_PIPELINE.md](docs/TESTER_PIPELINE.md).

## Previous: Range Breakout Reference

- **99 Python tests passed**, **15 JavaScript tests passed**, syntax checks and clean `pip check`. One existing Starlette/AnyIO dependency deprecation warning; no dependency changes.
- New tests cover complete range formation, causal closed-bar input and preview/future exclusion, long/short symmetry, holding beyond 4/8 candles, 144-bar session hold, no fixed TP/trailing, stop/gap precedence, one position/day, ambiguous first-side handling, missing/invalid/revised data, funding distinction, persisted restart/backfill identity, immutable completed outcomes, bounded recorded-chart routes, and GET-only API behavior.
- A recorded-chart integration test found cross-thread reuse of the live SQLite connection. The chart route now opens a request-local **read-only** connection; regression passes. No global thread-safety protection was disabled.
- One fixed replay over the already-seen **450 days / six pairs**: 2,700 sessions, 2,685 resolved closed model positions, 14 no breakout, one ambiguous. Mean net **-0.075788339R**, stress **-0.185124779R**, profit factor **0.874949901**. Results are exploratory, resolved-event statistics, not an account return or fresh validation. No parameter optimization or profitable-pair selection.
- Source-described engine matches the declared no-TP/no-trailing/first-breakout mechanism; Binance, UTC, weekends and M5 modeling are adaptations. Vendor profitability has not been independently verified.
- Active runtime verified at `http://127.0.0.1:8765`: 28 tasks, reference task RUNNING / 0 restarts; retired `research` and `strategy-lab` tasks absent; 24 feeds through REST fallback. Account CONNECTED, actual positions/orders **0/0**, `ip_restrict=false` unchanged. See PID file for the current owner after restarts.
- Browser integration in the isolated debug profile: source rules/evidence/archive/account tabs; synthetic HOLDING after 54 M5 bars; actual recorded XRPUSDT session for 2026-09-12 with 288 real M5 candles and exact net-result match against the report; historical mode and account isolation; return to live; scroll position preserved across re-render; no console errors/warnings on the final navigation.
- Desktop 1440px and mobile 390px layout inspected. Mobile document scroll width **390px**; chart scales to container. Screenshots inspected inline only; attempted file save was denied by the browser tool, so no screenshot file is claimed.
- Export includes all reconstructed sessions: `Workspace/outputs/binance-range-reference-results.json`. Model definition: [REFERENCE_ENGINE.md](docs/REFERENCE_ENGINE.md). Old code, tests, reports and database retained for reproducibility; only the active observer and homepage changed. No external account mutation or hosting migration.

## Archived: Phase 3

- Python: **80 passed**, one existing Starlette/AnyIO dependency deprecation warning. JavaScript: **10 passed** (eight terminal-model tests plus two new panel-cache regression tests). Syntax checks for `lab-panel.js`, `research-panel.js`, `terminal.js`, Python compile, and `pip check` pass. No new dependency was installed.
- Added tests cover mirrored long/short retracement geometry and valid stop/target direction; wrong-direction trigger rejection; context reversal/gap cancellation; future higher-timeframe rejection; explicit cost/room gates; prior-range sweep evidence; H4 neutral/opposition and revision-sensitive context; training selection unchanged by future extreme outcomes; uniform purge/embargo; no false promotion; experimental warmup isolation; signal/state persistence and restart deduplication; GET-only Lab routes, bounded inputs/404s/assets; and config provenance.
- Numeric eight-bar ablation test proves completion uses the eighth close, seven bars remain pending, and a ninth bar cannot alter the completed result. Recorded event-window test verifies 50 + 8 = **58 M5 candles**. Exact gap-exit funding boundary test verifies no uncertain credit is invented.
- Public dataset: **899,520 candles**, 24/24 complete feeds, **6,480 funding records**, 622,080 closed-M5 decision times across six pairs. Trial **lab-e98337744f4aefe8ba8625fc**: eight variants, three expanding train-selected folds, final 60-day holdout. Selected `SWEEP_T1_4`, holdout **76 / -0.227142R net**, stress **-0.402329R**; matched baseline **25 / -0.563761R**. All three folds and all eight full-period variant means are negative: **NO_PROMOTION**.
- Prior trial `lab-6b33792f035df1f24d6cbbac` is retained. Direct report comparison verifies equal validation, baseline, and variant results except run IDs after adding config fingerprint provenance. These are not independent samples. Current strategy/config/protocol source fingerprints match the final stored report.
- Browser integration in a separate debugging context: Lab API and assets load; HASIL UJI / OBSERVASI BARU switch correctly; actual historical eight-bar evidence opens with stored amber area, entry/stop/target, 58 candles, explicit hindsight shading and `TIME STOP / 8 M5`; the new live chart uses actual archived M5 bars without fabricating an area. Expanded results survived subsequent live refreshes. Main chart is explicitly labelled as baseline in the Lab panel. No console errors/warnings during normal checks.
- Final integration review found Research's cached-render guard also matched the new Lab toolbar. A unique `phase2-toolbar` marker fixes stale cross-tab content. Two dependency-free DOM-fixture tests and repeated actual browser Research → Lab → Research switching verify the fix.
- Desktop 1440px and mobile 390px screenshots inspected inline. Mobile document/body width both **390px**; wide comparison tables scroll inside the panel (not the page). No screenshot file is claimed.
- Service restarted through verified project PID/port ownership; new owner **25112**. **29/29 tasks RUNNING**, **24/24 LIVE_REST_FALLBACK**, max lag **0**, `strategy-lab` and `research` restarts **0** at check. Account **CONNECTED** with positions/regular/conditional/execution-algo/total orders **0/0/0/0/0**. Binance WS still reports its real silent/reconnecting state.
- New experimental namespace `shadow-abb5544cdf1afa3c84942894` started **2026-09-13 17:16 UTC**. The first new M5 close ending 17:19:59.999 UTC advanced **48 variant/pair states**: 8 H1_NEUTRAL, 30 WAIT_IMPULSE, 10 WAIT_SWEEP; no signals or pending outcomes at that check. Historical warmup was not relabelled as new observations. This is one observed closed-bar transition plus deterministic restart tests, not a multi-day live soak.
- The Sites workflow preserved the existing local terminal structure/assets and local SQLite storage; nothing was hosted or migrated. No exchange mutation capability was added; IP-restriction policy, credentials, leverage, margin, orders, and positions were not modified.

Findings and interpretation: [PHASE3_RESULTS.md](docs/PHASE3_RESULTS.md). The remainder preserves earlier-phase evidence; do not add its counts as new independent validation.

## Previous: Phase 2

- Python: **63 passed**, one existing Starlette/AnyIO dependency deprecation warning. JavaScript model: **8 passed**. Research/terminal JS syntax, Python compile, and `pip check` pass.
- New tests cover next-open rather than signal-close fills; long/short fee/slippage; missing funding; conservative funding timing; stop/target ambiguity; gap stop; invalidated entry; incomplete/gapped horizons; preview/future exclusion; purge/embargo; empty metrics; immutable completed outcomes; SQLite restart/dedupe; live/historical separation; recorded event-window bounds; GET-only research routes; and warmup anti-backdating.
- One run exposed a pre-existing Windows timer race: a planned-renewal timeout could be labelled WS_SILENT when the clock read fell slightly before the deadline. Classification now uses the deadline that was armed; the regression suite passes.
- Public download: 228,840 validated candles with 24/24 complete feed windows; 1,620 funding records. Final causal study: 155,520 decisions, 27 signals, 16 development / 11 holdout; modeled net means -0.395672R / -0.403454R. Repeat instrumentation run has identical signals/outcomes, not new independent evidence.
- Isolated browser integration: Research API/assets 200; historical/live source switch displays distinct samples; event dialog returns 54 recorded M5 candles and matching levels/model outcome; no JS console errors during normal testing. Frequent live renders preserve expanded research details. Responsive document width verified at 390px. Screenshot inspected inline; saving through the browser tool to Workspace/outputs was denied by that tool's path policy, so no screenshot file is claimed.
- Service restarted using verified project PID/port ownership. Latest owner PID **26676**; **28/28 tasks RUNNING**, **24/24 LIVE_REST_FALLBACK**, max lag **0**; research recorder RUNNING with no restarts. The first new M5 close created **6 live decision rows**, distinct from historical rows. Account CONNECTED; positions and all order counts **0**.
- No execution path was added. IP restriction policy, account credentials, margin, leverage, and order state were not modified. A normal closed-candle capture and synthetic restart test are not a multi-day live soak.

The remaining sections record earlier phases and should not be mistaken for additional current validation samples.

Offline command: `.\.venv\Scripts\python.exe -m pytest -q`

Result after terminal UI patch: `48 passed`, with one third-party Starlette `TestClient` / AnyIO alias deprecation warning. Project lifespan warnings are gone. `pip check` reports no broken requirements. JavaScript: `node --test tests/terminal-model.test.cjs` reports `8 passed`; `node --check web/assets/terminal.js` succeeds.

Terminal-specific verification:

- REST warmup/update preview is `closed=false`, remains outside the decision cache, and is removed when finalized. Delayed WS previews cannot resurrect a finalized candle.
- Python decision explanations expose the actual neutral-H4 blocker and preserve dependency states. No strategy threshold was changed.
- Six independent replay frames progress from ARMED to TRIGGERED and four completed outcome candles. Each frame excludes later candles/events.
- JavaScript tests cover offline/stale snapshots, HTTP fallback, replay symbol selection, zone-matched candidate selection, explicit archive selection, and missing-value freshness/distance behavior.
- Isolated browser: ETHUSDT → REPLAY selected BTCUSDT automatically; step exposed TRIGGERED with matching candidate/zone and zero outcome bars; play ended at COMPLETED with four bars and gross final 1.6R in the synthetic fixture.
- Browser offline emulation retained DISCONNECTED through repeated renders, with “Data belum siap” and a cached-snapshot chart label. No Binance service network setting was changed.
- Desktop and 390px mobile visual checks verified the terminal layout. Mobile document width stayed 390px; chart bitmap matched CSS size × devicePixelRatio. Fixed an intrinsic canvas sizing issue found during resize QA.
- No application console warnings/errors during normal live/replay inspection. Expected network errors occurred only during explicit offline emulation.
- The Windows Computer Use helper stopped native browser control because it could not confidently determine the current browser URL. Native control was not retried after that stop. Visual and interaction checks were completed in the separate browser debugging context; the user's existing tab may still require manual refresh.

Runtime after this patch: verified owner PID 8768, 24/24 LIVE_REST_FALLBACK feeds, lag 0, 27/27 tasks RUNNING, account CONNECTED, all order counts 0. This is a point-in-time check; the earlier Phase 1D soak below remains historical evidence rather than a new multi-day soak.

Semantic replay result:

`REGIME → IMPULSE → DETECTED → ARMED → TOUCHED → TRIGGERED → CANDIDATE → COMPLETED → OUTCOME`

The deterministic long fixture records MFE `2.1R`, MAE `0.4R`, final `1.6R`, and target first-hit on outcome bar 3. Tests also cover future-candle isolation, snapshot purity, persistence reload, duplicate-bar rejection, long/short direction, zero risk, and same-bar ambiguity.

Live read-only soak after Phase 1D restart:

- Global transport stayed `LIVE_REST_FALLBACK` for all 40 samples.
- REST fallback stayed `24/24`; stale maximum `0`.
- Task health stayed `27/27 RUNNING`.
- Maximum lag was `0` bars; duplicate delta was `0`.
- Maximum preview age was about `29.9s`, matching the slowest configured 4h fallback cadence.
- Account snapshot age advanced from about `1.2s` to `31.4s` and reset on refresh, proving it is no longer a static zero.
- Account remained `CONNECTED`; regular/conditional/execution-algo/total orders were `0/0/0/0`.
- WebSocket delivered `0` valid payloads. Its substate cycled through connect/silent/reconnect while REST feed state remained stable.
- Public REST requests increased from `165` to `352` during the sample; observed used-weight telemetry ended at `338` against the advertised minute limit `2400`.

Security checks: no secret material is tracked; the Binance client exposes GET only and blocks non-allowlisted paths before network. No order or account mutation was sent.
