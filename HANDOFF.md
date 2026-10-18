# Handoff

Project: `~/Workspace/projects/binance-sniper`

## 0.6.14 terpasang IDLE — jangan aktifkan sesi tanpa instruksi pengguna

[Review offline](../../outputs/binance-review-014-20260921-nhQrIi/REVIEW.md)
lulus tanpa blocker baru. Anchor aktivasi memakai start/deadline saat begin dan
floor akuisisi frame; recovery membawa acquisition/observation Instant terpisah
serta tetap mensyaratkan60detik lokal+endpoint, gap<=10detik, no replay/rollback.
[Kontrak](docs/STALE_CLOCK_14.md).

Pra-pasang21September09:08–09:20WIB: kandidat/release/source69file cocok.
Audit network resmi kedua lockfile lulus489/87; RustSec HEAD
`d5c17953a895cf19e8d3ce66eaa42b6fcfe1fb16` (19September). Probe resmi600s
lulus209snapshot/0failure/2rollover/maxage2792ms/gap5753ms/2confirmedbar/
2freshdecision/0signal/0mutation. Setup tersimpan dua snapshot lulus dan jalur
`Frame::setup` fresh GET membuktikan posisi yang diperiksa kosong serta
order/algo BTCUSDT kosong. `OpenOrders`/`OpenAlgos` memakai `symbol=BTCUSDT`,
jadi pending order simbol lain belum dibuktikan; tidak ada bukti order asing
ada. `within_30s_recovery_budget` hanya label metrik probe, bukan guard recovery:
`Healthy` tetap gap<=10detik dan health>=60detik; gap aktual5753ms lolos10detik.

Runtime saat handoff: unit `binance-desk-debug-014.service`, PID201536 sejak
09:20:44WIB, active/running, Restart=no. Candidate=installed=`/proc/201536/exe`
SHA256 `5903daea453675bd76b6df122e48973a7bbc8d8f1dedbfd83273fa95b020c19f`.
Hanya satu instance; unit012 sudah inactive/not-found. Startup mencapai
`TERHUBUNG · BACA SAJA`; journal0sessionevent/0orderdispatch.37 file state
identik baseline: sesi lama finished/Stop,currentnull,decisionnull,
recoveryinactive. Tidak ada sesi live/paper otomatis.

Backup binary0.6.12/launcher/unit dan prosedur rollback ada di
`../../outputs/binance-install-014-20260921/rollback/` dan `ROLLBACK.md`;
hash lama `0b940b1a004fcf24d013f570407c4dffa27a6f4d4e443e908c896e87d0886d54`.
Jangan rollback/restart bila kemudian ada exposure atau sesi aktif. Tidak ada
perubahan risiko/config/state, order, cancel, atau aktivasi trading pada putaran
ini. [Laporan lengkap](../../outputs/binance-install-014-20260921/LAPORAN.md).
Instalasi idle bukan bukti acceptance posisi, operasi24jam, atau profit.

## Kandidat 0.6.13 teruji, BELUM dipasang

Mulai dari [review independen](../../outputs/binance-review-012-20260921-jHdZpY/REVIEW.md)
dan [desain revisi](docs/STALE_CLOCK_13.md). Baseline harness `review12_`
0PASS/2FAIL exit101: upper membuang carry 1,8+1,8ms dan refinement rentang
konsisten memicu Stop terminal. Kandidat menyimpan endpoint sub-ms selama
proyeksi/intersection, membulatkan keluar sekali, dan memisahkan bukti progres
persisten `clock_floor_ms` dari metadata `updated`.

Supervisor menerima refinement selama current latest masih mencapai durable
lower floor, lalu hanya menaikkan floor dengan current lower. Seluruh rentang di
bawah floor tetap `ClockRollback` terminal; disjoint antarsampel tetap ditolak
Connector untuk revalidasi. Accounting5detik dan recovery health60detik tidak
lagi memakai selisih latest-latest. Start ledger dan expiry intent memakai lower
yang sesuai agar uncertainty tidak mengecualikan fill atau memperpanjang sinyal.

FAIL→PASS: harness sebelum edit 0/2 exit101; repo final `review12_` 2/2 dan
`r13_` 8/8. Suite desktop222PASS/2ignored, engine36PASS, Clippy dan dua
`rust-workbench check` lulus. Kandidat release hash
`bce35f4f7c68b2749f88f0fd9438b73ec14f62a094156ac799e270e8b099809a`;
manifest source hash
`321a0885d418d9dbe154883138d445c42ab361449a32c3933c7e5cce6cc2b3e0`.

Jangan install/restart kandidat ini sampai review susulan menerima bukti.
Runtime tetap unit0.6.12/PID163098/hash
`0b940b1a004fcf24d013f570407c4dffa27a6f4d4e443e908c896e87d0886d54`, tidak direstart. Semua
37 file state/config/outbox/ledger hash-identik; sesi tetap final/Stop/current
null. Putaran ini tidak mengaktifkan live/paper/order, tidak melakukan probe
akun/jaringan, dan tidak mengubah risiko. Laporan/bukti final berada di
`../../outputs/binance-clock-013-20260921-review/`.

## 0.6.12 terpasang IDLE — bounded clock uncertainty

[Kontrak](docs/STALE_CLOCK_12.md), bukti lengkap
`../../outputs/binance-clock-012-20260921-Ye3Jxz/`. Runtime saat pemasangan:
unit `binance-desk-debug-012.service`, PID163098, Restart=no, binary
`0b940b1a004fcf24d013f570407c4dffa27a6f4d4e443e908c896e87d0886d54`.
0.6.11 dibackup dengan hash `1454e0a24ed5010cad73be974292529318b0d85113b8c03f2c82d60437d34ea5`.
Jangan menganggap PID/status tetap sama pada sesi berikutnya.

Akar masalah: GET time hanya menyimpan `serverTime`+Instant sesudah bounded body
read. 0.6.11 memperlakukan proyeksinya sebagai exact sehingga depth E/T+17/+14ms
menjadi future terminal. 0.6.12 `ClockSample` merekam request start sampai body
complete dan `ClockWindow` membawa lower/upper tanpa RTT/2. Sampel berurutan
diiriskan bila konsisten; disjoint membuang satu cycle lalu memakai sampel baru
sebagai baseline tanpa membawa irisan menjadi presisi palsu. Signing memakai
lower bound dan ketidakpastian>recvWindow5s gagal sebelum request bertanda
tangan/mutation.

Keputusan berbeda memakai batas berbeda: latest untuk freshness/late
signal/funding/risk/deadline, earliest untuk bukti candle close dan start sesi;
UTC harus berada pada hari yang sama. `ClockUncertain` typed adalah transient:
flat pause+consume recovery candle; dengan intent close-only/accounting dan
aturan existing maksimal2episode+60s+candle baru. `InvalidTimestamp`, schema,
urutan depth, level/spread, replay/ID mundur, auth/rate/unknown tetap terminal.
Stop/deadline/risk/trade/cancel/restart/no-blind-POST tidak dilonggarkan.

Evidence: harness lama 1PASS observed+1FAIL desired sebelum patch. Sesudah:
212desktopPASS/2ignored,36enginePASS; pipeline `rust-workbench check` kedua crate
exit0. Delapan test r12 mencakup production clock→Book parse→supervisor,
+17/+14ms, response/body/asymmetric delay, sample disjoint/clock mundur,
freshness5s, invalidfuture, flat+intent, terminalStop, candle earliest,
deadline/UTC. Suite lama mencakup expiry/cancel, schema/replay/ID,
accounting/2episode/60s/restart/no duplicate order. Validator0.6.12 menerima
paket nyata exporter0.6.8/0.6.9/0.6.10/0.6.11.

ProbeGET-only600s exit0:208snapshot,0failure,2rollover,maxage4842ms,
gap7185ms,23primaryrouteerror/10fallbacksuccess,2confirmedbar,
1freshdecision,0signal/0mutation. Dua setup invocation exit0, masing-masing dua
snapshot. Probe tidak membuktikan Wi-Fi/ISP/server sebagai sumber delay dan
bukan acceptance posisi aktif. 37 file state/config/outbox/ledger hash-identik;
sesi final,currentnull,stoptrue; settings standalone absent sebelum/sesudah.
Restart tidak membuka sesi otomatis. Aplikasi hanya tersambung baca-saja.

Yang belum selesai: tidak ada acceptance live dengan posisi, soak24jam, atau
bukti profit. Ketidakpastian yang terus menerus/disjoint akan menahan entry;
itu perilaku fail-closed. Maxage probe dekat batas5s tetapi tidak mengubah batas.
Jangan aktifkan live hanya berdasarkan compile/test/probe ini tanpa keputusan
pengguna terpisah.

## Riwayat audit06:22WIB — sesi9detik/regresiclock sebelum patch

[Laporan](../../outputs/binance-session-9s-20260921-KKGYIK/LAPORAN.md).
Sessionb1a0c0a360ca226d0,04:05:27.754→37.585,9.831ms/180menit,0trade/net0.
DepthE/T+17/+14ms di depan estimasi now → errorfuturegenerik → terminalflat.
Framebaik2.433ms kemudian tidak membatalkan Stop. Clockphase217ms versus
95ms sebelumnya, totalsnapshot517ms; bukan stale>5s. accountprepare memberi
serverTime+Instantsetelahresponse;0.6.11 menjadikan proyeksi itu exactclock
untukBookparse. Transit/clockuncertainty tidak dicatat. Rawtime/delayoneway
sebenarnya tidak terbukti, jangan mengklaim17ms kerusakan data exchange.
Harnesssalinan1PASSactual+1FAILdesired;120msdelayhanya sintetis. Tidakedit
sourceproduksi/binary/state/restart/order/probeakun;16JSON/hashunchanged.
Masih0.6.11PID141008idle; **jangan arahkan aktivasiulang sebelumclockfix**.
Tes240/probe215 terdahulu melewatkan responsclocktertunda yangmenjadikan
datafresh terlihatfuture. Berikutnya perluboundedclockuncertainty/resync,
guardumurkonservatif5s, genuinefuture/integrity/Stop tetapkeras; bukanmagic
tolerance atau mengembalikan GETtime sesudahsnapshot. Scopeauditreadonly
selesai; patchproduksi belumditerapkan.

