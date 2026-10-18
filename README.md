```text
╭────────────────────────────────────────────────────────────────╮
│   b i n a n c e - s n i p e r                                  │
│   [TRADING] Binance Desk — desktop Rust + mesin riset          │
╰────────────────────────────────────────────────────────────────╯
```

> Aplikasi desktop **Linux native (Rust)** untuk pasar Binance, dengan
> alur lengkap: feed candle/volume → pembacaan akun Futures → sesi
> eksekusi otomatis yang **diaktifkan pengguna** → paper engine dan
> reference engine untuk riset. Semua klaim hasil ditulis apa adanya,
> termasuk yang negatif.

**Ringkas**

- **Binance Desk** — UI desktop (`desktop-rs`, lihat `binance-desk --page futures`): candle/volume BTC/ETH Spot/Futures (1m–1h), status umur data, panel akun.
- **Akun Futures (read-only)** — API key HMAC dimasukkan lewat aplikasi (opsi simpan di KWallet); tidak ada key di repo.
- **Sesi Futures otomatis** — setelah diaktifkan pengguna: pemilihan Long/Short, SL/TP, sizing, pengiriman order, pengawasan posisi, ledger fill/fee/funding, batas rugi; tombol **Hentikan & tutup posisi** untuk berhenti.
- **engine-rs** — fondasi paper: sizing, lifecycle order/posisi, TP/SL, batas risiko, journal restart (36 tes lulus).
- **Reference engine** — session range breakout (03:00–06:00 UTC) sebagai pembanding riset; replay 450 hari tetap **negatif** dan dipertahankan sebagai riwayat.
- **Admin**: dokumen status ada di `STATUS.md` / `HANDOFF.md`; laporan tiap putaran di `docs/`.

## Aplikasi utama: Binance Desk untuk Linux

UI native Rust sudah terpasang. Cari **Binance Desk** di menu aplikasi KDE, atau:

```bash
binance-desk
```

Versi terpasang **0.6.11** memperbaiki sesi berhenti saat flat akibat data
usang: jeda entry dan lewatkan candle pemulihan. Clock snapshot disiapkan
sebelum data, tanpa GET waktu tambahan sesudah quote. Saat ada intent tetap
close/accounting, maksimal2episode dan60detik sehat, hanya candle baru.
Guard5detik, Stop, risiko dan strategi tetap.204tes desktop+36engine serta
probe GET600s/215snapshot tanpa kegagalan snapshot lulus; fallback koneksi
masih terpakai. Dipasang idle, bukan aktivasi sesi. UI/ekspor kompatibel
arsip lama, Hermes/model tidak aktif. Belum bukti24jam/recoveryreal0.6.11/profit.
[Laporan patch stale/clock](docs/STALE_CLOCK_11.md).

**Ruang kerja** kini menampilkan candle/volume Binance BTC/ETH, Spot/Futures,
timeframe 1m/5m/15m/1h, zoom/pan, serta status/umur data. Polling pasar publik
sekitar dua detik. Versi **0.6.0** menyediakan **Akun Futures**: masukkan API
key/secret HMAC di aplikasi, lalu hubungkan untuk membaca saldo, posisi, margin
dan leverage. Opsi simpan di KWallet tersedia. Pembacaan akun nyata sudah
berhasil diverifikasi setelah perbaikan parser simbol Unicode pada 0.3.1.
**Eksekusi Futures → Sesi otomatis** menghubungkan tiga aturan strategi ke
pemilihan Long/Short/menunggu, SL/TP, sizing, pengiriman order, pengawasan posisi,
pembukuan fill/fee/funding dan batas rugi. Buka dengan `binance-desk --page futures`.
Isi batas modal/risiko, leverage, durasi dan jumlah posisi, lalu aktifkan sesi
melalui tombol native. Setelah satu aktivasi, order dikelola program; pengguna
tidak memilih arah atau SL/TP setiap entry. Kalkulator manual tetap opsional.
**Hentikan & tutup posisi** mengakhiri sesi setelah posisi dan biaya terkonfirmasi.
Startup hanya membaca; setelah crash gunakan **Pulihkan & tutup sesi lama**.
Pengujian patch memakai exchange tiruan dan Binance GET-only. Sesi yang
diaktifkan pengguna19September membuktikan satu entry, SL/TP, exit reduce-only
dan ledger produksi, dengan hasil bersih negatif; bukan bukti profit/operasi24jam
atau acceptance semua cabang order. Tidak ada sesi uang nyata yang diaktifkan
oleh agen selama implementasi.
Fitur **Mesin paper** lama tetap tersedia; tidak ada sesi paper baru pada putaran
ini. Strategi baru maupun lama belum terbukti profit.
**Demo sintetis** tetap menyediakan fixture engine untuk melihat lifecycle secara
langsung. Aplikasi berjalan dalam jendela desktop sendiri.

