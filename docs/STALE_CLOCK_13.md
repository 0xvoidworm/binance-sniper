# Refinement dan pembulatan rentang clock — kandidat 0.6.13

## Temuan review 0.6.12

`Session::updated` menyimpan batas atas terbaru. Supervisor lalu membandingkan
batas atas frame berikutnya dengan nilai itu. Saat rentang yang lebar
`[t,t+4500]` disempurnakan menjadi rentang konsisten `[t+2500,t+2600]`, batas
atas memang turun walau batas bawah maju dan kedua bukti beririsan. Perbandingan
dua estimasi latest salah mengklasifikasikannya sebagai clock mundur.

Selain itu, `ClockSample::upper_at` membulatkan RTT dan proyeksi secara terpisah
ke bawah. RTT 1,8 ms ditambah proyeksi 1,8 ms menjadi 1+1=2 ms, sehingga upper
`S+2` tidak mencakup kemungkinan sah `S+3`.

## Model waktu 0.6.13

`ClockSample` dan `ClockWindow` mempertahankan endpoint dalam nanodetik selama
pembentukan, proyeksi, reanchor, dan intersection. Konversi ke timestamp
milidetik hanya dilakukan di tepi keputusan:

```text
lower_ms = floor(S + elapsed_sejak_body_selesai)
upper_ms = ceil(S + elapsed_sejak_body_selesai + durasi_request_sampai_body)
```

Untuk setiap nilai sah `x` di dalam rentang kontinu, sifat floor/ceil memberi
`floor(lower) <= x <= ceil(upper)`. Pecahan durasi dijumlahkan sebelum
pembulatan. Reanchor berulang menambahkan durasi ke endpoint nanodetik, jadi
tidak menumpuk kehilangan carry atau satu milidetik pada setiap langkah.
Pembulatan keluar berasal dari resolusi timestamp milidetik dan durasi yang
diukur; tidak ada toleransi future tetap.

Signing tetap memakai lower yang dibulatkan ke bawah. Ketidakpastian request
tetap maksimal lima detik dan lease tetap kedaluwarsa lima detik. Timestamp
snapshot di atas upper tetap `InvalidTimestamp`; timestamp yang hanya mungkin
karena lebar rentang tetap `ClockUncertain` dan tidak dapat membuat order.

## Progres sesi dan metadata

Tiga hal sekarang dipisahkan:

- `ClockWindow` adalah bukti waktu frame pada proses berjalan;
- `Session::clock_floor_ms` adalah lower bound terbesar yang benar-benar sudah
  diterima dan menjadi bukti rollback persisten;
- `Session::updated` adalah high-water untuk tampilan/arsip dan tidak lagi
  dipakai sebagai bukti urutan clock.

Refinement diterima bila latest frame baru masih mencapai `clock_floor_ms`.
Lower yang baru kemudian menaikkan floor. Rollback nyata terjadi bila seluruh
rentang baru berada di bawah floor (`current_latest < clock_floor_ms`) dan tetap
terminal. Sampel berurutan yang disjoint tetap ditolak lebih awal oleh
`Connector::begin_read_window` sebagai ketidakpastian sinkronisasi dan memerlukan
revalidasi; guard itu tidak dihapus.

Field floor bersifat opsional agar state lama tetap dapat dibaca dan
diserialisasi kembali tanpa membuat bukti historis. Restart tidak membawa bukti
ephemeral dan tetap masuk jalur Stop/close-only yang sudah ada.

## Dampak keputusan waktu

| Keputusan | Bukti yang dipakai |
|---|---|
| Freshness account/quote/depth | usia terburuk dari upper, batas 5 detik |
| Candle benar-benar tutup | lower |
| Session deadline, funding, batas rugi/trade | upper; awal/deadline berasal dari lower |
| Deadline intent 15 detik | lower saat frame diperoleh, lalu diuji dengan upper |
| Rentang ledger | mulai dari lower frame pembuat intent; akhir dari upper saat flat |
| Wait accounting 5 detik | current lower dikurangi upper yang disimpan |
| Health recovery 60 detik | waktu lokal 60 detik dan current lower melampaui first upper 60 detik |
| Batas harian UTC | lower dan upper wajib berada pada hari yang sama |
| Tampilan/arsip selesai | `updated` high-water metadata |

Konsekuensinya, ketidakpastian dapat menunda accounting atau health recovery,
dan dapat mempercepat penolakan entry/penutupan karena deadline. Ketidakpastian
tidak dapat memalsukan durasi minimum, memperpanjang peluang entry, atau
menghidupkan kembali Stop.

## Batas perubahan

Strategi, leverage, risk/trade limit, durasi pengguna, freshness lima detik,
recovery maksimal dua episode, cancellation, accounting, larangan retry POST,
dan restart tanpa auto-session tidak berubah. Kandidat ini belum dipasang dan
belum menjadi acceptance live.