## Riwayat pemasangan: 0.6.11 — 21 September 03:38:13 WIB

[Kontrak patch](docs/STALE_CLOCK_11.md),
[laporan](../../outputs/binance-stale-clock-011-20260921-mhBot6/LAPORAN.md).
Source/release/installed/proses0.6.11, unit binance-desk-debug-011.service,
PID141008. SHA256 `1454e0a24ed5010cad73be974292529318b0d85113b8c03f2c82d60437d34ea5`.
204desktopPASS/2ignored+36enginePASS/pipeline lulus. Probe600s215snapshot,
0failure/accountmaks2449ms/quote88–379ms/gap6808ms/2confirmedbar/0signal.
22errorprimary ditanganifallback. Setup2invocations/4snapshotPASS, finaldibawah
lock sebeluminstall; akunflat/orderBTCUSDTkosong. ATR0.0973% dan TP6ATR masih
menahan entry.16JSONfile/hashunchanged, config180menit/risiko tetap.
Tidak aktivasilive/paper/order/model; aplikasi dibukaidle. Label77menit adalah
arsip0.6.10, jangan dianggap kejadianversibaru. Rollback setelah metadata
0.6.11 muncul memerlukan decodercompatibility, jangan overwrite state.
Patch menambahkan typedStaleRead/ReadWindowExpired ke recoveryallowlist:
flatpause/consumecandle, exposureclose/accounting/60s/2episode tetap. Clock
snapshot dari GETlease sebelummutable, tanpa GETtime sesudahquote. Future,
integrity, auth/rate, unknown, orderproofstale dan Stop tetap konservatif.
Belumacceptanceposisiaktif0.6.11/24jam/profit. Hermes tetapditunda.

## Riwayat audit03:04WIB — sesi77menit sebelum patch

[Bukti/diagnosis](../../outputs/binance-session-77m-20260921-3ivGTg/LAPORAN.md).
Sessionb1a0c01896fd1ffb7,01:33:52.125→02:51:47.806,77m55,681s/180m.
Satu Long0.002BTC/pullbacktren: gross−0.05439999/fee0.16241480/funding0/
net−0.21681479USDT. EpisodeNetwork1/2 saatposisi berhasilclose/accounting,
sehat60detik kemudianWatching02:45:10.630. Jadi recovery memang sempatbekerja.
02:51:45.222depthquote7267ms ditolak lalu read_failed mengunci Stop meski
currentNone; freshframe2,633s kemudian mengakhiri sesi. Kuotaepisodebelumhabis.

Regresi: quotes::Book::parse menggabungstale/future menjadi anyhowgenerik;
read_failed nontransient→stop sebelum cekflat. Handlerdata_failed lama akan
pauseflat. Readiness masih GETdepth→premium→time serial; time7157ms membuat
quote/akun terdahulu menua. Jangan naikkan guard atau replayharga. Butuhtipe
stale terpisahintegrity, pauseflat/close-recoverbounded saatintent, clock di
luar criticalsnapshot dan regressionmatrix nyata. Networkfisik belumterbukti.

Reproduksiharness salinan:1PASSobserved+1FAILacceptance yangdisengaja; bukan
patchatauclaimtestssemualulus. Repo produksi tidakdiedit;16JSON/source/hash
binaryunchanged. SetupGET03:03PASS2snapshot akunflat/orderBTCUSDTkosong.
Runtime0.6.10PID130999 tetapidle, tidakrestart/order/activation. Jangan anggap
regresi77menit selesai; laporan mencatatperbaikanberikutbelumditerapkan.

## Riwayat: 0.6.10 dipasang idle — 21September01:20:43WIB

[Patch](docs/TRANSPORT_RECOVERY_10.md),
[laporan/bukti](../../outputs/binance-recovery-010-20260921-1JfFpw/LAPORAN.md).
User meminta langsung patch/pembaruan; tidak mengaktifkan sesi/order.
Source/release/terpasang/proses0.6.10, unit `binance-desk-debug-010.service`,
PID130999. SHA256 `d4ecd338462b0f8b7e963fc5ddfd4bffc8279342ca00bc888985c9a46e37d4f5`.
Startup baca saja.14JSON sesi/outbox/ledger identik; sesi0.6.9 tetapfinal.
Label early-stop di UI adalah riwayat lama, bukan sesi baru0.6.10.

Akar sesi terbaru: route/time gagal15099ms; frame pulih4386ms setelaherror,
21521ms sejakframe sehat. Net1trade−0.07636919USDT; Stop lama melatch seluruh
sesi meski posisi/accounting sudah aman. Bukan crash atau batch akun serial.
0.6.10 memisahkan close trade dari terminal Stop: maksimal2episode transport
bertipe saat ada intent; posisi/order kosong+ledger dua kali,60detik health
monotonic/server maju (gap≤10detik), konsumsi candle pemulihan, hanya candle
baru boleh entry. Error berulang sameepisode resetproof, bukan counterbaru.
Episodeketiga, Stop/window/restart/deadline/limit/auth/rate/schema/integrity/
protection uncertainty terminal; tidak auto-resume atau retry POST buta.
Revalidasi candle saat flat tetap pause, tidak memberi recoveryauthorityposisi.

Error persiapan tidak lagi semuanyaNetwork; HTTP401/403/400/404 GET bukan
transient. Batas persiapan GET20detik/cancel, log tahap tanpa secret. Runtime
UI dan metadata recovery opsional; exporter lama0.6.8/0.6.9 tetap diterima,
metadata final/versi ditolak bila tidak cocok. Counter bukan timeline penuh.

198desktopPASS/2ignored+36enginePASS,20faulttesPASS, fmt/clippy/doc/release/
audit489/87PASS. Probe600s201snapshot/0failure, akunmaks2705ms/quote2281ms/
gap6882ms;1confirmedbar5644ms,2freshdecisions termasukwarmup,0signal.
22errorprimary ditangani fallback; bukan bukti jaringan sembuh. Dua setup
PASS4snapshot, kedua exclusive lock sebelum rename/restart. ATR0.1659%,
hitungan0.002BTC dua arah; tidak sinyal/profit. Binary probe=final=terpasang.
Paket lama nyata diterima, eksporlatest cocok, Gitleaks paket0temuan. Scan
source1fixture sintetisUI existing identikbaseline, laporanredacted. Capture
nativeoffline diperiksa; stateprivat disamarkan. Tidak mengubah worktree lain.

Batas tetap180menit/100margin/1risk/200notional/5×/0.1funding/3sesi/5harianUTC/
maks5trade. Strategi dan freshness5detik tetap. Belum acceptanceposisiaktif
0.6.10, recoveryreal/24jam atau profit. Hermes/model ditunda; tidak ada paket,
provider/toolchainbaru. Berikutnya sesi acceptance diaktifkan pengguna,
tanpa menaikkan risiko. Backup0.6.9 di folderbukti/before, **jangan rollback
state/ledger atau menjalankan binarylama pada metadata baru tanpa cek**.

## Riwayat: 0.6.9 dipasang idle, Hermes ditunda pengguna

[Diagnosis dan perubahan](docs/ACCOUNT_SNAPSHOT_DIAGNOSTICS_09.md),
[laporan hasil](../../outputs/binance-session-78m-fix-20260920-MRMfwz/LAPORAN.md).
User menyetujui pemasangan/restart idle setelah tes dan akun aman, bukan
aktivasi trading. Source/release/terpasang/proses kini0.6.9; unit
`binance-desk-debug-069.service`, PID117808, sejak22:41:13WIB20September.
SHA256 `5e0db9cec0cf92a40ee5962215f8b4039dc7249759b1069d923034cd76fcc062`.
Startup akun terhubung baca saja;12JSONstate/outbox/ledger identik baseline.
Sesi lama tetapfinal,1trade/currentNone,config180menit, belum sesi baru.

Akar sesi 78 menit: produksi 0.6.6 (P1/P2 belum dipasang saat sesi) membaca
enam GET akun serial; symbolConfig 4310 ms membuat account 5407 ms, quote
201 ms. Stop karena DataFailure saat posisi aktif; bukan crash, holding 30
menit, batas rugi atau satu order. Net −0.03190040 USDT, fee 0.16110040,
gross +0.1292. Journal: 14 keputusan, 11 ATR, 2 tanpa pola, 1 Long. Isi receipt
mentah tidak ada; uncertain entry/exit berhasil dikonfirmasi GET 202/216 ms.
Jangan membuat parser permisif atau resend.

Patch: GET akun serentak maksimal enam, cancel tanpa detached task, cooldown
bersama tidak memendek, timing per endpoint. Counter cabang aktual tersimpan
hanya pada sesi baru dengan build enum; bukan learning atau full history.
UI/runtime dan ekspor/validator diperbarui; paket lama 0.6.8 diterima.
Guard 5 detik, risiko dan strategi tetap.
P1/P2 ikut rilis. Tidak ada dependency/toolchain/provider/model baru.

