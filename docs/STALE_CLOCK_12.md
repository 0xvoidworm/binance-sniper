# Rentang clock dan revalidasi snapshot — 0.6.12

## Masalah yang diperbaiki

0.6.11 menyimpan `serverTime` bersama `Instant` setelah body `/fapi/v1/time`
selesai dibaca. Proyeksi itu kemudian diperlakukan sebagai satu waktu pasti.
Akibatnya depth sesi 21 September dengan `E=+17 ms` dan `T=+14 ms` terhadap
proyeksi tersebut diklasifikasikan sebagai timestamp future dan mengunci Stop,
meski snapshot berikutnya sehat.

Audit tidak membuktikan Wi-Fi, ISP, arah delay, atau server sebagai penyebab
fisik. Delay 120 ms pada regresi hanyalah fault injection deterministik.

## Bukti clock baru

`account::ClockSample` mencatat `request_started` tepat sebelum GET time yang
berhasil dan `response_completed` setelah body JSON terbatas selesai dibaca.
Untuk nilai Binance `S`, clock pada `response_completed` direpresentasikan
sebagai:

```text
earliest = S
latest   = S + (response_completed - request_started)
```

Rentang ini mencakup request, pemrosesan server, perjalanan respons, dan baca
body. Implementasi tidak membagi RTT dua dan tidak menyatakan delay satu arah.
Rentang diproyeksikan hanya dengan `Instant` monotonic lokal. Dua sampel
berurutan diiriskan bila konsisten. Sampel yang tidak beririsan membuang satu
siklus dan menjadikan sampel baru sebagai baseline; irisan tidak dibawa sebagai
bukti mentah ke siklus berikutnya agar tidak menciptakan presisi palsu.

Timestamp signing memakai batas bawah. Nilai ini tidak dapat berada di masa
depan server berdasarkan bukti sampel. Sampel dengan rentang lebih dari
`recvWindow=5000 ms` ditolak sebagai ketidakpastian sinkronisasi; tidak ada POST
yang dicoba dan tidak ada retry POST.

## Keputusan berdasarkan rentang

| Keputusan | Batas yang dipakai | Alasan |
|---|---:|---|
| Freshness depth/quote | usia terburuk dari `latest` | data ambigu tidak dapat membuat order |
| Bukti candle sudah tutup | `earliest` | penutupan harus sudah pasti terjadi |
| Late signal, funding, risiko, deadline | `latest` | mencegah entry terlambat dan sesi melewati batas |
| Awal sesi | `earliest` | deadline tidak diperpanjang oleh ketidakpastian |
| Batas harian UTC | kedua batas harus pada hari yang sama | ledger tidak berpindah hari saat clock ambigu |
| Signing GET/POST | `earliest` dan rentang maksimal 5 detik | tidak future dan masih dalam `recvWindow` |

Klasifikasi timestamp snapshot:

- di atas `latest`: `InvalidTimestamp`, terminal;
- di antara `earliest` dan `latest`: `ClockUncertain`, frame dibuang dan
  sinkronisasi diulang;
- pasti lebih tua dari batas freshness: `StaleTimestamp`/`StaleRead`;
- rentang melintasi batas freshness: `ClockUncertain`;
- schema, urutan timestamp, level depth, spread, ID mundur, atau replay tetap
  kegagalan integritas terminal.

Saat flat, `ClockUncertain` menjeda data, membuang candle pemulihan, dan menunggu
candle baru tanpa mengubah deadline. Saat ada intent, jenis ini masuk jalur
close-only yang sama dengan gangguan data terklasifikasi: maksimal dua episode,
accounting dan rekonsiliasi tetap wajib, lalu 60 detik kesehatan dan candle baru.
Stop terminal tidak pernah dibuka kembali.

## Keterkaitan kode

- `src/account.rs`: menangkap bukti request sampai body selesai dan memilih
  timestamp signing fail-closed.
- `src/futures/transport.rs`: `ClockWindow`, irisan sampel, expiry lease, dan
  klasifikasi stale/uncertain/invalid.
- `src/futures/readiness.rs` dan `quotes.rs`: parse depth/mark/funding memakai
  rentang yang sama tanpa GET time setelah snapshot.
- `src/futures/session_input.rs` dan `live.rs`: sampling candle memakai batas
  bawah; dua-read confirmation dan larangan mengejar candle lama dipertahankan.
- `src/futures/supervisor.rs` dan `recovery.rs`: freshness, UTC, deadline,
  recovery, Stop dan accounting memakai kelas error bertipe.
- `src/futures/diagnostics.rs` dan `evaluation/package.rs`: build 0.6.12 dapat
  diekspor; paket 0.6.8–0.6.12 tetap dapat dibaca dengan larangan klaim build
  masa depan oleh exporter lama.

Format state, outbox, ledger, strategi, leverage, risk limit, durasi pengguna,
dan schema arsip tidak berubah.
