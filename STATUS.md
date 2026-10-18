# Status

Updated: 2026-09-21 WIB

## 0.6.14 terpasang dan terverifikasi IDLE — 09:20:44 WIB

[Desain](docs/STALE_CLOCK_14.md) dan
[review independen](../../outputs/binance-review-014-20260921-nhQrIi/REVIEW.md)
menutup blocker anchor aktivasi/recovery. Hasil offline tetap desktop227PASS/
2ignored, engine36PASS, review13 3/3, review12 2/2, r13 8/8, r14 2/2, serta
uji clock produksi/wall-time64detik1PASS tanpa command tambahan.

Gerbang pra-pasang lulus. `cargo audit` melakukan fetch RustSec resmi dan update
indeks crates.io; desktop489 dan engine87 dependency exit0. HEAD advisory
terbaru yang tersedia tetap
`d5c17953a895cf19e8d3ce66eaa42b6fcfe1fb16` tanggal19September2026.
ProbeGET-only600detik exit0:209snapshot/0failure,2rollover,
maxsnapshotage2792ms,maxhealthgap5753ms,2confirmedbar/2freshdecision/0signal,
0mutation.22diagnostik rute primer menghasilkan10 fallback success dan tidak
membatalkan snapshot. Setup dua snapshot exit0 memakai batas tersimpan
100margin/1risk/200notional/5x/0.1reserve/3sessionloss/5daily/5trade; fresh GET
membuktikan posisi yang diperiksa kosong serta order/algo BTCUSDT kosong dan
setup viable. Pending order simbol lain belum dibuktikan; tidak ada bukti bahwa
order asing ada. Label probe `within_30s_recovery_budget` adalah metrik probe,
bukan guard `Healthy`: continuity tetap10detik dan health tetap60detik;
gap terukur5753ms berada di bawah10detik.

Kandidat/release/installed/proses identik SHA256
`5903daea453675bd76b6df122e48973a7bbc8d8f1dedbfd83273fa95b020c19f`.
Unit `binance-desk-debug-014.service` active/running, Restart=no, PID201536,
start09:20:44WIB. Unit012 inactive/not-found. Journal startup hanya koneksi
baca-saja,0 `LIVE_SESSION_EVENT`,0 `LIVE_ORDER_DISPATCH`; tidak ada sesi
otomatis.37 file state/config/outbox/ledger hash-identik; state tetap finished,
Stop,currentnull,decisionnull,recoveryinactive. Backup0.6.12 hash
`0b940b1a004fcf24d013f570407c4dffa27a6f4d4e443e908c896e87d0886d54`
beserta launcher/unit ada di folder bukti.

Tidak ada aktivasi live/paper, order/cancel, perubahan akun, config, risiko,
leverage, state atau backup data. Instalasi IDLE bukan acceptance posisi nyata,
soak24jam, atau bukti profit. [Laporan dan bukti](../../outputs/binance-install-014-20260921/LAPORAN.md).

## Kandidat 0.6.13 teruji, belum dipasang — revisi review clock

[Desain](docs/STALE_CLOCK_13.md). Review independen 0.6.12 menemukan dua
masalah: refinement wide→narrow yang konsisten dibandingkan lewat dua nilai
latest sehingga dianggap rollback, dan RTT/proyeksi pecahan dibulatkan ke bawah
secara terpisah sehingga upper tidak selalu mencakup waktu sah. Harness review
sebelum edit merekam 0PASS/2FAIL, exit101.

Kandidat 0.6.13 mempertahankan endpoint nanodetik sampai tepi keputusan lalu
membulatkan lower ke bawah dan upper ke atas. `clock_floor_ms` opsional menyimpan
lower bound terbesar yang terbukti; rollback nyata tetap terminal bila seluruh
rentang baru berada di bawah floor. `updated` kini hanya high-water metadata.
Wait accounting memakai current-lower versus saved-upper, health60detik memakai
waktu lokal plus current-lower versus first-upper, dan rentang ledger mulai dari
lower frame intent. Deadline sesi/UTC/candle/recovery/no-duplicate tetap diuji.

Regresi final: `review12_` 2PASS dan `r13_` 8PASS. Suite desktop 222PASS/
2ignored; engine36PASS. Clippy final dan `rust-workbench check` kedua crate
lulus termasuk fmt/doc/release/audit489/87. Kandidat release 0.6.13 SHA256
`bce35f4f7c68b2749f88f0fd9438b73ec14f62a094156ac799e270e8b099809a`;
manifest source SHA256
`321a0885d418d9dbe154883138d445c42ab361449a32c3933c7e5cce6cc2b3e0`.
Tidak ada probe jaringan karena bukan bukti untuk dua sifat deterministik ini.

Status putaran ini sengaja hanya kandidat. Aplikasi terpasang tetap 0.6.12,
unit `binance-desk-debug-012.service` tetap PID163098 sejak07:05:11WIB dan
binary terpasang tetap hash
`0b940b1a004fcf24d013f570407c4dffa27a6f4d4e443e908c896e87d0886d54`;
service tidak direstart. 37 file
state/config/outbox/ledger identik dengan baseline, sesi tetap finished/Stop/
currentnull. Tidak ada live/paper/order/perubahan akun/risiko. Bukti lengkap di
`../../outputs/binance-clock-013-20260921-review/`.

## 0.6.12 terpasang IDLE — perbaikan rentang clock

[Desain](docs/STALE_CLOCK_12.md), bukti dan laporan di
`../../outputs/binance-clock-012-20260921-Ye3Jxz/`. Akar regresi 0.6.11 adalah
`serverTime` yang dipasangkan dengan `Instant` setelah body selesai lalu dipakai
sebagai waktu exact. 0.6.12 mencatat awal request sampai body selesai dan
membawa rentang `[serverTime, serverTime+elapsed]`; tidak ada asumsi RTT/2 atau
toleransi future tetap. Depth +17/+14ms menjadi `ClockUncertain`, frame dibuang
dan flat session direvalidasi. Timestamp di atas batas atas, schema, replay,
ID mundur dan integritas tetap terminal. Signing memakai batas bawah dan
menolak ketidakpastian >recvWindow5detik sebelum mutation.

Tes pra-patch merekam 1PASS perilaku gagal+1FAIL acceptance. Setelah patch:
212desktopPASS/2ignored dan36enginePASS; dua `rust-workbench check` lulus
fmt/clippy/test/doc/release/audit. Fault test melewati pembentukan clock→parse
depth→supervisor, +17/+14ms, asymmetric/body delay, clock jump, lease expiry,
batas5detik, invalid future, schema/replay/ID, flat/intent, recovery/Stop,
candle/deadline/UTC dan no duplicate order. Paket evaluation0.6.8–0.6.11
divalidasi oleh0.6.12.

