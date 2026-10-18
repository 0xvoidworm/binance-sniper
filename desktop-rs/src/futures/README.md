# Sesi Futures otomatis — 0.6.10

## Pemulihan transport 0.6.10

Sesi baru menyimpan episode recovery opsional: maksimum dua gangguan dengan
intent, close-only sampai flat dan accounting cocok dua kali, lalu 60 detik
health monotonic/server dengan gap ≤10 detik. Candle pemulihan dikonsumsi;
tidak ada reentry pada candle lama. Proof tidak survive restart. Hanya tipe
Network yang diizinkan; Stop terminal, auth/rate/schema/protection uncertainty
tetap mengakhiri sesi. Revalidasi history saat flat tetap pause tanpa entry.
Lihat [kontrak patch](../../../docs/TRANSPORT_RECOVERY_10.md); tidak mengganti
freshness5detik, batas uang, planner atau aturan POST sekali.

## Komponen P1/P2 yang dipertahankan

P2 menambahkan modul `evaluation` dan CLI offline untuk exporter/validator serta
proposal review-only. Tidak dipanggil worker live; snapshot hanya final latest
session, shared lock read-only, tanpa vault/transport. Paket allowlist/Decimal/
hash atomik menyatakan data hilang secara eksplisit; output model tidak pernah
diterapkan ke config/order. Runtime/model Hermes belum ada.
[Kontrak P2](../../../docs/EVALUATION_PACKET_V1.md),
[hasil/keterbatasan](../../../docs/EVALUATION_BRIDGE_P2_08.md).

P1/P2 ikut rilis gabungan0.6.9; status runtime terbaru di STATUS. History memakai worker
publik tanpa key, antrean satu request/result dan timeout10detik. Confirmed
history disalin ke frame; history pending tidak menunggu di loop proteksi.
Failure menahan entry; sesi baru memakai kebijakan0.6.10 di atas. Stop membuang worker.
Frame produksi: reference → persiapan privat → GET order → akun/quote, dengan
clock/route lease≤5detik bersama dan timestamp bukti asli. Metadata yang expired
ditolak tanpa refresh di tengah bukti. Mutasi tidak memakai lease. ID siklus
menghubungkan waktu tahap dengan pemeriksaan umur data.

`stop_code`, `exit_decision` dan `exit_outcome` opsional menjaga pembacaan/hash
legacy; outcome diturunkan hanya dari order dengan fill terverifikasi ledger.
Metadata absent pada arsip lama tetap absent, bukan ditulis ulang. Waktu exit
decision bukan waktu klik UI atau fill. Settlement Stop langsung Finished;
tanpa Stop tetap boleh Watching. Binary lama belum dijamin bisa membaca hash
state yang memuat field baru, sehingga rollback perlu pemeriksaan.
[Hasil dan batas kandidat](../../../docs/RELIABILITY_P1_07.md).

Kode Rust ini menghubungkan strategi → sizing → order → proteksi → penutupan →
ledger Binance USD-M. Tombol **Aktifkan sesi live** membuat satu sesi dengan
batas eksplisit pengguna; supervisor selanjutnya bekerja tanpa klik per order.
Startup, analisis arah, preflight dan probe memakai `Access::ReadOnly`.
`Access::LiveSession` hanya dibuat saat aktivasi/pemulihan melalui UI pengguna.
Tes lifecycle memakai exchange lokal tanpa kredensial/jaringan; probe nyata GET.

## Kontrak komponen