178 desktop PASS/2 ignored, 36 engine PASS; pipeline fmt/clippy/test/doc/
release/audit 489/87 PASS. Regresi serial 5407→parallel 4598 ms waktu virtual;
endpoint lain fixture sintetis. Probe 600 detik PASS: 203 snapshot/0 failure,
account maksimum 2726 ms/gap 7917 ms, satu confirmed bar 6603 ms, dua fresh
decision termasuk warmup/0 signal. Test UI ditambahkan selama probe, sehingga
hash binary probe berbeda; kode produksi sama, provenance dicatat. Dua setup
final binary PASS, empat snapshot; ATR 0.1300%, kedua arah 0.002 BTC lolos
hitungan, masih perlu pola. Cek kedua di bawah lock sebelum restart. Native
capture offline 0.6.9 diperiksa. Bukan uji order/posisi aktif/24 jam atau profit.

Hermes ditunda eksplisit oleh pengguna; jangan menahan rilis inti demi P3
atau meminjam key layanan lain. Berikutnya sesi acceptance diaktifkan pengguna,
tanpa menaikkan risiko atau perubahan strategi otomatis. Batas tersimpan:
180 menit/100 margin/1 risk/200 notional/5×/0.1 funding/3 sesi/5 harian/maks5.
Rollback binary lama setelah data diagnostics baru butuh cek kompatibilitas;
jangan overwrite ledger/state atau rollback saat exposure aktif.

## Riwayat: P2 kandidat0.6.8 — lokal selesai, saat itu menunggu pilihan P3

Pengguna meminta lanjut sampai siap live, bukan menyalakan order dari agen.
[Laporan P2](docs/EVALUATION_BRIDGE_P2_08.md),
[schema/CLI](docs/EVALUATION_PACKET_V1.md). Source/release0.6.8 mencakup P1+P2,
binary/service terpasang masih0.6.6/PID74909. Belum deploy atau sesi baru.
166desktop PASS/2ignored,36engine PASS; pipeline penuh/audit489/87 PASS.
BinarySHA256 `42fa3ee61498bf5db853f91caf5b3db86c17d702add4bdbaa19f869f430616ad`.
Artefak `~/Workspace/outputs/binance-hermes-p2-20260920-q1ZstL`.

Probe360detik PASS92snapshot/0failure,accountmaks4082ms/gap7313ms;1confirmedbar
11689ms tetapi0evaluasisegar (sudahlate saat evaluasi). Setuppertama FAILrequest
resolverpublik, keduapulih PASS2snapshot; ATR0.0816%/cap6ATR tertahan. Jangan
menulis semua cek jaringan bersih atau entryterjamin.10JSONstate+binaryterpasang
identik baseline; tidak ada deploy/order. Cek ulang readiness sebelum rilis.

Exporter hanya sesi final terakhir, shared try-lock existing, tanpa vault/
network/commit. Paket kanonik bertipe 0700/0600, ID lokal, Decimal, hash,
manifest akhir, limit/symlink/unknown-field guard. 3trade arsip cocok total
−0.34155740USDT; missing data tetap jelas. Full journal/price path tidak diimpor.
Validator proposal hanya reviewable, tidak menguji semua narasi atau biaya
yang sebenarnya; tidak ada runner/model call/apply/auto-export/scheduler.

Pertanyaan async provider/model+plafonbiaya+izin data keluar belum dijawab.
Jangan meminjam auth coding agent, mengirim paket atau memakai model berbayar.
Berikutnya P3: pilih provider, pin/install vendor terisolasi, egress/meter,
boundary tests, pilot arsip, review; lalu deployment idle. **Tidak perlu live
lebih dahulu.** Podman rootless/SELinux/seccomp tersedia, bukan bukti isolasi
Hermes sudah lolos. Sampel3trade/tes/probe bukan bukti profit atau24jam.
Jangan menyebut keseluruhan permintaan "siap live dengan Hermes" sudah selesai.

## Riwayat: P1 kandidat0.6.7 — implementasi, belum deploy

Pengguna meminta mulai langkah berikutnya. [Laporan P1](docs/RELIABILITY_P1_07.md)
memuat perubahan dan batasnya. Source/build0.6.7; aplikasi terpasang0.6.6/PID74909
tidak diganti. Jangan menyebut source/release/proses identik pada putaran ini.
Pipeline desktop150PASS/2ignored dan engine36PASS; fmt/Clippy/doc/release/audit
lulus. Probe360detik GET-only PASS:97snapshot/0error/1candle10118ms; accountmaks
3359ms/gapsehat10955ms,0sinyal. Dua setup PASS, ATR0.0851% tertahan;10file JSON
sesi/outbox/ledger dan binary terpasang identik sebelum/sesudah probe. Tidak ada order,
aktivasi sesi/paper, perubahan budget atau pemasangan Hermes/model.

History kini worker publik bounded di luar critical path. Lease clock/route
privat dibagi GET order+akun dan dibuang sebelum mutasi; guard data5detik tetap.
Metadata expired ditolak tanpa refresh di tengah bukti. Stop settlement tidak
kembali Watching. Metadata exit opsional membaca state/arsip lama; jangan
asumsikan rollback binary lama otomatis memahami hash state bermetadata baru.
P2 exporter/validator masih belum ada; Hermes tetap tidak punya akses trading.
Release SHA256 `ae0583211aec7a20b16aed8d7cce7df8616a4dafeee5d2e84b2f3205398f930d`.
Salinan kandidat dan backup source ada di
`~/Workspace/outputs/binance-reliability-p1-20260920-owA0xD`.
P1 selesai untuk handoff software; deployment idle/aktivasi live tetap terpisah.

## Riwayat: rancangan pembaruan + Hermes — 20 September 2026

Permintaan terbaru adalah **merancang**, bukan memasang atau mengaktifkan.
Acuan berikutnya: [HERMES_INTEGRATION_PLAN_07.md](docs/HERMES_INTEGRATION_PLAN_07.md).
Urutan: P1 snapshot aktif + status Stop/exit reason; P2 exporter Rust; P3 pilot
Hermes setelah sesi; P4 satu eksperimen strategi; P5 UI/scheduler opsional.
Baseline tetap0.6.6, nomor07 bukan versi rilis. Tidak ada patch produksi,
perubahan config/binary/dependency, instalasi Hermes atau run model pada desain.
Jangan menganggap provider/model/biaya/data keluar sudah disetujui.

Sesi acuan terbaru `b1a0bcf9e6d31249d`,300menit diminta,194menit5,332detik
aktual;3trade;net−0.34155740USDT;Stop `order_batch_age_ms=5069`. Sesi600menit
yang disebut pada riwayat di bawah bukan sesi terakhir lagi. Bukti:
`~/Workspace/outputs/binance-session-review-300m-20260920-ip2XD9`.
Audit sebelumnya memastikan ledger cocok dan dua GET pasca-sesi flat; tidak
mengulang probe akun pada desain ini. Tiga trade bukan bukti edge/operasi24jam.

Hermes tidak mendapat key Binance, live-state/outbox, repo writable, KWallet
atau D-Bus/host socket. Seluruh proses perlu isolasi, tidak hanya terminal.
Output hanya laporan/proposal tidak tepercaya; validator dan pengujian Rust
sebelum review. Data/angka sumber dan arsip eksperimen bukan milik memori agent.
Pengamatan live dan cron belum diaktifkan. Verifikasi desain:98tautan lokal
valid, whitespace PASS, hash56file sourceRust/config/toolchain/lockfile identik.
Tidak menjalankan test/build trading atau panggilan model; perubahan hanya
dokumentasi. Putaran desain selesai, pekerjaan implementasi selanjutnya P1.

## Riwayat: input/plan — 0.6.6, 20 September

Build release0.6.6 SHA256:
`77842c0b8492423e0403613d332960954d9e720302da1e95d70e0897ad774dcb`.
Terpasang dan aktif baca saja sejak10:52:55WIB, unit
`binance-desk-debug-066.service`, PID74909. Release/binary/proses identik;
14file state/outbox/ledger identik baseline. Unit065 idle dihentikan di bawah
lock session sebelum penggantian atomik; binary0.6.5 tersedia untuk rollback.
Tidak merestorasi state lama saat rollback tanpa verifikasi aktivitas terbaru.
Probe600detik PASS:168snapshot/0error/0rewarm,2candle dievaluasi/0sinyal,
quote193–686ms/accountmaks3916ms/gapsehat7598ms. Bukan acceptance24jam.
137tes desktop/36engine lulus; pipeline penuh/audit489 PASS. Source perubahan
utama: quotes/readiness/market, session_input/recovery-cutoff di supervisor,
planning::review dan live::check_setup. Dependency/pin Rust tetap. Laporan
akhir termasuk probe dan verifikasi instalasi: `docs/INPUT_EXECUTION_PLAN_06.md`.
Artefak/rollback: `~/Workspace/outputs/binance-input-plan-066-20260920-qut8Al`.

Sesi pengguna terbaru **b1a0baa280e4c6f3**,600menit selesai10:06:50WIB,
finished=true/currentNone/sequence0/completed0. Jangan memakai sesi180menit
b1a0b9d2ccad94ae sebagai sesi terakhir; itu riwayat terpisah dengan1trade rugi.
Config pengguna600menit/margin100/risk1/notional200/leverage5/funding0.1,
rugi3/sesi dan5/hariUTC/maks5transaksi tetap. Tidak ada live/paper oleh agen.