ProbeGET-only600s:208snapshot/0failure,2rollover, maxage4842ms,
maxhealthgap7185ms,23primary route errors dan10fallback success;2confirmedbar,
1freshdecision,0signal/0mutation. Dua invocation setup masing-masing PASS dua
snapshot, akun/konfigurasi aman untuk observasi; ini bukan acceptance live.
37file state/config/outbox/ledger identik sebelum/sesudah, sesi final dan
`current=null`; settings standalone memang tidak ada sebelum/sesudah.

Release/source/installed/proses identik. SHA256
`0b940b1a004fcf24d013f570407c4dffa27a6f4d4e443e908c896e87d0886d54`;
unit `binance-desk-debug-012.service`, PID saat pemasangan163098, Restart=no.
Backup0.6.11 hash `1454e0a24ed5010cad73be974292529318b0d85113b8c03f2c82d60437d34ea5`.
Tidak ada live/paper/order/perubahan akun. Belum acceptance posisiaktif,
operasi24jam atau profit; jangan menyimpulkan sumber fisik delay dari probe.

## Riwayat audit06:22WIB — regresi clock0.6.11 sebelum patch

[Sesi9,831detik](../../outputs/binance-session-9s-20260921-KKGYIK/LAPORAN.md):
04:05:27.754→04:05:37.585,0intent/0trade/net0. Estimasi clock menempatkan
E/T depth17/14ms di depan now; error future generik melatch Stop saat flat.
Siklusgagal517ms/clock217ms, freshframe2.433ms kemudian tetapFinished.
Clockdipasangkan denganInstant sesudahresponse, tanpa ketidakpastian transit;
proyeksi itu dipakai acuan absolut sejak0.6.11. Bukan input atau crash.
Reprosalinan source1PASSactual+1FAILacceptance; delay120ms sintetis bukan
ukuranproduksi. Belum patch/restart/order/probeakunbaru;16JSON/source/binary
unchanged. Runtime tetap0.6.11PID141008idle. Penilaian siaplive sebelumnya
ditarik: perlu perbaikan ketidakpastian clock, bukan toleransi future buta.

## Riwayat pemasangan: 0.6.11 — 03:38:13 WIB

[Patch stale/clock](docs/STALE_CLOCK_11.md) mengembalikan pause saat flat untuk
data usang bertipe, mempertahankan terminal untuk integritas/future, dan
menghapus GET waktu setelah snapshot. Guard 5 detik dan risiko tidak berubah.
204 desktop PASS/2 ignored + 36 engine PASS; pipeline kedua crate lulus.
Probe600s PASS215snapshot/0failure, accountmaks2449ms/quote88–379ms,
gap6808ms,2confirmedbar/2freshdecision/0signal;22primaryerror ditangani fallback.
Dua setup PASS4snapshot; terakhir di bawahlock, akunflat/orderBTCUSDTkosong.
ATR0.0973% dan biaya/TP masih menahan entry, bukan alasan mengubah guard.
Source/release/terpasang/proses0.6.11 identik, PID141008,
unit `binance-desk-debug-011.service`.16JSON state+daftarfile unchanged.
Config180menit dan batas risiko tetap, tidak aktivasilive/paper/order/model.
[Laporan dan bukti](../../outputs/binance-stale-clock-011-20260921-mhBot6/LAPORAN.md).
Belum bukti recovery posisi produksi0.6.11, operasi24jam atau profit.

## Riwayat audit03:04WIB — regresi flat/stale pada0.6.10 sebelum patch

[Laporan sesi77menit](../../outputs/binance-session-77m-20260921-3ivGTg/LAPORAN.md):
77m55,681s/180m, satu trade/net−0.21681479USDT. Recovery transport posisi
berhasil kembaliWatching02:45:10, tetapi02:51:45quote7267ms masuk error
generik lalu Stop terminal **saat flat**. GET time publik7157ms setelah depth
konsisten membuat frame menua. Ini regresi handler0.6.10 versus pause lama,
bukan input/batas rugi/kuota2episode. Reproduksi di salinan source:1PASS
perilaku aktual+1FAIL acceptance yang disengaja. SetupGET03:03PASS2snapshot,
16JSON/source/binary unchanged; tidak restart/order/patch produksi. Runtime
tetap0.6.10/PID130999; jangan menyebut masalah terbaru sudah diperbaiki.

## Riwayat: 0.6.10 terpasang — pemulihan transport terbatas

- [Patch](docs/TRANSPORT_RECOVERY_10.md) sesudah sesi0.6.9 berakhir34m22,750s:
  tutup trade/accounting tidak lagi otomatis mengakhiri sesi untuk error
  transport bertipe. Maksimal2episode dengan intent, kemudian60detik data
  sehat/gap≤10detik dan hanya candle baru. Stop/deadline/risiko/proteksi tak
  pasti/schema/auth/rate tetap terminal. Restart tidak auto-resume.
- Klasifikasi error persiapan/HTTP diperbaiki, total time/DNS GET≤20detik,
  log tahap disamarkan, UI/metadata/ekspor kompatibel0.6.8/0.6.9. Guard5detik,
  strategi, biaya dan batas pengguna tetap. Tidak ada dependency/model baru.
- **198desktop PASS/2network ignored,36engine PASS**; pipeline/audit489/87
  PASS.20tes fault khusus PASS; reproduksi Stop lama dan HTTP401 FAIL→PASS.
- Probe600s PASS201snapshot/0kegagalan snapshot, akunmaks2705ms,
  quotemaks2281ms, gap6882ms,1candleconfirmed/2freshdecision termasukwarmup,
  0signal.22kegagalan primary ditangani fallback; jaringan tidak bebas error.
- Dua setup PASS4snapshot, terakhir di bawahlock: tanpa posisi akun dan
  order/algoBTCUSDT. ATR0.1659%/qtyhipotetis0.002BTC, bukan sinyal/profit.
- **Source/release/terpasang/proses0.6.10**, unit `binance-desk-debug-010.service`,
  PID130999 sejak21September01:20:43WIB; hashidentik,14JSONstate unchanged.
  Startup baca-saja, tidak aktivasi live/paper/order atau perubahan akun.
  Config tetap180menit/100margin/1risk/200notional/5×/0.1funding/3sesi/5hari/5trade.
- [Laporan lengkap](../../outputs/binance-recovery-010-20260921-1JfFpw/LAPORAN.md).
  Belum acceptance recovery posisi nyata/24jam atau profit; Hermes ditunda.

## Riwayat: 0.6.9 terpasang — snapshot akun dan diagnosis keputusan