| Komponen | Perilaku |
|---|---|
| `margin` | Decimal eksak, tier maintenance, initial margin, sizing long/short, biaya dan estimasi likuidasi isolated |
| `protocol` | Fixed-host typed GET/POST/DELETE; market entry, reduce-only exit, TP/SL Algo close-all, cancel/query, leverage, isolated |
| `transport` | HMAC, TLS, server time, public DNS fallback sebelum mutasi, satu percobaan kirim, cooldown; error tidak menyalin respons privat |
| `outbox` | Reserve sebelum kirim, lock, 0600, fsync+atomic replace, checksum, client ID unik, rekonsiliasi GET, larangan regresi fill |
| `flow` | Keputusan otomatis konfigurasi → entry → SL → TP → penutupan → cleanup → membutuhkan pembukuan |
| `readiness` | GET akun/posisi/config/bracket/fee/open orders/algo dan harga BTCUSDT; data privat hanya memori |
| `quotes` | Depth5 BTCUSDT, ID monotonic, E/T exchange≤5s, level/spread valid; Quote memakai T tanpa restamping |
| `automatic` | Ensemble tetap breakout/pullback/range; candle selesai, resolusi konflik, arah dan SL/TP dasar |
| `planning` | Planner bersama preview/aktivasi/entry: SL2ATR, TP3–6ATR, sizing ulang/net RR; review ATR aktual dan jejak≤8percobaan tanpa wallet |
| `supervisor` | Satu posisi, validasi snapshot, intent persisten, dispatch/reconcile, batas sesi/rugi, settlement sebelum entry berikut |
| `ledger` | Query order/fill/funding berhalaman, deduplikasi, pemeriksaan quantity/quote/PnL dan komisi USDT |
| `session_store` | Lock global folder, binding hash key, intent/config/daily PnL, arsip transaksi, fsync/atomic replace/0600 |
| `live` | Worker native, aktivasi satu kali, polling, Stop, pemulihan close-only; key hanya memori/KWallet |
| `session_input` | Snapshot akun segar untuk pemulihan; riwayat candle hanya diminta ketika mencari/mengelola sinyal |

## Aturan strategi yang dibekukan

Versi `ensemble-2-cost-aware`, BTCUSDT, candle 5m yang sudah selesai. Warmup 80–240 bar;
sinyal harus ≤15 detik setelah close dan bukan candle sebelum aktivasi. Gap,
revisi candle atau pembacaan gagal menghentikan entry. Tidak ada catch-up entry.
Pada 0.6.3, revisi saat tanpa intent dapat direvalidasi menggunakan dua respons
riwayat identik terpisah ≥2 detik (jam monotonik dan server); semua candle hingga
baseline baru dikonsumsi TANPA entry. Evaluasi baru menunggu candle berikutnya.
Perubahan berulang, akun usang, Stop dan deadline tidak dilewati. Grace REST
dua detik menghindari pembacaan tepat di batas close; usia sinyal tetap ≤15 detik.

Sejak0.6.6, candle BARU juga perlu dua history identik≥2detik, tetapi ini
finalisasi normal, bukan error/rewarm. Baseline lama dipertahankan sampai
konfirmasi. Candle baru yang berubah sebelum konfirmasi mengulang penantian;
revisi pada candle lama tetap error/rewarm/skip. Finalisasi terlambat>15detik
tidak memperpanjang jendela entry. Data pulih mengonsumsi semua interval yang
sudah tertutup saat pemulihan, termasuk candle pending. Stop/deadline tidak
menunggu finalisasi dan tidak dihapus oleh hasil konfirmasi.

Quote preflight memakai GET `/fapi/v1/depth?symbol=BTCUSDT&limit=5` (weight2,
sama dengan bookTicker satu simbol). E adalah outputtime, T transactiontime;
keduanya diperiksa≤5detik dan≤serverTime; Quote menggunakan T. Cache/ID/waktu
yang mundur atau isi berubah pada ID sama ditolak. Ini tetap REST polling,
bukan stream atomik atau dukungan HFT. `PUBLIC_READ_HEALTH` mencatat endpoint
allowlisted, waktu dan route untuk error/latency≥1detik, tanpa URL/query/body.

Setup tetap dapat LAYAK BERSYARAT karena contoh ATR grid; kelayakan ATR aktual
ditampilkan terpisah dan bukan izin entry tanpa pola/data/funding. Input
`LIVE_ENTRY_REVIEW` schema1 berisi quote/mark/ATR/filter/bracket/fee/budget/side;
saldo tersedia diganti boolean `margin_available`, cukup untuk guard sizing.
Roundtrip replay diuji memakai budget.margin atau nol sebagai saldo surrogate.
Ini jejak ekonomi untuk diagnosis berikutnya, bukan replay penuh strategi
OHLCV/akun atau rekonstruksi satu Short sesi lama yang inputnya tidak tersimpan.

- ATR14/close harus 10–300 bps. Tren jika gap EMA20/50 ≥0.35ATR; range jika
  gap ≤0.20ATR. Kondisi di antaranya menunggu.
- Breakout: close melewati high/low20 candle sebelumnya, searah EMA, volume
  ≥1.1 kali rata-rata20 bar. Long dan Short memakai kondisi simetris.
- Pullback tren: bar sebelumnya menyentuh EMA20, lalu close terbaru melewati
  high/low bar tersebut dan EMA20 ke arah tren.