Quote kini depth5 dengan T exchange≤5s, E/ID monotonic dan level valid.
Candle baru harus sama dua kali≥2s, berbeda dari rewarm atas revisi baseline.
Pending baru boleh entry jika≤15s dan semua guard lolos; recovery memakan
interval terbaru meski pending. Stop/deadline/no duplicate/proteksi/ledger
tidak dilonggarkan. Planner tetapSL2/TP3–6ATR/RRnet≥1.1/ATR0.1–3%; review
aktual/logwhitelist memakai fungsi inti yang sama. Short lama tidak bisa
direplay lengkap; jangan menyebutnya salah sizing tanpa input yang hilang.
Aktivasi selanjutnya melalui GUI pengguna, bukan agen. Cek log entry review,
umur data, finalisasi, entry→SL, exit dan ledger pada sesi berikutnya.

## Riwayat: freshness/reconciliation — 0.6.5, 19 September

Unit aktif `binance-desk-debug-065.service`, binary/source0.6.5. SHA256 release,
terpasang dan proses:
`808f830df6597941cc8bdd2abd1ce7a108e8221c186c61d0abb5cb2e7b96618b`.
Sesi tersimpan `b1a0b9d2ccad94ae` finished/currentNone/completed1, config180menit
tidak diubah. Jangan mengira config masih60menit atau sesi terakhir tanpa trade;
keterangan0.6.4 di bawah adalah riwayat sebelum pengguna mengaktifkan sesi baru.

Urutan produksi metadata→GET status order→frame akun/harga→keputusan. Bukti
rekonsiliasi sekali pakai terikat intent≤5s, guard account/quote≤5s tetap.
Batch GETmaks16 satu persiapan DNS/clock; receipt gagal tidak menghapus bukti
terakhir. Market response tanpa cumQuote meminta satu GET≤5s, tanpa resend.
Missing proteksi pada open-algo terbaru meminta close-only. Stop/deadline,
ownership, residual limit, accounting dan risiko existing tetap berlaku.

129tes passed/1ignored, pipeline penuh termasuk audit489 lulus. Dua setup GET
layak bersyarat; native capture0.6.5 terverifikasi. Probe600detik168snapshot
sehat/2penolakan revisi volume candle/1rewarm, **exit1**, bukan koneksi bersih.
Umur snapshot maksimum4040ms, gap sehat10056ms, dua rollover/nolsinyal.
Tidak ada aktivasi live/paper/POST/DELETE atau perubahan akun oleh agen.
Backup binary/source/state privat tersedia di folder bukti; hash state/order/
ledger harus tetap identik pada akhir putaran. Tidak perlu tool baru.

Satu siklus produksi pengguna0.6.4 sudah terbukti (net negatif), belum semua
cabang real execution/24jam/profit. Batch proteksi aktif dan receipt mutation
patch diuji dengan exchange tiruan; probe nyata flat bukan penggantinya.
Jaringan/data yang sungguh terlambat masih dapat menghentikan sesi. Bila pengguna
memulai sesi berikut, pantau umur account/quote/order, entry→SL dan sebab Stop;
jangan memperbesar risiko atau menyalakan live dari agen.

[Laporan dan batas](docs/FRESHNESS_RECONCILIATION_05.md),
[artefak](../../outputs/binance-freshness-065-20260919-H2KhLa).

## Riwayat: setup cost-aware — 0.6.4, 19 September

Binary terpasang/release/proses sama, unit `binance-desk-debug-064.service`.
State sesi selesai `b1a0b9500b5858d7` tidak diubah; belum ada aktivasi baru.
Planner `ensemble-2-cost-aware`: SL2ATR, TP3–6ATR, rasio bersih minimal1.1
termasuk slippage TP; preview GET dan aktivasi berbagi perhitungan entry.
Form punya **Periksa kelayakan setup**; kelayakan bukan sinyal/profit.

119 tes passed/1 ignored dan pipeline lengkap lulus. Snapshot setup pertama
layak bersyarat, kedua gagal harga usang; jangan mengklaim dua PASS. Deadline
empat jam hanya tes sintetis. Tidak mengaktifkan paper atau live oleh agen.
Probe GET300detik:79 snapshot sehat/3 error harga usang, pulih tetapi exit1;
gap sehat maks6964ms dan nol sinyal. Jangan melabeli koneksi bersih.
Risiko/funding/leverage lama tidak dinaikkan; pengguna mengisi durasi180–240.
Form restore masih60 menit karena konfigurasi lama sengaja tidak ditimpa.

[Laporan, batas usulan dan bukti](docs/FEASIBILITY_LOGIC_04.md).

## Riwayat: sesi macet dipulihkan — 0.6.1, 17 September

**0.6.1 sudah terpasang**, cocok dengan binary release yang lulus pipeline.
Unit user sementara `binance-desk-recovery-061.service` aktif pada halaman
Futures. Sesi lama `b1a0ae8d70b416a2` kini `stop=true, finished=true`, tanpa
intent/posisi/transaksi. Tidak ada sesi live baru yang dimulai oleh agen.

- Recovery membaca akun/quote segar tanpa history strategi; entry tetap menolak
  candle berubah. Error disimpan dan status UI tidak menyebut rekonsiliasi aktif.
- `--finish-empty-session` hanya recovery lokal untuk sesi stop/expired yang
  belum pernah membuat intent/transaksi; dua snapshot GET, flat/no-orders,
  wallet cocok, lock/binding wajib. Jangan gunakan untuk melewati ledger sesi
  yang pernah trading. Tidak membawa transport live.
- Batas terakhir pengguna dipulihkan ke form tanpa acknowledgment. Untuk sesi
  baru pengguna memeriksa batas, mencentang dana nyata, lalu **Aktifkan sesi live**.
  Jangan mengaktifkan sesi dari agen atau menganggap seluruh saldo sebagai margin.
- **89 desktop passed, 1 network ignored**; pipeline penuh/audit489 lulus.
  Probe terpasang PASS dua GET snapshot (maks2209ms). Acceptance order/profit
  tidak diuji. Satu percobaan recovery gagal DNS tanpa mengubah state; retry lulus.
- Binary dan folder state lama dicadangkan privat di output putaran. Binding,
  config, daily, initial wallet, net/fees/funding dan completed tetap identik.
- Pemicu error spesifik sesi lama tidak tercatat. Desktop terkunci saat screenshot
  akhir: proses dan state telah diverifikasi, tampilan akhir belum dilihat langsung.

[Catatan patch](docs/SESSION_RECOVERY_FIX_01.md),
[bukti](../../outputs/binance-recovery-fix-20260917-lzZWCi/DECISION.md).

## Keputusan pengguna setelah tahap feed: Futures

Pengguna memilih **Futures** untuk memakai leverage; jangan menanyakan ulang
Spot vs Futures. Acuan implementasi tetap USDⓈ-M/USDT sesuai proyek. Default
binary terpasang sudah Futures dan diperiksa lewat `--help`.
Pengguna kini menyatakan dana Binance memang disediakan untuk proyek dan
menyerahkan strategi/rules. Jangan menanyakan ulang alokasi khusus bot. Batas
kerugian tetap berbeda dari alokasi. Pengguna sudah mengisi batas sesi pada
17 September; pertahankan config tersimpan, jangan menggantinya dengan default.

## Riwayat: sesi Futures otomatis 0.6.0

Permintaan **langsung bikin aja** sudah diterjemahkan menjadi integrasi Rust,
bukan patch label: `automatic → margin → supervisor → outbox/flow/transport →
ledger/session_store`, dengan worker `live` dan UI `live_ui`.

**0.6.0 terpasang**, `binance-desk --page futures` membuka **Sesi otomatis**.
Jendela baru dibuka melalui unit user sementara `binance-desk-auto-ui`.
Form hanya batas margin/risk/notional/funding/rugi harian+sesi/leverage/durasi/
maksimum posisi. Pengguna mengaktifkan satu sesi; software mengurus arah dan
order. Jangan mengembalikan UI menjadi persetujuan setiap order/arah tetap.
Agen tidak mengaktifkan live, membuat order, mengubah akun atau memulai paper.

- Ensemble tetap `ensemble-1`, 5m, breakout20/volume, pullback EMA20/50 dan
  range-reversal; konflik menjadi menunggu, SL2ATR/TP3ATR, filter biaya.
- Satu posisi BTCUSDT standar USDT/One-way/Single-asset/Isolated. Fee BNB ditolak.
  Isolated/leverage dapat diubah **program setelah aktivasi pengguna**, kemudian
  GET konfirmasi wajib sebelum entry. Keputusan warmup tidak dieksekusi.
- Sesi/outbox/ledger privat di `~/.local/share/binance-desk/live` (XDG didukung).
  Lock global folder, key binding hash, permissions, fsync/atomic persistence.
  Arsip ledger menyertakan config, versi/alasan strategi dan plan. Tidak ada
  key/secret mentah, raw response atau nilai akun di audit outputs.
- Startup membaca riwayat saja. Recovery UI reconcile+close tanpa entry baru.
  Close jendela meminta Stop, menunggu flat dan accounting. Crash/kill dapat
  meninggalkan posisi; SL exchange hanya ada jika sudah diterima sebelumnya.
- Ledger order/fill/child/funding berhalaman, net dicocokkan terhadap wallet
  dua kali≥5s. Missing fill, fee non-USDT, perubahan wallet asing atau ketidakpastian
  permanen menahan trade berikutnya. Maks3 exit residual setelah terminal/reject.
- **84 desktop +36 engine**, pipeline penuh/audit489 lulus. GET probe terpasang
  dua snapshot (maks1793ms); belum acceptance POST/DELETE/TRADE/profit. Engine
  lama unchanged; tidak ada dependency baru atau kode Python/JS baru.