- [Investigasi/patch](docs/ACCOUNT_SNAPSHOT_DIAGNOSTICS_09.md): sesi produksi
  0.6.6 berakhir78menit32,458detik dari180, bukan crash. GET symbolConfig4310ms
  + urutan enam GET membuat account5407ms>5000. Satu trade/net−0.03190040USDT.
- Enam GET akun kini serentak terbatas; cooldown/cancel, log waktu endpoint,
  counter keputusan bertipe/versi sumber dan UI; validator paket0.6.8 tetap
  kompatibel. P1/P2 ikut rilis. Guard5detik, strategi dan batas uang tetap.
- **178desktop PASS/2network ignored,36engine PASS**, pipeline/audit489/87 PASS.
  Probe600s PASS203snapshot/0error/accountmaks2726ms/gap7917ms,1confirmedbar,
  2freshdecision termasukwarmup/0signal. Bukan bukti posisi aktif atau24jam.
- Dua setup GET final PASS, total4snapshot; akun tanpa posisi/orderBTCUSDT.
  ATR0.1300% saat22:40–22:41, hitungan kedua arah lolos0.002BTC; bukan sinyal.
- **Terpasang/proses0.6.9**, unit `binance-desk-debug-069.service`, PID117808
  sejak22:41:13WIB. Release/binary/proses hashidentik,12JSONstate identik.
  Tidak mengaktifkan sesi/order; startup baca saja. Config pengguna tetap180
  menit/100margin/1risk/200notional/5×/0.1funding/3sessionloss/5daily/5trade.
- Hermes/model ditunda sesuai pengguna; bukan syarat menahan patch inti.
  Tidak ada provider/dependency/toolchain baru. Sesi baru harus diaktifkan
  pengguna; belum terbukti profit atau operasi24jam.

## Riwayat: P2 kandidat 0.6.8 — jembatan lokal, belum dipasang pada putaran P2

- [Exporter/validator P2](docs/EVALUATION_BRIDGE_P2_08.md) sudah tersedia:
  snapshot final read-only, allowlist DTO, pseudonim, Decimal/hashes, manifest
  atomik, guard file/lock/ukuran dan validator proposal tanpa auto-apply.
- 166desktop PASS/2network ignored,36engine PASS; seluruh pipeline kedua crate
  lulus. Paket sesi acuan3trade/net−0.34155740USDT valid. Ekspor juga berhasil
  tanpa network dan environment; source tidak dimutasi oleh evaluator.
- Source/release0.6.8; **terpasang/proses tetap0.6.6**. P1 ikut kandidat, belum
  deployment. rustix1.1.4 yang sudah ada di lockfile menjadi direct dependency
  untuk openat2; tidak mengganti versi crate/toolchain atau risiko/strategi.
- Probe360s PASS92snapshot/0error,accountmaks4082ms/gap7313ms;1candleconfirmed
  tetapi0evaluasisegar. Setup pertama FAILrequestDNS publik, kedua PASS2snapshot.
  ATR0.0816% dan capTP menahan entry; bukan dua setup PASS.10JSONstate+binary
  terpasang identik baseline; tidak restart/order atau perubahan batas.
- Provider/model, plafon biaya dan izin kirim data Hermes belum dipilih.
  Belum install vendor/runner, isolasi Hermes, pilot model atau acceptance24jam.
  Selesaikan itu sebelum keputusan deploy idle; tidak perlu sesi live baru
  untuk menguji integrasi. Agen tidak mengaktifkan trading.

## Riwayat: P1 kandidat 0.6.7 — belum dipasang

- [Patch reliability P1](docs/RELIABILITY_P1_07.md) sudah diimplementasikan:
  history publik asinkron, lease DNS/waktu untuk pasangan GET order/akun,
  metadata di luar jendela bukti, guard quote awal, settlement Stop dan alasan/
  outcome exit terstruktur. Batas5detik, strategi dan batas uang tetap.
- 150tes desktop PASS/2network ignored,36tes engine PASS; fmt/Clippy/doc/release/
  audit489/87 PASS. Dua regresi FAIL→PASS; uji deadline24jam hanya waktu virtual.
  Probe GET360detik PASS:97snapshot/0error,1candle terkonfirmasi10118ms,
  maksaccount3359ms/gapsehat10955ms. Dua setup PASS; ATR0.0851% masih tertahan.
  Hash10file sesi/outbox/ledger dan binary terpasang tetap. Bukan uji order.
- Kandidat0.6.7 hanya source/build. **Aplikasi terpasang tetap0.6.6**, tidak
  restart/deploy. Tidak ada perubahan dependency/toolchain/config risiko,
  pemasangan Hermes/model atau pengiriman order. P2 exporter masih berikutnya.

## Riwayat: rancangan pembaruan + Hermes — 20 September 2026

- Pengguna meminta merancang pembaruan dengan Hermes. [Rancangan 07](docs/HERMES_INTEGRATION_PLAN_07.md)
  mengikat prioritas snapshot/exit-state, kontrak ekspor aman, evaluator
  terisolasi, proposal tervalidasi dan eksperimen terbatas. Ini dokumentasi,
  bukan rilis 0.6.7 atau integrasi yang sudah berjalan.
- Hermes dimasukkan ke desain sebagai evaluator setelah sesi; tidak memegang
  key/order/risk/live-state. Provider/model, izin data keluar/biaya, pin vendor
  dan runtime isolasi belum ditetapkan atau diuji. Tidak ada instalasi,
  panggilan model, patch source, penggantian binary atau aktivasi trading.
- Audit terbaru adalah [sesi300menit b1a0bcf9e6d31249d](../../outputs/binance-session-review-300m-20260920-ip2XD9/LAPORAN.md):
  3 trade, gross+0.14079999/fee0.48235739/funding0/net−0.34155740USDT;
  selesai194menit5,332detik karena `order_batch_age_ms=5069` melampaui5000.
  Aplikasi tidak crash. Dua GET pasca-audit16:44–16:45WIB lolos flat; bukan
  pemeriksaan akun baru dalam putaran rancangan. Belum diterima untuk24jam.
- Verifikasi desain: referensi source/bukti dan konsistensi arah diperiksa;
  98tautan lokal valid, whitespace PASS. Hash56file sourceRust/config/toolchain/
  lockfile sebelum-sesudah identik. Tes/build tidak diulang karena perubahan
  hanya dokumentasi. Putaran rancangan selesai; implementasi berikut mulai P1.

## Riwayat: input dan rencana eksekusi — 0.6.6