- Pantulan range: low/high menembus range20 lalu close kembali ke dalamnya,
  dengan candle bullish/bearish sesuai arah pantulan.
- Semua suara harus searah. Tidak ada suara atau suara bertentangan → menunggu.
- SL tetap 2ATR; TP mulai 3ATR dari bid/ask/mark, diperluas seperlunya hingga
  maksimum 6ATR (rounding menurut tick). Imbalan model setelah fee, funding
  reserve dan slippage entry/exit harus ≥1.1 kali kerugian model sampai stop.
  Perubahan TP menghitung ulang quantity dan biaya; SL/risiko tidak diperbesar.
  Maksimum delapan iterasi; gagal konvergen atau melewati cap berarti ditolak.
  TP akhir disimpan pada keputusan, rencana dan trigger proteksi yang sama.
- Entry ditahan 10 menit sebelum funding. Penutupan diminta 2 menit sebelum
  funding, setelah 30 menit memegang posisi, atau ketika arah sinyal berlawanan.
  Posisi lama dan biayanya harus selesai sebelum arah baru boleh dieksekusi.

Ini aturan implementasi, bukan hasil optimasi atau bukti keuntungan. TP yang
lebih jauh bisa mengurangi hit rate; timeout posisi 30 menit tetap berlaku.
Sesi lebih panjang memberi lebih banyak kesempatan mengamati candle, bukan
belajar atau mengubah parameter otomatis.

Preview **Periksa kelayakan setup** dan `--check-setup` hanya GET, tidak
mengaktifkan sesi atau mengubah catatan. Pemeriksaan memakai fee/rules akun,
saldo tersedia, sisa batas rugi harian UTC, dan batas pengguna. Budget margin
adalah plafon yang dijepit ke saldo tersedia; bukan syarat seluruh plafon harus
tersedia. Pencarian contoh ATR 0.10–3.00% berlangkah 0.0001% berhenti pada contoh
pertama per arah. Satu contoh layak cukup untuk kelayakan bersyarat; sinyal
aktual tetap harus lolos semua pemeriksaan. Tidak menemukan contoh pada grid
bukan bukti matematis seluruh harga masa depan mustahil. Aktivasi mengulang
pemeriksaan dengan snapshot baru sebelum sesi dibuat atau konfigurasi dikirim.

`Command::Protect` memakai `/fapi/v1/algoOrder`, `closePosition=true`,
`workingType=MARK_PRICE`, `priceProtect=false`, tanpa quantity/reduceOnly.
Penutupan biasa memakai MARKET dengan `reduceOnly=true`, arah berlawanan.
Command/Read tidak menerima URL, endpoint, header atau field arbitrer.
Strategi tidak mendapat akses secret atau raw HTTP.

Margin dihitung dari `notional / leverage`; maintenance `notional * rate - cum`.
Likuidasi diperkirakan dengan mencari titik equity isolated = maintenance di
setiap tier. Collateral model memakai notional pada harga entry model/leverage,
bukan seluruh saldo cadangan. Supervisor mengecek harga fill terhadap batas
entry dan harga likuidasi yang dilaporkan exchange terhadap stop jika tersedia;
tidak menghitung ulang isolated wallet secara terpisah. Cadangan fee memakai
taker akun untuk entry dan exit; funding reserve adalah batas model, sedangkan
funding aktual dibaca dari income history saat settlement. Stop wajib berada
sebelum perkiraan likuidasi ditambah buffer. Gap/slippage nyata tetap bisa
melampaui risiko model. `notionalCoef != 1` ditolak sampai semantiknya didukung.

## Timeout, restart dan lifecycle

Riwayat0.6.5–0.6.6: `prepare_reference → reconcile_orders → fetch frame → tick_prepared`.
Rekonsiliasi GET dibatasi16 record, memakai satu persiapan waktu, menghasilkan
bukti sekali pakai terikat intent. Umur bukti sejak GET pertama (setelah DNS/
clock), akun dan quote masing-masing wajib≤5detik; bukan menempel timestamp
baru pada data lama. Metadata mendapat headroom5detik sebelum query order,
tetapi TTL60detik tetap diperiksa saat fetch. Stop/deadline dapat membatalkan
GET; restart tidak membawa bukti memori lama. Proteksi NEW yang hilang di
snapshot open-algo terbaru meminta close-only. REST tetap tidak atomik.