Lihat [panduan](desktop-rs/README.md), [kontrak/batas](desktop-rs/src/futures/README.md),
[putaran](docs/AUTONOMOUS_SESSION_01.md),
[bukti](../../outputs/binance-autonomous-session01-20260916/DECISION.md).
Angka budget/leverage pengguna belum dimasukkan untuk sesi0.6.0. Jangan menganggap
blank form sebagai kegagalan koneksi atau otomatis memakai seluruh saldo.
Migrasi API key/akun, WebSocket, key rotation dan evaluasi profit belum tersedia.
Jendela versi lama dipertahankan; versi0.6.0 harus dilihat dari judul/footer.

## Riwayat: klarifikasi arah otomatis 0.5.1

User menegaskan arah mengikuti sinyal, bukan Long/Short tetap pilihan manual.
UI sekarang **Kalkulator satu posisi — contoh manual**, selector **Arah contoh**,
label budget/risiko per posisi, dan penjelasan strategi memilih Long/Short/menunggu
serta SL/TP. Ini koreksi label, bukan implementasi strategi campuran/live baru.
Jangan meminta pengguna menentukan arah dan harga SL/TP setiap transaksi pada
desain sesi otomatis. Tangani konflik sinyal dan posisi existing sebelum berbalik.
Jendela0.5.0 yang berisi input pengguna tidak ditutup. Versi0.5.1 dibuka terpisah.
[Lingkup/bukti](docs/DIRECTION_UI_CLARIFICATION_01.md).

## Riwayat: margin dan konektor Futures 0.5.0

Pengguna meminta mulai membangun dukungan margin/leverage dan order nyata,
tanpa sesi paper baru, serta menegaskan **otonom**. Jangan mendesain persetujuan
per order: produk harus mengelola entry/TP/SL/exit setelah satu aktivasi sesi
oleh pengguna. Agen mengimplementasikan dan menguji software, tidak menjalankan
transaksi keuangan pengguna dari chat.

**0.5.0 terpasang**, unit sementara `binance-desk-futures-ui` dibuka pada
**Eksekusi Futures**. Jendela lama tetap menggunakan versinya masing-masing.
`binance-desk --page futures` membuka versi terbaru. Tidak ada autostart live.

- `src/futures/margin.rs`: exact Decimal, bracket/maintenance, margin vs risiko,
  fee/funding reserve, sizing, estimasi likuidasi isolated long/short. Metadata
  bracket dapat menyebut leverage >125; parser menerima, command/form tetap≤125.
  Coefficient khusus `notionalCoef !=1` ditolak. Kalkulasi bukan harga fill pasti.
- `protocol.rs`/`transport.rs`: fixed production host, typed signed requests,
  Algo close-all TP/SL, reduce-only exit, config/query/cancel. Public DNS/time
  preparation sebelum send, reqwest retry dimatikan. Unknown tidak dikirim ulang.
- `outbox.rs`: reserve sebelum dispatch, lock/fsync/atomic replace/0600/hash,
  client ID unik, GET reconciliation, monotonic fill/terminal/child identity.
  `flow.rs`: keputusan lifecycle otomatis, tidak mengirim HTTP sendiri.
- `readiness.rs`: GET account/config/position/bracket/fee/open orders/algo,
  lalu metadata/BBO/mark/time publik. Worker menyimpan secret hanya memori,
  generation guard/disconnect. `StoreSession` tidak memicu fetch; akun UI dapat
  memasok kredensial sesi, KWallet opsional. Preflight diperbarui atas permintaan.
- `ui/futures_ui.rs`: form budget margin/risiko/notional/leverage/SL/TP/funding,
  snapshot30s, kalkulasi estimasi; capture audit menyembunyikan nilai privat.
  UI/probe selalu `ReadOnly`; **tidak ada tombol aktivasi live** pada versi ini.
- **67 tes desktop +36 engine**, pipeline penuh dan audit489 lulus. Probe nyata
  GET-only dua snapshot lulus; tidak melakukan POST/DELETE termasuk order-test.
  Engine0.1 source untouched dan checksum cocok. Rust baru tanpa Python/JS.

Sisa konkret sebelum aktivasi: supervisor berkelanjutan/stream dan pemulihan
posisi/proteksi/residual, persistensi intent/sesi terikat akun + lock per akun,
ledger fill/fee/funding/realized P&L + batas rugi sesi, integrasi strategi/sizing
segar dan UI aktivasi/stop. `NeedsAccounting` bukan izin entry berikutnya.
Outbox dan controller saja tidak membuktikan live aman atau strategi profitable.

[Kontrak serta kewajiban caller](desktop-rs/src/futures/README.md),
[lingkup putaran](docs/FUTURES_EXECUTION_ROUND_01.md),
[bukti](../../outputs/binance-futures-execution-round01-20260916/DECISION.md).

## Riwayat: mesin forward paper 0.4.0

Unit sementara **`binance-desk-paper-ui`**, PID awal 167151, menjalankan
`binance-desk --paper-run`. Halaman **Mesin paper** pada jendela **0.4.0**.
Jendela lama tidak ditutup. Tidak ada autostart atau order Binance; agen hanya
membangun/menguji simulator, pengoperasian transaksi uang nyata tetap pada pengguna.

- Sesi: `~/Workspace/outputs/binance-paper-1789562863719-167151`.
  Started 19:47 WIB, deadline 20:47 WIB (60 menit). Jangan menganggap unit aktif
  berarti paper masih aktif; periksa phase/error/timestamp pada `status.json`.
- `desktop-rs/src/paper`: GET publik BTCUSDT, metadata filter, quote bid/ask,
  funding time, candle selesai 5m; EMA20/50 + breakout, ATR14. Closed-bar cutoff
  ditetapkan sebelum mengambil klines; forming cache tidak dipromosikan otomatis.
- `execution.rs` menerjemahkan effect engine existing ke full fill virtual dan
  ack proteksi. Modal virtual1000, risiko/entry2.5, notional250, fee6bps/sisi,
  slippage≤1bp; detail dan batas di README desktop/protokol. Tidak membaca saldo
  atau kredensial akun sebagai input simulator. Signal harga publik bukan saran akun.
- Stop menunggu quote segar untuk menutup virtual. Data hilang ≥8 detik menghentikan
  entry/sesi; failure/gap/revisi tidak melakukan catch-up. Tidak ada resume otomatis.
  `status.json`, `decisions.jsonl`, `journal.jsonl` hanya publik/virtual.
- 46 tes desktop +36 engine dan pipeline penuh lulus. Probe publik tiga update
  berhenti flat, replay CLI cocok; capture native laporan tersimpan diperiksa.
  Probe awal ditolak oleh batas metadata1MiB; khusus exchangeInfo kini4MiB.
- Bukan evaluasi performa strategi: tanpa optimasi/backtest baru, hasil negatif
  lama dipertahankan. Margin/likuidasi/funding penuh, real execution belum tersedia.

[Lingkup](docs/FORWARD_PAPER_ROUND_01.md),
[bukti](../../outputs/binance-forward-paper-round01-20260916/DECISION.md).

## Riwayat: parser diperbaiki, akun nyata terhubung di 0.3.1

Pengguna melaporkan Payload generik dengan kredensial sesi-only. Validator
ASCII-only menolak lima nama Unicode dalam metadata publik Futures. Fixture
897 simbol mereproduksi bug, lalu lulus dengan Unicode alphanumeric/underscore;
control/bidi/zero-width/whitespace/duplikat tetap ditolak. Tidak ada normalisasi
nama simbol atau nilai finansial default nol.

**0.3.1 terpasang**, jendela `Binance Desk 0.3.1` aktif pada unit sementara
`binance-desk-account-fix-ui`. Worker UI berhasil membaca dua snapshot akun
nyata. KWallet kini memiliki entry pengguna; tidak ada migrasi secret oleh agen.
34 tes dan pipeline penuh lulus. stderr hanya mencatat transisi status, label
endpoint/check statis, dan penanda `ACCOUNT_READ_ONLY_CONNECTED; snapshots=2`.
Tidak ada key/secret, signed URL atau data akun di diagnosis.

Scope tetap observasi Futures, belum strategi/order/reconciliation event stream.
Jendela versi lama tetap terbuka; gunakan judul/footer 0.3.1 untuk parser baru.
[Protokol](docs/ACCOUNT_PARSER_FIX_01.md),
[bukti](../../outputs/binance-account-parser-fix01-20260916/DECISION.md).

Bagian berikut adalah riwayat sebelum patch dan koneksi akun nyata.

## Pembacaan akun Futures tersedia, autentikasi menunggu kredensial — riwayat 0.3.0

**Binance Desk 0.3.0** terpasang, unit sementara `binance-desk-account-ui` dibuka
langsung pada **Akun Futures**. Footer `v0.3.0`; jendela 0.1/0.2 yang telah
terbuka tidak ditutup otomatis. `binance-desk --page account` membuka versi baru.

`desktop-rs/src/account.rs` membatasi signed GET empat endpoint USD-M resmi;
HMAC SHA256, server time, GET/host/path/query allowlist, TLS, DNS fallback publik
tanpa kredensial, error redaction, bounded response, Retry-After dan stale.
`vault.rs` memakai Linux Secret Service/KWallet. Pair key/secret hanya masuk UI;
default sesi saja, centang KWallet opsional. Startup memuat entry aplikasi bila
tersedia. Key RSA/Ed25519 dan migrasi Windows DPAPI belum didukung.