- **0.6.6 terpasang dan berjalan baca saja**, unit `binance-desk-debug-066.service`;
  hash release/binary/proses identik, seluruh14file state/outbox/ledger identik
  baseline. Tidak mengaktifkan sesi.137tes desktop lulus/2network ignored,36tes engine
  lulus, fmt/Clippy/doc/release/audit489 lulus. Tidak ada dependency/toolchain
  baru; hanya versi paket aplikasi berubah. Instalasi/probe akhir dicatat pada
  [laporan putaran](docs/INPUT_EXECUTION_PLAN_06.md).
- Probe600detik PASS:168snapshot,0error/rewarm/held,2candle terkonfirmasi dan
  dievaluasi,0sinyal. Quote193–686ms, accountmaks3916ms, gapsehat7598ms.
  Candle10:45/10:50 dikonfirmasi pada usia12486/7536ms. Bukan bukti24jam.
- Depth5 mengganti bookTicker untuk quote eksekusi; E/T exchange dan ID/level
  diperiksa, guard5detik tetap. Candle baru perlu dua pembacaan identik≥2detik;
  candle pending tidak menjadi baseline permanen. Recovery mengonsumsi interval
  tertutup saat data pulih agar konfirmasi tidak mengejar sinyal pemulihan.
- Preview menunjukkan rencana pada ATR aktual, bukan hanya contoh ATR. Jejak
  ekonomi/sizing dibatasi8percobaan, tanpa wallet/credential/raw private body.
  Tidak terbukti bug matematika planner; oracle15.000kombinasi sudah PASS pada
  kode lama. Batas ATR/SL/TP/RR, risiko, Stop dan ledger tetap.
- Dua setup GET10:41WIB PASS; ATR0.1757%, Long/Short masing-masing0.002BTC
  layak secara hitungan. Tidak berarti ada pola atau keuntungan. Sesi terakhir
  pengguna `b1a0baa280e4c6f3`600menit selesai normal,0transaksi; jangan campur
  dengan satu trade rugi sesi19September. Config tetap600/100/1/200/5,
  funding0.1, rugi3/sesi/5harian, maks5transaksi. Agen tidak aktivasi live/paper.

## Riwayat: freshness dan rekonsiliasi — 0.6.5

- **0.6.5 terpasang dan berjalan**, unit `binance-desk-debug-065.service`.
  Source/release/binary/proses cocok; akun baca saja, tanpa live/paper baru.
  Sesi pengguna terakhir `b1a0b9d2ccad94ae` tetap selesai, config180menit,
  satu transaksi; state/outbox/ledger identik backup.
- Rekonsiliasi order mendahului frame akun/quote, batch GET berbagi persiapan
  DNS/clock, bukti sekali pakai terikat intent dan≤5detik. Proteksi hilang,
  Stop/deadline dan data benar-benar usang tetap menghalangi kelanjutan.
  Respons market tanpa `cumQuote` dikonfirmasi satu GET, tidak resend POST.
- **129 passed, 1 ignored**, pipeline fmt/Clippy/doc/release/audit489 lulus.
  Regresi reproduksi lama gagal sebelum patch dan lolos sesudahnya. Dua
  snapshot setup GET berhasil; capture native diperiksa. Probe600detik:
  168snapshot sehat/2penolakan revisi candle/1rewarm, **exit1**, tanpa mutation.
  Dua pergantian candle, nol sinyal; bukan bukti stabilitas24jam.
- Audit sesi pengguna0.6.4 membuktikan satu entry/SL/TP/exit+ledger nyata,
  net−0.39563580USDT; berhenti147menit25detik karena stale, bukan aturan
  satu trade mengakhiri sesi. Tidak membuktikan profit atau stabilitas24jam.
- Strategi dan batas risiko tersimpan tetap; toolchain/dependency tidak berubah.

[Laporan0.6.5](docs/FRESHNESS_RECONCILIATION_05.md).

## Riwayat: konsistensi setup dan biaya — 0.6.4

- **0.6.4 terpasang dan berjalan**, tanpa aktivasi live oleh agen. State sesi
  lama tetap identik dan selesai; form tetap memuat batas lama, termasuk 60
  menit, sampai pengguna mengubahnya sendiri.
- Planner bersama preview/aktivasi/entry mempertahankan SL 2 ATR dan seluruh
  batas risiko. TP dasar 3 ATR boleh diperluas hingga 6 ATR untuk rasio bersih
  ≥1.1 termasuk fee/funding/slippage. Ini kandidat strategi baru, bukan bukti
  profit. Minimum ATR 0.10% dan timeout posisi 30 menit tetap.
- **119 tes passed, 1 ignored**, fmt/Clippy/doc/release/audit489 lulus.
  Deadline empat jam teruji sintetis. Satu snapshot setup GET layak bersyarat;
  snapshot kedua gagal harga usang. Tidak mengklaim koneksi bersih atau uji
  eksekusi nyata. Lihat laporan untuk hasil probe dan batas konfigurasi.
- Probe GET300 detik:79 snapshot sehat/3 penolakan harga usang; pembacaan
  pulih, exit1 tetap dicatat. ATR0.069% pada candle baru belum memenuhi syarat.
- Mesin tidak belajar otomatis; sesi panjang bisa tetap tanpa entry. Batas
  usulan lama dipertahankan: margin100/risk1/notional200/leverage5, rugi3/sesi
  dan5/hari UTC, funding0.1, maks5 transaksi, durasi180–240 menit.

[Laporan 0.6.4](docs/FEASIBILITY_LOGIC_04.md),
[riwayat pemulihan 0.6.3](docs/CANDLE_RECOVERY_03.md).

## Riwayat: pemulihan sesi macet — 0.6.1

- **Binance Desk 0.6.1 terpasang dan berjalan.** Sesi lama berhasil diselesaikan
  secara lokal setelah dua snapshot akun segar memastikan flat, tanpa order
  BTCUSDT dan tanpa perubahan wallet. Tidak memulai sesi live baru.
- Jalur rekonsiliasi tidak lagi bergantung pada history candle strategi.
  Validasi candle untuk entry tetap ketat; detail error kini disimpan dan
  footer membedakan rekonsiliasi dari sesi aktif. Form memulihkan batas sesi
  sebelumnya, tetapi persetujuan dana nyata tetap harus diberikan pengguna.
- **89 tes desktop lulus, 1 jaringan ignored**; fmt/Clippy/doc/release/audit489
  lulus. Probe GET-only binary terpasang PASS dua snapshot, maks2209ms.
- Backup binary/state lama disimpan; config, binding, daily dan angka accounting
  tetap identik. Tidak mengirim order atau mengubah akun pada putaran patch ini.