Jika respons POST market tidak memuat `cumQuote`, driver meminta satu GET
konfirmasi (timeout5detik) dan tetap memvalidasi receipt lengkap. Kegagalan
tetap Uncertain; tidak resend POST. Fill yang berhasil dikonfirmasi masuk
jalur SL segera. Log hanya label command/endpoint, kategori dan umur/latency;
tidak merekam raw response/URL bertanda tangan atau secret.

1. Caller membuat ID intent stabil serta menyimpan konfigurasi intent sebelum
   mengirim. Driver menyimpan status `Uncertain` sebelum satu percobaan mutasi.
2. Timeout, 408/5xx, respons rusak atau kehilangan respons tidak menjadi bukti
   penolakan. Dispatch yang sama tidak dikirim ulang, termasuk setelah restart.
   `NotFound` saat GET juga tidak mengizinkan POST ulang.
3. `Intent::next` memeriksa posisi/quote segar (maksimum 5 detik), identitas
   intent, mode akun, kepemilikan order, deadline dan perubahan harga.
   Leverage/margin harus terbaca sesuai sebelum entry.
4. Fill parsial dipasang SL terlebih dahulu, kemudian sisa entry dibatalkan.
   Cancel yang berlomba dengan fill tetap memakai jumlah terisi aktual.
   Respons SL hilang direkonsiliasi dengan ID yang sama. Penolakan proteksi
   mengarahkan lifecycle ke penutupan reduce-only setelah entry terminal.
5. Proteksi exchange dipertahankan sampai posisi terbaca flat dan entry tidak
   dapat menambah exposure. ACK cancel Algo belum membuktikan cleanup; GET
   harus memastikan terminal dan child order tetap dicatat.
6. `NeedsAccounting` mengembalikan ID entry/exit/child untuk pembukuan. Ini
   **bukan** status selesai atau izin entry berikutnya. Semua biaya dan fill
   harus direkonsiliasi sebelum sesi boleh melanjutkan.

Supervisor memproses error/`Halt`/`Observe`/`Uncertain` dengan rekonsiliasi;
tidak menjalankan intent lain untuk melewati hasil entry yang belum pasti. `Watch` meminta
pemantauan berkelanjutan terhadap order/proteksi dan posisi, bukan jaminan
proteksi tetap aktif. Outbox menolak receipt yang menurunkan cumulative fill,
mengganti ID exchange/child, atau menghidupkan kembali order terminal. Receipt
terakhir tetap disimpan saat pemeriksaan gagal; caller harus menghentikan
kemajuan dan memperoleh keadaan yang konsisten.

## Supervisor, ledger dan pemulihan

- REST setiap ±2 detik ditambah waktu permintaan; akun/quote wajib ≤5 detik.
  Metadata/tier/fee disegarkan tiap60 detik. GET berurutan bukan snapshot atomik.
  Snapshot awal, feeBurn, available balance, mode akun dan maxNotional harus
  terbaca. Fee BNB tidak didukung, tidak diubah otomatis. Isolated/leverage
  diperiksa lagi sebelum entry. Satu sesi hanya satu posisi BTCUSDT.
- Intent dan candle yang sudah diproses disimpan sebelum dispatch. SL langsung
  dikirim setelah fill entry terkonfirmasi, sebelum poll berikut. Ada jeda nyata
  antara entry dan penerimaan SL; sistem ini bukan bracket atomik di exchange.
- Data gagal menjeda entry saat belum ada intent bot, hingga data pulih atau
  deadline sesi asli. Tidak ada restart/perpanjangan otomatis. Frame pemulihan
  wajib melewati pemeriksaan akun/posisi/order/wallet; candle saat pemulihan
  dikonsumsi tanpa entry, termasuk jika beberapa candle terlewati. Deadline
  monotonik dapat membatalkan GET menggantung dan menghentikan entry; selesai
  tetap memerlukan verifikasi akun/ledger. Dengan intent, data gagal memakai
  jalur stop/close-only. Jika query proteksi gagal namun ownership
  entry dan posisi segar terbukti, supervisor meminta exit reduce-only.
  Saat seluruh koneksi gagal, penutupan menunggu data pulih; SL exchange tetap
  ada jika sebelumnya diterima. Maksimum3 upaya exit dengan ID berbeda hanya
  setelah upaya sebelumnya terbukti terminal/reject dan posisi residual terbaca.
