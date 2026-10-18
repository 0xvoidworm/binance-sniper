# Konsistensi anchor aktivasi dan recovery — kandidat 0.6.14

## Temuan review 0.6.13

Dua jalur memakai endpoint waktu akuisisi tetapi memasangkannya dengan anchor
lokal yang berasal dari saat pemrosesan:

1. aktivasi menyimpan `clock_floor_ms` dari lower yang sudah diproyeksikan
   setelah pemeriksaan FeeBurn, lalu tick pertama membandingkannya dengan upper
   mentah dari frame yang sama; dan
2. recovery menyimpan interval server saat akuisisi tetapi membuat
   `tokio::time::Instant::now()` baru ketika supervisor memproses frame.

Akibatnya, jeda pemrosesan yang sah tampak sebagai rollback pada tick pertama
atau sebagai interval recovery yang terpisah. Reproduksi harness sebelum patch
memberi 0 PASS / 3 FAIL.

## Kontrak aktivasi 0.6.14

Satu frame kini membawa dua bukti yang sengaja dibedakan:

- waktu aktivasi diproyeksikan ke saat `Engine::begin` dan dipakai untuk
  `Session::started` serta `deadline`; durasi yang dipilih pengguna tetap utuh;
- lower mentah pada saat akuisisi frame disimpan sebagai `clock_floor_ms` dan
  dibandingkan dengan endpoint akuisisi pada tick pertama.

Dengan begitu, jalur produksi tetap `frame → pemeriksaan tambahan → begin →
tick frame yang sama`, tetapi kedua sisi pemeriksaan rollback mempunyai
provenance yang sama. Tick berikutnya tetap menaikkan floor hanya dengan lower
yang benar-benar diterima. Seluruh rentang baru di bawah floor tetap menjadi
`ClockRollback` terminal; tidak ada `max(old,new)` sebagai jam global dan tidak
ada toleransi baru.

State 0.6.13 sah bila floor berada paling jauh lima detik sebelum start. Batas
ini bukan kelonggaran freshness baru: aktivasi sendiri masih menolak frame bila
selisihnya lebih dari lima detik, dan timestamp account/quote/depth tetap tunduk
pada guard freshness lima detik. State legacy tanpa floor tetap dibaca tanpa
membuat bukti historis. Floor yang mendahului start juga diuji melewati commit
dan reopen.

## Kontrak recovery 0.6.14

`ClockWindow` sekarang membawa dua anchor monotonik untuk sampel yang sama:

- `std::time::Instant` untuk proyeksi endpoint clock pada jalur produksi; dan
- `tokio::time::Instant` untuk pembuktian durasi lokal, termasuk tes paused-time.

Anchor Tokio dibuat saat clock window dibentuk, dibawa melalui proyeksi dan
intersection, lalu dikirim ke recovery sebagai `acquired`. Supervisor juga
mencatat `observed` saat pemrosesan. `Healthy` memproyeksikan interval lama
dengan selisih acquisition-to-acquisition, bukan observation-to-observation.
Jitter pemrosesan karena itu tidak mengubah hubungan endpoint.

Recovery tetap mensyaratkan semuanya sekaligus:

- minimal dua sampel;
- sedikitnya 60 detik waktu lokal sejak akuisisi pertama;
- current lower sedikitnya 60 detik melewati first upper;
- gap akuisisi dan gap observasi masing-masing paling lama 10 detik;
- endpoint maju, beririsan setelah proyeksi, dan bukan replay/rollback.

Gap, replay, rollback, interval disjoint, atau observation mundur mereset bukti.
Tidak ada epsilon/toleransi baru. Bukti tetap ephemeral dan restart tidak
melanjutkan penghitung health.

## Guard yang tidak berubah

Freshness lima detik, pembulatan endpoint keluar, uncertainty recoverable,
accounting lima detik, UTC/funding, signing memakai lower, recvWindow, Stop,
deadline, cancellation, maksimal dua episode recovery, hanya candle baru,
larangan duplicate order dan no blind POST retry tetap berlaku. Strategi,
leverage, risk, limit, dan durasi tidak diubah.

Kandidat 0.6.14 ini hanya artefak review. Belum dipasang, tidak dijalankan
live/paper, dan tidak melakukan order atau mutasi akun.