UI menampilkan saldo/margin/P&L mengambang, posisi terpisah Hedge/One-way,
konfigurasi leverage/margin Binance. Akun tidak tersedia bukan saldo nol.
Tidak ada endpoint order/mutasi, fee/open orders/riwayat, user-data-stream,
integrasi strategi atau alokasi modal bot. Poll 10 detik bukan snapshot atomik
dan tidak boleh dipakai sebagai rekonsiliasi mesin eksekusi.

**30 tes offline + 1 transport publik nyata lulus**, pipeline penuh dan audit
489 dependency lulus. KWallet dummy roundtrip/cleanup lulus. Form sempat terkunci
pada vault kosong; state selesai-loading diperbaiki dan ditambah regresi.
Native capture menyamarkan data akun. Engine lama tetap identik menurut checksum.

**Titik berhenti: software siap, penerimaan live TERHAMBAT kredensial.** Probe
terpasang menghasilkan `ACCOUNT_CREDENTIAL_MISSING`; ini bukan kegagalan signing
yang sudah diuji ke akun nyata, dan bukan bukti akun terhubung. Pertanyaan lokasi
penyimpanan kredensial telah disampaikan; belum ada jawaban saat pencatatan.
Langkah berikut: pengguna memasukkan pair langsung di aplikasi. Jika memilih
KWallet, jalankan `binance-desk --probe-account` untuk dua snapshot read-only,
output status saja; jangan log saldo/posisi atau rahasia. Sesi-only tidak bisa
dibaca oleh probe proses lain. Jangan mengubah akun/leverage/order untuk tes.

[Panduan](desktop-rs/README.md), [kontrak](docs/ACCOUNT_DESKTOP_ROUND_01.md),
[bukti](../../outputs/binance-account-desktop-round01-20260916/DECISION.md).
Tidak ada perubahan engine/Python/web/backup, riset performa, commit atau push.

Bagian berikut mempertahankan riwayat sebelum tahap akun desktop.

## Feed pasar native selesai — 16 September 2026

`Binance Desk 0.2.0` terpasang pada launcher/binary yang sama. Jendela versi baru
dibuka lewat unit sementara `binance-desk-market-ui`; tidak ada autostart.
Jendela 0.1.0 yang sudah terbuka sebelumnya tidak ditutup otomatis. Pengguna
dapat menutup jendela lama dan memakai versi dengan footer `v0.2.0`.

Ruang kerja kini membaca BTC/ETH untuk Spot / USD-M Futures, 1m/5m/15m/1h,
240 candle, OHLCV hover, zoom/pan/reset dan jeda. Semua networking pada worker
Rust, GET publik time/klines tanpa API key; REST poll dua detik. DNS sistem lokal
gagal, fallback DNS-over-HTTPS khusus host publik + TLS resmi berhasil.
Selector pasar hanya tampilan; pengguna kini memilih Futures untuk mesin.
Modal bot, besaran leverage dan batas risiko belum ditetapkan.

16 tes dan pipeline penuh lulus, termasuk parser/gap/stale, worker failure/recovery,
inflight cancellation, pointer market controls/pan dan layout minimum. Seluruh
feed publik diperiksa lewat probe worker yang sama. Screenshot native live
Futures normal dan Spot minimum berhasil; bukan mockup. [Bukti](../../outputs/binance-market-desktop-round01-20260916/DECISION.md).

**Titik berhenti: observasi pasar native.** Belum akun/strategi/live order,
WebSocket/order book, arsip pasar atau paper trading memakai pasar nyata.
Simulasi mesin tetap fixture SYNTHUSDT terpisah dari data live. Tidak ada
perubahan engine/Python/web lama, mutasi akun, commit/push atau file backup.
Tahap berikut yang diusulkan: integrasi akun read-only pada aplikasi dengan
penyimpanan kredensial yang sesuai, lalu pengujian strategi/operasi tersendiri.

Bagian berikut mempertahankan riwayat sebelum integrasi pasar.

## Terbaru: aplikasi Linux native diprioritaskan

Pengguna mengoreksi sasaran UI: aplikasi yang dipasang dan berjalan di Linux,
bukan web lokal. Rencana HTTP viewer dihentikan sebelum implementasi. Probe
read-only fapi pada percakapan sebelumnya gagal lewat DNS sistem; lookup DNS publik
berhasil, tetapi tidak dibangun/diaktifkan feed dari hasil probe tersebut.

**Binance Desk 0.1.0 sudah terpasang**: `~/.local/bin/binance-desk`, launcher
`~/.local/share/applications/io.yuri.BinanceDesk.desktop`, ikon hicolor SVG.
Sumber [desktop-rs](desktop-rs/README.md), Rust/egui native, Wayland/OpenGL diuji.
Jalankan `binance-desk` atau menu **Binance Desk**; demo ada pada **Simulasi mesin**.
Desktop entry valid dan cache KDE diperbarui. Pemanggilan `gtk-launch` dari tool
keluar 0 tetapi prosesnya tidak bertahan; sesi aktif kemudian dibuka melalui
`systemd-run --user --collect --unit=binance-desk-ui ...`, tanpa startup permanen.
Jangan menutup jendela pengguna otomatis; closing UI mengakhiri unit sementara.

6 tes native-model/widget-pointer + pipeline fmt/Clippy/doc/release/audit lulus;
52 doctor lingkungan lulus. Screenshot framebuffer jendela native diperiksa untuk
workspace, simulation, journal, settings, serta minimum 1024×720. Eksport UI
merekonstruksi journal engine sampai cursor, memeriksa state dan menyimpan ke
Workspace/outputs. Preferences disimpan eksplisit melalui UI.

Belum ada API/market/account adapter, strategi atau order nyata. Replay 50 event
dibangun dari satu fixture sintetis yang dibekukan; tidak menjadi performa strategi.
UI tidak meminta API key, tidak menjalankan server web, tidak membuka browser.
Engine/Python/web lama tetap dipertahankan, bukan diuji ulang seluruhnya pada fase UI.
Packaging RPM/Flatpak dan distro lain belum diuji. [Paket bukti](../../outputs/binance-desktop-round01-20260916/DECISION.md).

## Handoff terbaru — inti eksekusi Rust, 16 September 2026

Pengguna meminta mulai membangun mesin trading Binance. Putaran implementasi 01
selesai: [engine-rs/README.md](engine-rs/README.md) berisi kontrak, CLI dan batas.
Library Rust menangani sizing/filter desimal, long/short satu posisi, cumulative
fills/fee/P&L, proteksi, cancel-before-exit, timeout/query, halt risiko, cleanup,
serta jurnal append-only dengan lock, hash dan replay. Effect merupakan instruksi
simulasi; tidak ada pengiriman order atau pembacaan kredensial.

36 tes dan seluruh `rust-workbench check` lulus; doctor lingkungan lulus. Lock
sempat tertahan oleh descriptor proses anak pada dua uji paralel; Drop melakukan
unlock eksplisit dan regresi baru lulus. Log awal/final disimpan. Release demo
dan CLI status dari proses berbeda memulihkan state/hash yang sama.

```bash
cd ~/Workspace/projects/binance-sniper/engine-rs
target/release/binance-engine status --journal \
  ~/Workspace/outputs/binance-execution-round01-20260916/demo/journal.jsonl
```

[Paket bukti](../../outputs/binance-execution-round01-20260916/DECISION.md).
Putaran **LANJUT**, terbatas fondasi software. Proses berikut perlu kontrak/scope
tersendiri: adapter feed/account read-only dan rekonsiliasi nyata sebelum adapter
order. Tidak otomatis membuka tuning/backtest baru atau aktivasi live.

Input masih ditunggu: tetap USDⓈ-M Futures atau Spot, modal bot, batas rugi dan
target bersih. Acuan v1 Futures bukan keputusan produk pengguna. Limitasi nyata:
tanpa koneksi/dispatcher/daemon/strategi; conditional fills/race, residual exit
recovery, funding, margin/likuidasi dan likuiditas belum diimplementasi. Caller
wajib berhenti pada event ditolak dan mengirim Tick berkala. Hasil riset lama
tetap gagal; demo sintetis bukan profit. Tidak ada perubahan Python, UI, layanan,
akun, pemasangan global, commit/push, ataupun file backup `~/data`.

Bagian berikut merupakan riwayat. Lokasi Windows yang tercatat dahulu:
`%USERPROFILE%\Desktop\Workspace\projects\binance-sniper`.

## Arah terbaru — 15 September 2026, Fedora

Lokasi kerja sekarang `~/Workspace/projects/binance-sniper`.
Pengguna menegaskan sasaran mesin trading otonom melalui API Binance, meliputi
keputusan, order, TP/SL, posisi dan risiko; strategi boleh tunggal/kombinasi.
Utamakan Rust untuk komponen baru dan pertahankan fondasi existing yang berguna.
[Arah produk](docs/BINANCE_AUTONOMOUS_SCOPE.md) memuat temuan kode dan kebutuhan.
Hermes belum dipilih/dipasang. Modal bot, batas rugi, target numerik dan pilihan
Futures/Spot belum ditegaskan. Perubahan kali ini hanya dokumentasi; tidak ada
verifikasi akun/runtime, pengujian strategi baru atau transaksi. Bagian di bawah
tetap menjadi riwayat dan bukti pekerjaan sebelumnya.

## Aktivasi observasi atas permintaan pengguna — 2026-09-14

**Sudah berjalan:** http://127.0.0.1:8765/observe, GET /api/observer. Homepage tetap Research Desk dengan tautan OBSERVASI LIVE. Menjalankan engine H4 v2 lama pada BTC/ETH, kedua config setara, bukan Turtle harian/strategi baru. Warmup tidak trading; posisi/fill next-open dan stop hanya model OHLC yang direkonstruksi saat close H4. Net lengkap sengaja null karena funding live belum lengkap; jangan menyebutnya forward fill atau keuntungan akun.