- Pemicu spesifik error lama tidak tersimpan. Mekanisme history yang menghambat
  recovery terbukti pada kode/tes; bukan bukti bahwa itulah respons Binance lama.

[Lingkup dan hasil](docs/SESSION_RECOVERY_FIX_01.md),
[bukti](../../outputs/binance-recovery-fix-20260917-lzZWCi/DECISION.md).

## Keputusan pengguna terbaru: Futures

Pengguna memilih **Binance Futures** karena ingin memakai leverage. Integrasi
akun dan mesin selanjutnya diarahkan ke Futures, dengan acuan USDⓈ-M/USDT
existing. Default aplikasi terpasang telah diperiksa: `--market futures`.
Arahan berikutnya: pengguna mengalokasikan dana Binance yang disediakan untuk
proyek ini dan menyerahkan pilihan strategi/rules. Pada 17 September pengguna
sudah mengisi batas sesi dan mengaktifkan sesi lama sendiri. Batas tersebut kini
dipulihkan dari state; alokasi atau keberhasilan koneksi bukan bukti profit.

## Riwayat: sesi Futures otomatis terhubung — 0.6.0

- **Binance Desk 0.6.0 terpasang.** Buka **Eksekusi Futures → Sesi otomatis**.
  Pengguna mengisi batas modal/risiko/leverage/durasi; tidak menentukan arah atau
  SL/TP. Satu aktivasi UI mengizinkan program mengelola order selanjutnya.
- Rust menghubungkan breakout20/volume, pullback EMA dan pantulan range ke
  Long/Short/menunggu, SL/TP ATR, margin/sizing, outbox, lifecycle dan supervisor.
  Satu posisi BTCUSDT; entry berlawanan menunggu flat+ledger posisi sebelumnya.
- Stop, deadline, batas rugi, rekonsiliasi, residual exit dan pemulihan restart
  tersedia. Ledger fill/fee/funding dicocokkan dengan wallet dua kali. Riwayat
  privat 0600 menyimpan alasan strategi, rencana dan hasil tanpa kredensial.
- **84 tes desktop +36 engine lulus**, 1 tes jaringan terpisah ignored.
  fmt/Clippy/doc/release/audit489 lulus. Probe binary terpasang: dua snapshot
  Binance nyata GET-only, usia maksimum1793ms termasuk pemeriksaan feeBurn.
  Source engine0.1 tetap cocok checksum baseline.
- **Tidak ada aktivasi dana nyata oleh agen.** Software kini memiliki jalur
  aktivasi live; izin TRADE, penerimaan POST/DELETE produksi dan profit belum
  dibuktikan. Tidak ada sesi paper baru. Batas versi awal tetap USDT standar,
  BTCUSDT, One-way, Single-asset, Isolated, komisi USDT dan akun didedikasikan.

[Lingkup/hasil](docs/AUTONOMOUS_SESSION_01.md),
[panduan](desktop-rs/README.md),
[bukti](../../outputs/binance-autonomous-session01-20260916/DECISION.md).

## Riwayat: label arah otomatis diperjelas — 0.5.1

Form yang semula "Rancang batas sesi" diperjelas menjadi **Kalkulator satu posisi
— contoh manual**, dengan **Arah contoh** dan budget/risiko per posisi. Pengguna
menegaskan mesin harus memilih Long/Short otomatis; strategi juga menentukan
SL/TP. Pilihan manual form tidak menjadi pengaturan arah tetap sesi live.
Ini patch penjelasan UI; integrasi strategi campuran/supervisor live masih terbuka.
[Lingkup dan hasil](docs/DIRECTION_UI_CLARIFICATION_01.md).

## Riwayat: margin dan konektor eksekusi Futures — 0.5.0

- **Binance Desk 0.5.0 terpasang**, halaman **Eksekusi Futures**. Kalkulator
  isolated memisahkan margin, notional dan risiko, memakai bracket/fee akun,
  filter, mark serta bid/ask. Aplikasi tetap native Rust.
- Typed production connector mencakup entry, reduce-only exit, Algo TP/SL,
  query/cancel, leverage dan isolated. Outbox durabel mencatat sebelum kirim,
  mencegah duplikat saat restart dan menolak laporan fill yang mundur.
- Pengendali keputusan otomatis diuji untuk fill parsial, kehilangan respons,
  penolakan SL, cancel/fill race dan cleanup. Desain tetap satu aktivasi sesi
  oleh pengguna, kemudian setiap order dikelola program secara otomatis.
- **67 tes desktop +36 engine lulus**; fmt, Clippy, doc, release, audit 489
  dependency lulus. Engine existing tetap cocok seluruh checksum sebelumnya.
  Probe bracket/fee/order/account aktual GET-only berhasil; tanpa nilai privat
  di laporan. POST/DELETE Binance tidak dijalankan pada putaran ini.
- **Belum siap live end-to-end:** supervisor posisi/proteksi, persistensi sesi
  yang terikat akun, ledger fill/fee/funding serta batas rugi nyata belum
  terintegrasi. UI belum menyediakan aktivasi live. Tidak ada sesi paper baru
  atau bukti profit baru.

[Lingkup](docs/FUTURES_EXECUTION_ROUND_01.md),
[kontrak komponen](desktop-rs/src/futures/README.md),
[bukti dan keputusan](../../outputs/binance-futures-execution-round01-20260916/DECISION.md).

## Riwayat: forward paper pada pasar nyata — 0.4.0

- **Binance Desk 0.4.0 terpasang**, unit sementara `binance-desk-paper-ui`
  menjalankan sesi 60 menit pada halaman **Mesin paper**. Ini simulasi otomatis
  BTCUSDT Futures dengan modal virtual 1000 USDT; tidak memakai dana akun.
- EMA20/50 + penembusan bar 5m, SL 2×ATR14 / TP 3×ATR14. Warmup tidak entry;
  candle baru sesudah sesi mulai menjadi pemicu. Sizing, fill, proteksi, exit,
  fee dan jurnal memakai engine Rust existing. Aturan belum terbukti profit.
- Risiko virtual/entry 2.5, notional maks. 250, batas rugi harian 10/total 30,
  sesi 60 menit, tanpa leverage simulasi. Angka ini bukan persetujuan risiko akun.
- **46 tes desktop + 36 tes engine lulus**, pipeline penuh dan audit 489 deps
  lulus. Probe publik mencapai tiga pembaruan lalu berhenti flat; jurnalnya
  diverifikasi lewat CLI engine. Source engine tetap cocok checksum terdahulu.
- Belum adapter order Binance, margin/leverage/likuidasi atau funding penuh.
  [Lingkup](docs/FORWARD_PAPER_ROUND_01.md),
  [bukti dan keputusan](../../outputs/binance-forward-paper-round01-20260916/DECISION.md).