[Panduan aplikasi](desktop-rs/README.md) ·
[Screenshot pasar native](../../outputs/binance-market-desktop-round01-20260916/workspace-installed.png) ·
[Bukti pasar](../../outputs/binance-market-desktop-round01-20260916/DECISION.md) ·
[Bukti tahap akun](../../outputs/binance-account-desktop-round01-20260916/DECISION.md)

[Perbaikan parser dan koneksi akun nyata](../../outputs/binance-account-parser-fix01-20260916/DECISION.md).
[Mesin forward paper dan bukti](../../outputs/binance-forward-paper-round01-20260916/DECISION.md).
[Margin/konektor Futures 0.5.0 dan batas integrasi](../../outputs/binance-futures-execution-round01-20260916/DECISION.md).
[Integrasi sesi otomatis 0.6.0](docs/AUTONOMOUS_SESSION_01.md).

Arahan pengguna memprioritaskan desktop native; dokumentasi web di bawah tetap
disimpan sebagai riwayat. Tidak ada server HTTP baru yang dibuat.

## Komponen terbaru — sesi Futures otomatis

[`desktop-rs/src/futures`](desktop-rs/src/futures/README.md) berisi model margin,
protokol order/Algo TP-SL/leverage/isolated, transport signed, outbox durabel
dan supervisor sesi, pemulihan, strategi tiga aturan, ledger serta aktivasi UI.
Pengujian order memakai exchange
lokal dalam memori; probe Binance hanya GET. Belum bukti acceptance order nyata.
[Lingkup dan syarat selesai](docs/AUTONOMOUS_SESSION_01.md).

## Fondasi simulasi Rust, 16 September 2026

[**engine-rs**](engine-rs/README.md) adalah fondasi paper untuk sizing, lifecycle
order/posisi, TP/SL, batas risiko dan restart journal. 36 tes serta pipeline Rust
lulus. [Demo yang sudah dijalankan](../../outputs/binance-execution-round01-20260916/demo/report.md)
dan [keputusan putaran](docs/EXECUTION_ROUND_01.md).

Adapter paper desktop kini menyambungkan strategi ke harga publik; belum ada
order exchange atau penggunaan dana akun. Harga dan fill demo CLI tetap
sintetis; hasil riset negatif lama tetap berlaku. Panduan jalan, kontrak adapter,
dan keterbatasan ada di README komponen. [Arah produk](docs/BINANCE_AUTONOMOUS_SCOPE.md).

## Riwayat Research Desk — MT5 Evidence & Binance Reference

Bagian berikut mempertahankan dokumentasi sebelumnya, termasuk runtime Windows;
bukan pemeriksaan ulang layanan atau profit pada Fedora hari ini.

Latest demo result (2026-09-14): manual login completed; official Jimmy EA v5.20 installed and **one visual compatibility test completed**. No live Algo Trading or purchase. [Actual test and limitations](docs/DEMO_RUN_01.md).

Open **http://127.0.0.1:8765/** or `/tester` for the original-EA evidence workspace: offline reports, numerical reconciliation, actual balance ledger/chart and repeat comparison. **1 candidate report; 1 labeled old SniperFX parser check.** Candidate sample: 22 trades, net +671.44 USD, reported equity DD 3.70%; 83% reported history quality and incomplete provenance. This short positive backtest does not establish live profitability.