24/24 feed publik pulih lewat REST fallback dengan `public_dns_fallback: true`: DNS-over-HTTPS aplikasi hanya untuk GET publik pada fapi.binance.com, TLS tetap hostname resmi. Resolver Windows/private reader/WebSocket tidak diubah. Private account saat verifikasi AUTH_UNAVAILABLE; jangan menyebut angka posisi/order nol. Tidak ada mutation route atau aktivasi order.

State observer baru: `state/channel-observer.sqlite3`. Menyimpan input, waktu penerimaan dan warmup/observation; startup merekonstruksi state deterministik. Identitas mencakup engine, adapter dan konfigurasi; perubahan kode memerlukan migrasi eksplisit, bukan menghapus DB. Revision/gap fail closed. Pengambilan REST menambah jumlah bar jika tertinggal (bounded warmup limit), tidak hanya mengambil tiga bar saat recovery. Private account fetch sekarang tugas background sehingga tidak memblokir startup feed publik.

Runner: `scripts/run.ps1`; stop/status memakai skrip proyek yang memeriksa PID/port. Layanan aktif pada akhir verifikasi; belum dipasang sebagai startup Windows. Launcher background ditolak lingkungan, runner proyek biasa berhasil. Jangan menghentikan sesi runner aktif jika pengguna masih mengamati. Tidak ada commit/push atau instalasi baru. httpcore 1.0.9 yang sudah terpasang/terkunci kini dicatat sebagai dependency langsung.

283 Python / 23 JS passed. Restart aktual mempertahankan session 17:27:07 WIB dan input BTC/ETH masing-masing 220 warmup / 0 observation ketika diuji. Harga bergerak; empat model cocok API; browser offline/reconnect serta desktop 1440/mobile 390 tanpa overflow terverifikasi. Putaran 01 tetap TOLAK; bukan profit proof. [Catatan aktivasi](docs/LIVE_OBSERVER_ACTIVATION.md).

## Tanggapan audit 01 — funding v2, dasar strategi belum cukup (2026-09-14)

Temuan A1/A2 ditanggapi melalui perbandingan tiga sumber primer/dua mekanisme alternatif dan tabel adaptasi lama. Keputusan **BELUM ADA DASAR CUKUP**; tidak memilih atau mengimplementasikan strategi berikutnya. Putaran 01 tetap **TOLAK**. Rancangan evaluasi berikutnya belum dijalankan dan bukan otorisasi trial.

Koreksi lokal A3: `round01/engine.py` memakai `funding_policy_version=ownership-interval-v2`. Funding sesudah gap-exit pasti bernilai nol; boundary entry/exit dan stop intrabar tetap debit-only jika ambigu. Entry-jitter yang pasti sesudah entry dan tanpa stop memakai debit/kredit normal. `round01/study.py` memblokir CLI protokol v1 sebelum membaca dataset/membuat output agar source v2 tidak dipasangkan dengan hasil v1. Untuk reproduksi v1 gunakan source beku paket asal; jangan menjalankan CLI aktif lalu menamai hasilnya v1.

Tes aktual: 137 unit terkait lulus pada koreksi pertama; regresi lengkap 267 Python lulus, satu warning deprecation dependency. Ditambahkan 112 kasus sintetis parametrik. Probe membuktikan +1/+999 ms memenuhi jadwal runner; +1 jam hanya valid di API engine. Tidak ada P&L historis ulang/final, perubahan strategi/gate/UI/layanan/dependency, commit/push, akses secret atau mutasi akun. Hasil v2 historis belum tersedia; ketiadaan gap-stop dalam delapan trial v1 tidak membuktikan semua hasil v2 akan identik.

[Tanggapan dan bukti](../../outputs/binance-sniper-audit-response01-20260914-150514/AUDIT_RESPONSE.md), [keputusan](../../outputs/binance-sniper-audit-response01-20260914-150514/DECISION.md). Snapshot/source/protokol/results putaran asal dipertahankan; paragraf di bawah adalah riwayat v1 dan pekerjaan sebelumnya.

## Putaran mandiri 01 — TOLAK (2026-09-14)

Instruksi `PROMPT-ASTRA-MESIN-TRADING-01.md` sudah dijalankan. Satu mekanisme kanal tren H4, dua konfigurasi 20/10 dan 55/20, BTC/ETH, BASE/STRESS. Kode offline baru `src/binance_sniper/round01/`; engine yang sama menghasilkan ledger dan proyeksi viewer. Tidak ada perubahan runtime/route atau observer permanen.

Seleksi 2025-12-20–2026-06-15: 20/10 = 72 trade, net indeks +0.00634892 / stress -0.03305116, PF 1.00976, DD indeks MTM 38.17%; 55/20 = 35 trade, net +0.03723795 / stress +0.01820619, PF 1.10187, DD 33.83%. Keduanya gagal gate yang dibekukan; BTC negatif, bootstrap lower5 negatif; 55/20 terkonsentrasi pada tiga trade (75.45%) dan development negatif. Semua angka unitless/exposure index, **bukan return/equity akun atau USD**. Evaluasi final tidak dijalankan karena tidak ada kandidat eligible. Seluruh data tetap eksploratif/already-seen.

Paket: [AUDIT_PACKET.md](../../outputs/binance-sniper-round01-20260914-135512/AUDIT_PACKET.md), [protokol/hash](../../outputs/binance-sniper-round01-20260914-135512/PROTOCOL.md), [command](../../outputs/binance-sniper-round01-20260914-135512/COMMANDS.md). Delapan trial + reproduksi byte-identik, 155 Python/19 JS passed; 12 prefix actual-data dan rekonsiliasi independen PASS. Satu fixture unit awal gagal lalu diperbaiki; hasil awal dipertahankan. Viewer standalone actual render/20 frame berurutan terverifikasi; belum diintegrasikan ke dashboard lama. Hasil vendor dan kegagalan lama dipertahankan.

Kendala: tiga public exchangeInfo GET timeout; historical contract filters, executable sizing/margin/liquidation, tick fills/orderbook dan fee akun belum dimodelkan. Tidak ada order/test/cancel, mutasi akun, perubahan kredensial/ipRestrict, instalasi, commit/push, atau aktivasi Algo Trading. Putaran berhenti dengan TOLAK; tidak otomatis membuka riset/konfigurasi baru. Catatan di bawah mendahului putaran ini.

## Arah tetap pengguna (2026-09-14)

**Membangun mesin sendiri dengan mengumpulkan dasar terbaik; bukan membeli robot.** Jimmy dan EA lain adalah referensi/pembanding, bukan produk yang akan otomatis dibeli. Aturan proyek tersimpan di `AGENTS.md`; [tujuan dan metode riset yang diusulkan](docs/RESEARCH_DIRECTION.md). Hasil vendor tidak boleh dianggap hasil implementasi kita atau Binance. Perubahan arah ini hanya dokumentasi; belum memulai shortlist baru, pemasangan, pengujian EA lain atau trading live.

Penegasan terbaru: pengguna meminta **batas kerja, hasil dan goal nyata; tidak berputar dalam loop riset/redesain**. Tiap putaran harus berakhir dengan LANJUT, TOLAK, atau BELUM DAPAT DIPUTUSKAN beserta bukti. Catat batas scope/percobaan/waktu dan syarat selesai sebelum bekerja; jangan menambah fase atau menurunkan kriteria diam-diam. Angka modal Binance, toleransi kerugian, target return dan tenggat belum disepakati. Penutupan putaran gagal tidak sama dengan tujuan akhir profit tercapai. Percakapan persetujuan arah ini belum memulai riset baru atau mengizinkan transaksi.

## Current handoff: MT5 evidence pipeline (2026-09-14)

Main UI is `web/tester.html` / `web/assets/tester-*`; Binance model remains `/market`. New `src/binance_sniper/tester/` contains offline parser/store/CLI. GET `/api/tester`, `/api/tester/runs/{id}`, `/api/tester/compare?left=...&right=...`; no raw report, upload or trading API. Context templates: `config/tester-*-context.json`.

Store now has 1 candidate report and 1 old SniperFX parser check. User completed manual MQL5 login. Official product 122237 / Jimmy Peter Eriksson / v5.20 demo installed; one preregistered visual Strategy Tester run completed with untouched vendor inputs. Run ID `2c033220a6748fb8e65adfa55466b4f5f15408b4b339c96b6ff5c3ecb61229ba`: 22 trades / 44 deals, net +671.44 USD, final 10671.44 USD, PF 1.98, reported equity DD 3.70%. API and actual dashboard verified. [Run details, artifact paths and next action](docs/DEMO_RUN_01.md). Do not ask for MQL5 login again unless a new authentication failure is actually observed. Do not purchase or attach to a live chart.

[Protocol, import commands and limitations](docs/TESTER_PIPELINE.md). Last code test suite: 130 Python / 19 JS; no application code or dependency changed during this demo run. This work and prior reference redesign remain uncommitted; preserve dirty worktree. Do not claim a clean repository, finished live bot or validated Jimmy profitability. Real-tick coverage starts late for XAUUSD, DE40 did not participate, and vendor logs show randomization whose seed is not exposed. Investigate these before broader/repeated tests; keep all results without profit-based selection.

## Previous handoff: Range Breakout Reference (2026-09-14)