- Setelah Stop (termasuk data gagal saat ada intent), polling memakai mode
  pemulihan tanpa riwayat candle.
  Revisi candle tetap menggagalkan entry, tetapi tidak mengunci rekonsiliasi
  pada cache candle yang sama. Mode ini tetap mensyaratkan akun dan quote segar;
  tidak menghidupkan kembali entry atau menilai sinyal dari cache yang gagal.
- Pemulihan kosong lewat CLI menggunakan transport `ReadOnly` dan lock/binding
  state existing. Dua GET wajib menunjukkan akun kosong dan wallet cocok;
  sequence/completed/current/net/fee/funding harus menunjukkan belum ada trade.
  Hanya status lokal yang diakhiri; tidak ada POST/DELETE Binance.
- Stop/deadline/batas rugi menutup posisi. Batas harian UTC persisten dan batas
  sesi memakai hasil bersih, floating serta cadangan biaya; gap dapat melewatinya.
  Ukuran entry juga dibatasi sisa budget rugi. Maks100 trade/24jam per sesi.
- Ledger mengumpulkan fill entry, exit dan child Algo; order harus terminal,
  qty entry=exit, cumQuote dan realized PnL cocok, tanpa ID duplikat. Pagination
  maksimal20 halaman×1000 per query; keterbatasan/ketidaklengkapan menahan entry.
  Funding yang terlewati harus ada; biaya selain USDT ditolak. Net = gross − fee
  + funding dan harus cocok dengan perubahan wallet (toleransi1e-6 USDT).
  Dua pembacaan identik berjarak≥5 detik diperlukan sebelum trade dianggap selesai.
- Direktori user-local `binance-desk/live` berizin0700, file0600. `session.json`
  menyimpan intent/config/status/daily; outbox per intent menyimpan bukti order;
  `*-ledger.json` menyimpan versi strategi, alasan entry, batas, rencana dan hasil.
  Arsip ditulis sebelum ringkasan sesi; crash di antaranya tidak menggandakan PnL.
  Lock global folder melarang dua jendela menjalankan mesin bersamaan. Tidak ada
  key/secret mentah; binding memakai hash domain+API key. Migrasi/rotasi key belum
  didukung. Jangan memindahkan folder/state untuk menjalankan sesi kedua.
- Startup hanya menampilkan catatan terakhir tanpa mengklaim posisi aktual.
  Pemulihan hanya reconcile+close, tanpa entry baru. Tutup jendela meminta Stop
  dan menunggu selesai; crash/kill/listrik mati perlu pemulihan oleh pengguna.

## Batas yang masih terbuka

Tidak ada uji acceptance POST/DELETE Binance production, testnet execution,
WebSocket/user-data stream, HFT, Hedge, multi-asset, Portfolio Margin, COIN-M,
multi-simbol, atau bukti profit. Akun harus didedikasikan selama sesi: posisi
asing/order BTC asing/perubahan wallet tidak diadopsi. Order aktif simbol lain
belum diinventarisasi; posisi non-BTC terdeteksi dan menghentikan sesi.
Perubahan manual yang kebetulan saling meniadakan pada wallet tidak dapat
diidentifikasi seluruhnya oleh rekonsiliasi REST ini.

Outbox per intent maksimal1024 record/4MiB; state sesi2MiB. Arsip trade disimpan,
belum ada kebijakan retensi otomatis. Kegagalan IO
memerlukan pembukaan ulang; crash tepat sesudah reserve bisa menyisakan status
tidak pasti meski request belum pernah terkirim. Ini mengutamakan pencegahan
duplikat; ketidakpastian permanen memerlukan pemeriksaan manual. SHA256 mendeteksi
kerusakan biasa, bukan manipulasi oleh pemilik file.

## Acuan resmi

- [Trade API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade): order, Algo Order, cancel/query, margin dan leverage.
- [Account API](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/account): bracket, commission, feeBurn, income dan konfigurasi.
- [General info](https://developers.binance.com/en/docs/products/derivatives-trading-usds-futures/general-info): status hasil tidak pasti dan batas laju.
- [Leverage dan margin](https://www.binance.com/en/futures/trading-parameters/perpetual/leverage-margin): initial/maintenance margin serta tier.
- [Rumus likuidasi USD-M](https://www.binance.com/en/support/faq/detail/b3c689c1f50a44cabb3a84e663b81d93): isolated wallet balance sebagai collateral posisi.

Dokumentasi diperiksa 16 September 2026. Bukti GET akun tidak membuktikan bahwa
POST/DELETE, izin TRADE, proteksi exchange atau profit sudah teruji.