## Riwayat: akun Futures terhubung pada patch 0.3.1

- Setelah laporan kegagalan parser dari pengguna, aturan ASCII-only diperbaiki.
  Metadata Futures resmi memuat lima simbol Unicode dari 897 simbol. Fixture
  publik mereproduksi bug; Unicode diterima tanpa mengubah identitas simbol,
  control/whitespace/duplikat dan data finansial invalid tetap ditolak.
- **0.3.1 terpasang; akun nyata berhasil dibaca dua kali berturut-turut pada
  worker UI.** KWallet kini memiliki entry pengguna. Tidak ada credential yang
  diambil dari memori/clipboard atau ditulis ke proyek/log/laporan.
- **34 tes lulus**, fmt/Clippy/doc/release/audit 489 dependency lulus. Diagnosis
  hanya label endpoint/check dan transisi koneksi, tanpa nilai respons akun.
- Tidak ada order, perubahan margin/leverage atau aktivasi strategi.
  [Perbaikan](docs/ACCOUNT_PARSER_FIX_01.md),
  [bukti](../../outputs/binance-account-parser-fix01-20260916/DECISION.md).

Bagian berikut mempertahankan riwayat sebelum patch parser.

## Akun Futures desktop 0.3.0 — 16 September 2026

- **Binance Desk 0.3.0 terpasang dan dibuka** pada halaman **Akun Futures**.
  Form HMAC tersamarkan, sesi saja atau KWallet, putus/hapus kredensial tersedia.
  Adapter Rust hanya signed GET ke account, positionRisk, accountConfig dan
  symbolConfig USD-M; tidak ada order atau mutasi akun.
- UI saldo/margin/P&L mengambang, posisi long/short, mode Hedge/One-way,
  leverage/margin per simbol tersedia. Desimal dipertahankan. Polling sekitar
  10 detik, stale 30 detik, retry/cooldown, pembatalan dan pembersihan data saat
  disconnect; akun belum dibaca tampil belum tersedia, bukan nol.
- **30 tes lulus**, ditambah satu tes transport publik nyata yang dijalankan
  terpisah. fmt/Clippy/doc/release/audit **489 dependency** lulus. KWallet dummy
  tulis/baca/hapus terverifikasi; source engine tetap cocok checksum lama.
- **Penerimaan akun live TERHAMBAT:** `KWALLET_CREDENTIAL_MISSING` dan
  `ACCOUNT_CREDENTIAL_MISSING`. Belum ada signed GET akun nyata yang berhasil
  diverifikasi. Masukkan kredensial langsung di aplikasi untuk melanjutkan;
  jangan kirim key/secret lewat chat. Data contoh dalam tes bukan data akun.
- Tampilan native diperiksa; unit sementara `binance-desk-account-ui` aktif,
  tanpa autostart. Jendela lama tidak ditutup otomatis. Integrasi read-only
  ini belum strategi, paper forward test atau eksekusi Futures/margin engine.
- [Lingkup](docs/ACCOUNT_DESKTOP_ROUND_01.md),
  [bukti/keputusan](../../outputs/binance-account-desktop-round01-20260916/DECISION.md).

Bagian berikut mempertahankan riwayat sebelum integrasi akun desktop.

## Pasar Binance di desktop 0.2.0 — 16 September 2026

- **Binance Desk 0.2.0 terpasang** user-local. Ruang kerja menampilkan BTC/ETH,
  pilihan Spot / USD-M Futures, candle/volume 1m/5m/15m/1h, harga watchlist,
  hover OHLCV, zoom/pan/reset dan jeda/lanjut feed. Akun tetap belum terhubung.
- Worker Rust GET publik memperbarui 240 candle per simbol sekitar dua detik.
  Ada status/umur data, stale setelah delapan detik, retry/backoff, Retry-After
  429/418, penolakan candle invalid/gap dan pembatalan konteks yang sudah diganti.
- Koneksi DNS sistem gagal pada Fedora ini; fallback DNS publik terverifikasi
  TLS memulihkan pembacaan. Tidak mengubah DNS OS atau mengakses kredensial.
- **16 tes lulus**; fmt/Clippy/doc/release/audit 422 dependency lulus. Data nyata
  kedua simbol dan semua delapan kombinasi pasar/timeframe berhasil dibaca;
  pause/resume lulus. Native framebuffer Futures 1400×900 dan Spot 1024×720
  diperiksa. Engine lama tetap identik menurut checksum, tanpa perubahan.
- Pemantauan feed nyata 70 detik setelah resume: 34 batch tambahan, tanpa putus
  teramati. Versi baru aktif pada unit sementara `binance-desk-market-ui`.
- Feed ini untuk observasi dengan REST polling, belum WebSocket/order book atau
  input strategi eksekusi. Tidak ada transaksi, akses akun, HFT, backtest atau
  bukti profit baru. Selector pasar di UI hanya mengubah data yang ditampilkan;
  arah produk sudah ditetapkan pengguna ke Futures setelah tahap ini selesai.
- [Kontrak tahap](docs/MARKET_DESKTOP_ROUND_01.md),
  [bukti/keputusan](../../outputs/binance-market-desktop-round01-20260916/DECISION.md).

Bagian di bawah adalah riwayat tahap sebelumnya.

## UI utama: Binance Desk native — 16 September 2026

- Sesuai arahan terbaru, UI utama berbentuk **aplikasi desktop Linux native**,
  bukan dashboard web lokal. [desktop-rs](desktop-rs/README.md), Rust + egui/eframe,
  memakai library engine existing; tidak mengubah kode inti/Python/web lama.
- Terpasang di `~/.local/bin/binance-desk`, ikon dan launcher **Binance Desk**
  pada menu aplikasi. Jendela native berhasil dibuka pada Fedora KDE/Wayland.
- Ruang kerja/watchlist belum terhubung; halaman demo interaktif chart/posisi/order/
  risiko, jurnal dengan pencarian/ekspor, pengaturan skala/grid/kecepatan tersimpan.
  Harga/akun live tetap kosong. Semua angka demo jelas berlabel sintetis.
- **6 tes desktop lulus**, mencakup klik pointer pada widget egui, replay/export,
  pengaturan dan layout minimum; fmt/Clippy/doc/release/audit 339 dependency lulus.
  Doctor lingkungan 52 pemeriksaan lulus. Framebuffer native empat halaman dan
  ukuran 1024×720 diperiksa, label/tinggi chart diperbaiki agar kontrol terlihat.