The homepage and active model have changed. `reference/engine.py` is the sole shared rule engine (live reconstruction, offline replay, synthetic demo); `reference/live.py` owns a separate SQLite session ledger and bounded recorded-chart reads. `MarketView` replaces the old Scanner in Runtime. Only `range-reference` is spawned; `research` and `strategy-lab` are retired, with old code/results retained below for reproducibility.

`web/range.html`, `range-terminal.js`, `range-model.js`, and `range-terminal.css` are the active UI. Old assets remain only for archived reproducibility tests. GET `/api/reference`, `/api/reference/replay`, and bounded `/api/reference/session?symbol=...&day=...&dataset=...` serve model data. Historical dataset IDs are allowlisted from the matching report; request-local read-only SQLite connections avoid cross-thread reuse. `/api/lab` returns the archived report, not an active shadow engine.

No fixed TP/trailing or four-candle horizon. Do not describe this as an adaptive trailing system: positions retain the initial stop and close by session end, matching the selected public reference mechanism. Binance instruments, UTC clock, daily/weekend operation, and M5 fill model are explicit adaptations. See `docs/REFERENCE_ENGINE.md` before changing rules.

Offline `.venv\Scripts\python.exe -m binance_sniper.reference.study` uses two frozen already-seen datasets without parameter search. `state/reference-history.json` is version-matched for UI; `Workspace/outputs/binance-range-reference-results.json` includes all sessions. 2,685 closed modeled positions / mean -0.075788339R / stress -0.185124779R: still negative, **not profit-validated**. One ambiguous session excluded explicitly. Do not select DOGE solely because its retrospective subgroup is slightly positive.

UI state and market freshness are separate: reference-capture age must expire even when market WebSocket snapshots keep arriving. Completed modeled results cannot be rewritten after data revisions; mark them. Live net including funding remains null. Actual account positions/orders stay separate. No trading API mutations or external dependencies added.

## Archived handoff (earlier phases)

Entrypoint: `binance_sniper.app:app`, bound to `127.0.0.1:8765`. The service is observation-only. Binance mutations are not implemented and non-allowlisted paths are rejected before network access.

Runtime design:

- `MarketStream` owns WebSocket connect/watchdog/reconnect/renewal.
- Twenty-four independently supervised REST feed loops maintain live fallback with timeframe-specific cadence.
- `ClosedBarCache` separates closed decision bars from visual preview and exposes quality/freshness counters.
- `Scanner` owns the causal lifecycle; `snapshot()` is pure and cannot create candidates or advance outcomes.
- `ZoneLedger` and `FourBarTracker` persist atomically under `state/`.
- `/api/replay` runs a deterministic production-pipeline fixture with no external mutation.
- `/api/account` computes snapshot age at read time and preserves last-known data as `STALE` on refresh failure.
- `/assets` serves local terminal CSS and JavaScript. `terminal-model.js` contains testable view selection/freshness rules; `terminal.js` owns the interactive chart and mode lifecycle.
- `presentation.py` projects backend scanner gates into Indonesian explanations. `MIN_EFFICIENCY` is shared with the existing H4 strategy rule.
- Snapshot includes zone identities and public tick-size metadata. Frontend overlays must match the selected zone and candidate; archived candidates are explicitly selected.
- `/api/replay` also exposes six independent synthetic frames for step/play controls. Live account information is hidden in replay mode.

On this network, Binance market WebSocket connections remain silent. The runtime therefore reports the WebSocket substate honestly while keeping all validated REST feeds independently live. Do not infer strategy profitability from transport health or replay correctness.

## Phase 2 handoff

- `research/store.py`: local SQLite WAL, schema v1, datasets/candles/funding/runs/decisions/signals/outcomes. Composite keys deduplicate restarts; completed modeled outcomes cannot be overwritten. Each run is isolated by mode and source fingerprint.
- `research/download.py`: public historical klines pagination/resumption, validated OHLC/timestamps, complete feed coverage and checksums, public funding history. Existing 90-day dataset is retained under `state/`, not tracked in git.
- `research/study.py`: sequential closed-M5 clock, higher-timeframe promotion only after their close, memory-only production Scanner, explicit development/holdout purge and embargo. Warmup cannot backdate a newly discovered zone or trigger.
- `research/model.py`: next-open hypothetical entry, directional SL/TP, four-bar maximum, gap handling, stop-first ambiguity, per-side fee/slippage assumptions, funding accounting with conservative boundary treatment. This is separate from the legacy gross four-bar excursion tracker.
- `research/recorder.py`: supervised `research` task; captures live closed bars and new decisions, recovers pending observations from local history, journals delay/coherence, and refreshes reports every 30 seconds. No exchange requests from this module. SQLite lock timeout is 200 ms to bound feed blocking.
- `research/report.py`: frozen source/config fingerprint, grouped event metrics and gate/state counts. Missing funding means no all-cost `net_r`; live before-funding metrics have separate labels.
- GET `/api/research` and `/api/research/event` serve reports and bounded recorded M5 evidence. Phase 3 extends the window to 50 preceding + actual horizon (4 or 8; hard cap 82) candles. Event endpoint uses parameterized SQL and bounded IDs. No run-start, mutation, or order route.
- `web/assets/research-panel.js` + `research.css`: separate historical/live sources, evidence dialog, explicit hindsight shading. Memoized renders preserve details/scroll while live snapshots stream.

Completed study: `history-6d32c2f80979f7e5dec0486f`. Source fingerprint: `17473b25bc8a6cf97154ea957a9810481a2688d4b59f5da64a35500292ceae72`. An earlier instrumentation run remains in SQLite for provenance; it produced the same 27 signals and outcomes, not an independent validation sample. Latest report is displayed first per mode.

Do not silently tune against the already-viewed holdout or turn this baseline into a live trading engine. Next research should predeclare a new pattern hypothesis, examine the origin-centered zone geometry and 20-minute horizon, and reserve genuinely new validation data. A change of source code creates a new journal/study version.

## Phase 3 handoff — NO_PROMOTION

The above Phase 2 suggestions were tested in Phase 3; they are no longer untested ideas. Read `docs/PHASE3_PROTOCOL.md` and `docs/PHASE3_RESULTS.md` before another strategy change. Both the original 90 days and the new 360 days have now been seen. Do not recycle either as a fresh holdout.

- `strategy_lab/strategies.py`: eight immutable variant definitions, causal H1/H4 context, retracement/retest/sweep areas, directional trigger, volatility stop floor, target-room/cost checks, area expiry/cancellation, one trigger per area, same-symbol horizon cooldown. Shared `PatternEngine.step` drives historical replay and experimental live observation. Score is a cost-feasibility heuristic, NOT a probability of profit.
- `strategy_lab/evaluation.py`: training-prefix selection, uniform eight-bar purge/embargo, three expanding walk-forward folds and final holdout. Deterministic seven-day calendar-block bootstrap clusters symbols together. Failed gates are reported, not relaxed after seeing results.
- `strategy_lab/runner.py`: fixed dated protocol, feed checksum verification, source/config/protocol/data-manifest fingerprints, resumable per-symbol historical replay and daily gate counts. Historical trials live in `lab_trials/lab_gates/lab_progress`; their `LAB` runs do not pollute Phase 2 latest reports.
- `strategy_lab/live.py`: supervised `strategy-lab` task captures only new, coherent closed-M5 decisions after its start. Eight hypotheses remain EXPERIMENTAL_MONITOR; this is data collection, not strategy promotion. Signal, modeled outcome, and engine state commit in one SQLite transaction. A failed capture propagates to supervision/reload; completed outcomes stay immutable. Source-bound shadow namespace also fingerprints the adapter. SQLite busy timeout: 200 ms.
- GET `/api/lab`, `/api/lab/events`, `/api/lab/chart`: stored result, latest 30 variant events, and at most 80 actual shadow M5 candles. Parameter bounds and unknown-ID 404s tested. No mutation routes.
- `lab-panel.js`: historical results/experimental observation separation, evidence actions, error/stale handling, preserved details/scroll. Main chart and original scanner remain baseline; the panel says so explicitly. `research-panel.js` shares recorded event dialogs and draws frozen area bounds only when present.
- `research/download.py`: explicit dated end boundary and funding pagination past 1,000 records. `engine.py`: revision-sensitive memoization of closed H4/H1 context; baseline rules unchanged. `research/model.py`: conservative funding handling at exact gap-exit boundary.
- Internal legacy exit identifier `FOUR_BAR_CLOSE` is retained for report compatibility; actual numeric horizon is `costs.horizon`. Tests prove eight-bar completion uses the eighth close and the UI labels it `TIME STOP / 8 M5`. Do not infer four bars from that old identifier.
- `.gitattributes` preserves the registered protocol's LF bytes so its fingerprint is stable on Windows checkout. Other source/config fingerprints normalize text newlines.

Final trial `lab-e98337744f4aefe8ba8625fc`, dataset `binance-1750377600000-1781481600000-v1`. Earlier `lab-6b33792f035df1f24d6cbbac` is retained; the second run added missing config provenance, with identical validation, baseline and variant results. These are the same experiment, not two independent validations. JSON: `Workspace\outputs\binance-sniper-phase3-360d.json`.

Runtime at handoff: verified PID 25112, `http://127.0.0.1:8765/?tab=lab`. Keep `ipRestrict=false` as the existing warning policy. No credentials, leverage, margin, orders, or positions were changed. Multi-day shadow soak, live funding reconciliation and executable fills remain absent. Next work needs a genuinely new causal hypothesis/data source and a newly declared validation plan; do not cherry-pick the positive SOL subgroup of 19 holdout events or tune the already-opened holdout until it looks profitable.