The Binance model/account observer remains at **http://127.0.0.1:8765/market** with unchanged negative exploratory results. It is not the Jimmy EA. No trading mutation capability is added. [Pipeline, commands and limitations](docs/TESTER_PIPELINE.md).

Tests: `.\.venv\Scripts\python.exe -m pytest -q` and `node --test tests/terminal-model.test.cjs tests/strategy-panels.test.cjs tests/range-model.test.cjs tests/tester-model.test.cjs`. CLI: `.\.venv\Scripts\python.exe -m binance_sniper.tester --help`.

## Retained Range Breakout Reference (now `/market`)

The active engine is now a single source-described **session range breakout**, not the old eight-pattern lab. Build the range at 03:00–06:00 UTC, model its first breakout, stop at the opposite boundary, and hold until stop or 18:00 UTC. No fixed TP, trailing, or four/eight-bar exit. The old strategy loops are retired; old code and results are preserved.

This is an independent **Binance adaptation** of public rules, not the vendor's EA or a proven-profit engine. It has no order, cancel, leverage, margin, or account-mutation capability. The already-seen 450-day replay remains negative: **2,685 resolved modeled positions, mean net -0.075788R, stress -0.185125R**, one unresolved first-direction ambiguity. No parameter training or optimization was run.

Open **http://127.0.0.1:8765** and refresh the tab. The terminal shows the session range, entry/stop, held position and its reason, source rules, historical evidence, and a separate legacy archive. **Bukti & hasil → Lihat chart sesi** opens actual stored M5 data with explicit hindsight labeling. **Demo visual** exercises the new engine using synthetic prices; it is not profit evidence. Market/account and recorded/synthetic modes are separated.

[Frozen method and adoption limitations](docs/REFERENCE_ENGINE.md). Reproduce: `.\.venv\Scripts\python.exe -m binance_sniper.reference.study`. Test: `.\.venv\Scripts\python.exe -m pytest -q` and `node --test tests/terminal-model.test.cjs tests/strategy-panels.test.cjs tests/range-model.test.cjs`. No new dependency.

## Preserved documentation for earlier versions

The sections below describe the retired Phase 1–3 interfaces and research for reproducibility, not the current homepage or active strategy. The setup/run/stop commands remain applicable. Use the reference command above for the current model.

## Setup and run

```powershell
.\scripts\setup.ps1
.\scripts\run.ps1
```

Open `http://127.0.0.1:8765`. Refresh an older browser tab after upgrades. The amber terminal includes Market Watch, a zoomable/pannable chart with crosshair, H4/H1/M15/M5 context, scanner explanations, and Journal / Setup history / Connection / Account tabs. Choose a timeframe in the toolbar or context strip; choose an archived candidate from Setup history and use Reset view to return to current context.

Use `http://127.0.0.1:8765/?mode=replay` for an explicitly synthetic six-frame example. Play, step, or scrub from the armed zone through the trigger and four outcome candles. Replay uses its own symbol list and does not display live account data. This example is not market-performance evidence.

Inspect the service with `.\scripts\status.ps1` and stop it with `.\scripts\stop.ps1`.

The account panel loads `%LOCALAPPDATA%\CodexBinance\credentials.dpapi` only in memory. Credentials are not copied into this repository. `ipRestrict=false` is an accepted operator policy and appears as a warning, not a blocker.

## What Phase 1D proves

- Validated closed bars drive every analysis decision; the current candle is visual preview only.
- Per-feed REST fallback remains live independently of a silent WebSocket.
- Zone, candidate, transition ledger, and four-bar outcome are deterministic and restart-safe.
- Replay exercises the production cache and scanner through regime, impulse, pullback, zone, trigger, candidate, and four-bar completion.
- Account access is GET-only and separates regular, conditional, and execution-algo orders.

## Test

```powershell
.\.venv\Scripts\python.exe -m pytest -q
.\.venv\Scripts\python.exe -m pip check
node --test tests/terminal-model.test.cjs tests/strategy-panels.test.cjs
```