- [Tampilan demo](../../outputs/binance-desktop-round01-20260916/simulation-final.png),
  [ruang kerja](../../outputs/binance-desktop-round01-20260916/workspace.png),
  [bukti/keputusan](../../outputs/binance-desktop-round01-20260916/DECISION.md).
- Keputusan **LANJUT untuk UI desktop**. Belum feed/account/strategi/live order.
  Tidak ada riset performa, order, perubahan akun atau setup startup permanen.
  Jendela sesi ini dijalankan dengan unit user sementara `binance-desk-ui`;
  tutup jendela untuk menghentikannya. Menu/command dapat membuka aplikasi kembali.

## Inti eksekusi Rust — 16 September 2026

- Komponen baru [engine-rs](engine-rs/README.md): library + CLI lokal untuk sizing
  desimal, satu posisi long/short, partial fill, TP/SL, permintaan cancel/exit,
  timeout/reconciliation, batas loss harian/total dan jurnal persisten.
- **Paper sintetis saja.** Belum tersambung ke feed/strategi/account Binance;
  tidak ada transport jaringan, signing atau order nyata. Referensi teknis
  USDⓈ-M one-way; pasar final, modal bot, risiko dan target pengguna belum ditetapkan.
- **36 tes lulus**, format/Clippy/doc/release/audit Cargo lulus. Doctor lingkungan
  lulus. Demo release menyelesaikan satu siklus; restart proses terpisah menghasilkan
  state/hash sama. Duplicate event tidak menambah record atau effect.
- Pipeline awal menemukan lock tertahan oleh descriptor proses anak; diperbaiki
  dengan unlock eksplisit dan satu tes regresi. Log gagal dipertahankan.
- Keputusan putaran **LANJUT untuk fondasi perangkat lunak**, bukan kelayakan live
  atau bukti profit. Tidak ada backtest/strategi baru; hasil negatif lama tetap berlaku.
- [Bukti demo](../../outputs/binance-execution-round01-20260916/demo/report.md),
  [keputusan dan batas](docs/EXECUTION_ROUND_01.md),
  [pipeline](../../outputs/binance-execution-round01-20260916/check-final.log).
  Python/UI/layanan existing tidak diubah atau diuji ulang pada putaran ini.

Bagian berikut menyimpan pemeriksaan terdahulu; keterangan layanan Windows lama
bukan verifikasi runtime Fedora pada 16 September.

## Arah produk — 15 September 2026

- Pengguna menegaskan Binance sebagai platform sasaran: mesin mandiri untuk
  keputusan trading, order, TP/SL, posisi dan risiko secara otonom. Strategi
  sniper/scalping/lainnya atau gabungan diperbolehkan berdasarkan bukti.
- [Arah dan pemetaan kemampuan](docs/BINANCE_AUTONOMOUS_SCOPE.md) serta AGENTS
  diperbarui. Komponen baru mengutamakan Rust; Hermes masih opsi pendukung.
- Pemeriksaan kali ini membaca kode/dokumen: klien privat tetap GET-only,
  order router dan manajemen risiko akun belum tersedia. Tidak ada perubahan
  kode/runtime, pemasangan, pengujian strategi baru, atau transaksi.
- Pilihan pasar, modal khusus bot, batas rugi dan target numerik masih perlu
  ditegaskan. API yang sudah disiapkan pengguna belum diuji ulang sesi ini.
- Bagian berikut adalah hasil pemeriksaan terdahulu, bukan verifikasi runtime
  Fedora atau bukti performa baru pada 15 September.

## Aktif sekarang: observasi kanal H4 (bukan order nyata)

- Halaman **http://127.0.0.1:8765/observe** dan `GET /api/observer` aktif. Tautan OBSERVASI LIVE ditambahkan pada homepage; bukti MT5 dan model range lama tetap tersedia.
- Engine H4 v2 yang sama, BTC/ETH × 20/10 dan 55/20, tanpa seleksi/optimasi/strategi baru. Input warmup 220 bar/simbol tidak menghasilkan trade historis. Session dimulai 14 September 2026, 17:27:07 WIB; posisi model dicatat saat candle H4 selesai, bukan fill live intrabar.
- DNS lokal mengarah ke alamat yang timeout. Fallback DNS-over-HTTPS khusus GET publik aplikasi memulihkan 24/24 REST feeds, lag 0, TLS/hostname Binance resmi tetap diperiksa. DNS Windows, private reader, WebSocket, whitelist dan kredensial tidak diubah. WebSocket masih RECONNECTING; panel akun AUTH_UNAVAILABLE, bukan klaim posisi/order akun nol.
- Persistensi `state/channel-observer.sqlite3`, identitas kode, stale/gap/revision guards, retry-after rate backoff. Restart aktual mempertahankan sesi dan 220 input/simbol tanpa duplikasi; browser menampilkan putus lalu pulih otomatis.
- 283 Python / 23 JavaScript tests passed; pip check bersih, satu warning dependency lama. UI desktop/mobile, level empat model vs API, harga bergerak, dan disconnect/recovery diperiksa.
- Keputusan aktivasi: **OBSERVASI AKTIF**. Putaran 01 tetap **TOLAK** dan tujuan profit belum tercapai. Funding live belum lengkap, net lengkap null; tidak ada order router/mutasi akun.
- [Kontrak, hasil, dan batas model](docs/LIVE_OBSERVER_ACTIVATION.md). Bagian berikut adalah riwayat audit sebelumnya, bukan status layanan saat ini.

## Latest audit response: A1/A2 dijawab, funding A3 diperbaiki

- **BELUM ADA DASAR CUKUP** untuk strategi berikutnya; tiga sumber primer dan dua alternatif ditinjau. Putaran 01 tetap TOLAK, seluruh gate/hasil v1 dipertahankan.
- Source offline `round01` kini funding `ownership-interval-v2`; tidak membebankan/mengkredit sesudah exit pasti, konservatif pada urutan intrabar ambigu. CLI study berprotokol v1 menolak source v2 sebelum IO. Belum ada protokol atau hasil historis v2.
- Verifikasi saat tindak lanjut: 137 unit terkait dan 267 regresi Python lulus; 112 tes sintetis baru, satu putaran koreksi. Satu warning deprecation dependency. UI/JS dan layanan tidak diubah atau diverifikasi ulang di tindak lanjut ini.
- [AUDIT_RESPONSE.md](../../outputs/binance-sniper-audit-response01-20260914-150514/AUDIT_RESPONSE.md), [SOURCE_COMPARISON.md](../../outputs/binance-sniper-audit-response01-20260914-150514/SOURCE_COMPARISON.md), [DECISION.md](../../outputs/binance-sniper-audit-response01-20260914-150514/DECISION.md). Tidak ada eksperimen historis baru atau mutasi akun; tidak lanjut otomatis. Bagian berikut merekam v1.

## Latest research: Putaran mandiri 01 — TOLAK

- Implementasi offline `round01`: breakout kanal H4 20/10 dan 55/20, shared engine/ledger/chart projection; 2 config × 2 bagian × 2 biaya = 8 trial. Reproduksi delapan file byte-identik, bukan sampel baru.
- SELECTION 177 hari BTC+ETH: 20/10 72 trade / PF 1.00976 / net indeks +0.00634892, stress -0.03305116 / DD indeks 38.17%; 55/20 35 trade / PF 1.10187 / net +0.03723795, stress +0.01820619 / DD 33.83%. Kedua config gagal protokol; final **NOT_RUN_NO_ELIGIBLE_CANDIDATE**. Hasil adalah indeks riset, bukan USD/return akun; data sudah pernah dilihat.
- Verifikasi aktual putaran ini: 155 Python dan 19 JS passed, pip check bersih; 12 prefix data nyata, ledger/funding/MTM reconciliation dan viewer parity PASS. Awal 24/25 unit passed; fixture keliru diperbaiki, bukti gagal tetap disimpan.
- Viewer terminal standalone terverifikasi render + 20 frame berurutan; belum terintegrasi ke route lama/observer. `/`, `/tester`, `/market` tetap 200 pada proses existing PID 32704, tidak direstart. Tidak ada transaksi/mutasi akun atau perubahan layanan permanen.
- Batas bukti: official exchangeInfo timeout tiga kali; tidak memodelkan executable lot/tick/min-notional, margin/likuidasi atau fee akun. Tidak ada klaim mesin profit. Putaran ditutup; tidak lanjut otomatis.
- [Paket audit dan reproduksi](../../outputs/binance-sniper-round01-20260914-135512/AUDIT_PACKET.md). Bagian berikut mempertahankan UI existing dan riwayat sebelum putaran ini.

## Active UI: MT5 Research Desk / evidence pipeline

- Homepage `/` and `/tester`: candidate evidence, report metrics, actual balance ledger/chart, provenance and gated repeat comparison. Existing Binance model moved to `/market`; observer/results unchanged.
- Offline bounded HTML parser, numerical reconciliation, input/comment-free API, append-only/idempotent SQLite and GET-only routes. No automatic tester runner or trading control.
- Store: 1 Jimmy report, 1 explicitly old SniperFX parser-check report. Manual login succeeded; official v5.20 installed and visual test completed on FBS, 2026-09-01 to 2026-09-13. 22 trades / 44 deals; net +671.44 USD; reported PF 1.98 and equity DD 3.70%. Ledger reconciled and dashboard refreshed/verified. No live trading or purchase. This is compatibility evidence, not validated profitability; quality 83%, four markets only, randomization and costs require follow-up. [Run 01](docs/DEMO_RUN_01.md).
- Verification: 130 Python / 19 JavaScript tests; eight real legacy report checks reconciled, one unit-harness format rejected; desktop/mobile and offline UI verified. [Full protocol](docs/TESTER_PIPELINE.md).

## Retained observer: Range Breakout Reference

- Main engine and homepage replaced with a single source-described session breakout model. Old scanner and research/strategy-lab tasks are no longer running.
- Range 03:00–06:00 UTC, first breakout, opposite-boundary stop, exit by 18:00 UTC. No fixed TP, no trailing, no four-bar forced close. Rules are an explicit Binance/timezone/M5 adaptation, not vendor performance replication.
- New restart-persisted `state/reference.sqlite3`; immutable completed sessions and revision warning; reconstructed observations are never reported as orders.
- New terminal matches the engine: range/entry/stop overlays, holding reason/duration, source/evidence/archive panels, actual historical session charts, separate synthetic demo.
- Exploratory replay: 2,700 sessions over the two already-seen datasets; 2,685 closed, 14 no breakout, one ambiguous. Mean net -0.075788R; stress -0.185125R; PF 0.874950. Profitability **NOT ESTABLISHED**. No tuning or live activation.
- Current verification and runtime evidence are in TEST_RESULTS.md. The section below is historical Phase 1–3 status only.

## Archived Phase 1–3

- [x] GET-only private client with credential redaction and exact official-host pinning
- [x] Six-symbol, four-timeframe production REST warmup and per-feed fallback scheduler
- [x] WebSocket silent watchdog, counted reconnects, reconciliation gate, and planned renewal
- [x] Shared REST/WS candle validation, preview separation, freshness, lag, and rate telemetry
- [x] Stable causal impulse/zone identity and monotonic persisted lifecycle
- [x] Deterministic candidate scoring with component breakdown
- [x] Persisted four-bar directional MFE/MAE/final-R and intrabar ambiguity tracking
- [x] Semantic replay with anti-lookahead and restart/idempotency tests
- [x] Supervised task health registry and server-time refresh
- [x] Exact PID/port/project ownership scripts and stale-lock recovery
- [x] Dark dashboard with live/replay modes, axes, time/price overlays, freshness, account age, task, and rate status
- [x] MT5/Bloomberg-inspired local terminal with Market Watch, chart crosshair/zoom/pan, decision gates, and terminal tabs
- [x] Browser disconnect/freshness protection, matching candidate/zone overlays, REST preview finality, and dataset-specific symbol selection
- [x] Synthetic step/play replay through six causal frames with isolated account display
- [x] Phase 2 immutable decision/signal journal, public-data historical event study, cost model, and terminal evidence panel
- [x] 90-day causal baseline with a fixed 60/30 chronological split; 27 signals, negative modeled net means in both partitions
- [x] Phase 3 preregistered eight-variant strategy lab with causal shared historical/live pattern engine
- [x] New 360-day dataset, three training-selected walk-forward folds, final 60-day holdout, cost stress, and clustered uncertainty
- [x] Historical area evidence, explicit 4/8-bar horizons, and separate restart-persisted experimental live observation
- [ ] Demonstrated strategy edge or future/live profitability: all eight alternatives remain negative; NO_PROMOTION
- [ ] Trading execution (intentionally absent)

Latest verification: Python `80 passed`, JavaScript `10 passed`, one third-party Starlette/AnyIO deprecation warning, clean `pip check`. Latest service owner: PID 25112, 29/29 tasks RUNNING, 24/24 fallback feeds, no positions/orders. Phase 3 result: `lab-e98337744f4aefe8ba8625fc`; selected candidate `SWEEP_T1_4`, 76 holdout events with -0.227142R mean net. The implementation is more testable and observable; a profitable trading engine has NOT been established. See `TEST_RESULTS.md` and `docs/PHASE3_RESULTS.md`.