This phase validates correctness and observability. It does not prove profitability and intentionally contains no trading execution.

## Phase 2: measured baseline

Open the **Research · Phase 2** terminal tab. Historical results and the live observation journal are separate. **Lihat candle** opens the stored M5 evidence for an event; its four post-signal candles are explicitly hindsight, not decision-time inputs.

The first frozen 90-day study (2026-06-15 through 2026-09-12 UTC) processed 228,840 public candles including warmup, 155,520 M5 decisions, and 27 signals. Modeled net mean was **-0.396R** in development (16 samples) and **-0.403R** in holdout (11 samples). This baseline has not demonstrated an edge. R is event risk-distance, not account return or a fixed percentage of capital.

Reproduce with the existing project environment; no new dependency is required:

```powershell
.\.venv\Scripts\python.exe -m binance_sniper.research.download --days 90
.\.venv\Scripts\python.exe -m binance_sniper.research.study
```

Download uses public market-data GET requests. Study replays local SQLite data, without network access. The runtime separately journals live observations to `state/research.sqlite3`. Research GET endpoints only read stored reports/events; opening the panel never starts a study or an order.

See [Phase 2 methodology and findings](docs/PHASE2_RESEARCH.md), `STATUS.md`, and `TEST_RESULTS.md`. Historical JSON export: `%USERPROFILE%\Desktop\Workspace\outputs\binance-sniper-phase2-90d.json`.

## Phase 3: tested alternatives, no promotion

Open **[Strategy Lab · 3](http://127.0.0.1:8765/?tab=lab)** (refresh an older tab). **HASIL UJI** compares the preserved baseline with eight registered alternatives; **OBSERVASI BARU** shows their separate experimental states and real recorded areas. The main terminal chart still shows the original baseline. Evidence buttons open stored candles with explicit post-decision hindsight shading; no invented area or trade is displayed.

New rules test retracement geometry, breakout/retest, sweep/reclaim, H1 direction with H4 opposition veto, directional expansion/trigger, volatility-based stop distance, structural target room, costs, and four/eight-bar horizons. No failed hypothesis replaces the baseline or gains execution capability.

The preceding **360 days / six pairs / 899,520 candles** were replayed under a [protocol frozen before outcomes](docs/PHASE3_PROTOCOL.md). All eight alternatives were negative after modeled costs. Training selected `SWEEP_T1_4`: **76 final-holdout events, -0.227R mean net** versus baseline **25 / -0.564R** on the same period. All three walk-forward folds were also negative. Lower loss per event is not profitability or better account return. Result: **NO_PROMOTION**.

Reproduce the fixed dataset and retained trial:

```powershell
.\.venv\Scripts\python.exe -m binance_sniper.research.download --days 360 --end 2026-06-15
.\.venv\Scripts\python.exe -m binance_sniper.strategy_lab.runner --export ..\..\outputs\binance-sniper-phase3-360d.json
```

The second command uses local data only and returns an existing completed trial when fingerprints match. Findings, assumptions, and the next research boundary are in [PHASE3_RESULTS.md](docs/PHASE3_RESULTS.md). The terminal remains local and read-only, with no additional dependency or hosted data migration.

## Disclaimer

Proyek ini **riset pribadi dan rekayasa perangkat lunak** — bukan nasihat
finansial, bukan ajakan trading, dan bukan jaminan profit.

- Kemampuan eksekusi Futures hanya aktif bila **pengguna mengaktifkan sesi** secara sadar; agen pengembang tidak pernah mengaktifkan sesi uang nyata.
- API key tidak disimpan di repo; key dimasukkan lewat aplikasi (opsi simpan di KWallet).
- Semua hasil backtest/paper yang dilaporkan di repo ini **negatif** (mis. `NO_PROMOTION`, replay 450 hari) dan dipertahankan apa adanya sebagai riwayat riset — bukan bukti performa.
- Pemakaian perangkat lunak ini sepenuhnya risiko pengguna; lisensi MIT berarti tanpa garansi apa pun.

## Lisensi

MIT — lihat [LICENSE](LICENSE).
