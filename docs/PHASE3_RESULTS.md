# Fase 3 — strategi lebih terukur, keunggulan belum terbukti

Diperbarui 14 September 2026 WIB. Hasil: **NO_PROMOTION**. Delapan variasi sudah dibangun dan diuji; semuanya masih negatif setelah asumsi biaya. Tidak ada order dikirim. Ini penyelesaian fase riset dan implementasi observer, bukan klaim mesin sudah matang untuk menghasilkan uang.

Buka [Strategy Lab lokal](http://127.0.0.1:8765/?tab=lab). **HASIL UJI** memuat perbandingan dan bukti sinyal; **OBSERVASI BARU** menampilkan tahapan/area delapan hipotesis secara terpisah. Chart utama masih baseline. Area kosong berarti belum terbentuk, bukan gambar yang gagal dibuat.

## Apa yang diperbaiki

- Area retracement benar-benar mengikuti 35–65% gerakan ekspansi, bukan kotak berpusat pada asal impulse. Hipotesis breakout/retest dan sweep/reclaim diuji terpisah.
- Konteks H1 menentukan arah; H4 yang berlawanan membatalkan, tetapi H4 netral tidak otomatis menghentikan pencarian.
- Ekspansi harus searah, cukup besar relatif terhadap volatilitas, didukung volume relatif, dan menembus struktur yang sudah diketahui. Trigger M5 memakai candle tertutup dan konfirmasi arah.
- Stop mengacu ekstrem yang sudah teramati dengan buffer/floor volatilitas. Target dibatasi ruang struktur. Rasio estimasi biaya terhadap jarak risiko diperiksa; ini bukan aturan alokasi modal 1%.
- Area punya waktu kedaluwarsa, pembatalan karena gap/perubahan konteks/invalidation, dan satu pemicu. Cooldown mencegah tumpang tindih sinyal pada pair yang sama dalam satu variasi.
- Horizon empat candle dibandingkan dengan dua variasi delapan candle. Sinyal menyimpan area, konteks dan referensi saat keputusan agar dapat diperiksa secara visual.
- Engine pola yang sama berjalan pada replay historis dan observasi live. State eksperimen, sinyal dan outcome disimpan secara transaksional untuk pemulihan restart.

Perubahan ini memperbaiki definisi serta kemampuan audit. Apakah perubahan meningkatkan hasil perdagangan harus dijawab oleh data, bukan kerumitan kode atau tampilan.

## Data dan cara uji

[Protokol](PHASE3_PROTOCOL.md) dibekukan sebelum melihat outcome baru: delapan variasi, tanpa pencarian parameter tambahan setelah hasil keluar. Periode Phase 2 yang sudah diketahui tidak dipakai sebagai validasi baru.

Data baru: **20 Juni 2025 → 15 Juni 2026 UTC, akhir eksklusif**, enam pair BTC/ETH/SOL/BNB/XRP/DOGE USDT perpetual, H4/H1/M15/M5. Total **899.520 candle termasuk warmup**, 24/24 feed lengkap, **6.480 catatan funding** (1.080 per pair), serta **622.080 waktu keputusan M5** lintas enam pair. Candle divalidasi dan checksum feed diperiksa saat replay. Unduhan memakai public market-data GET dan funding berpaginasi, sesuai [dokumentasi Binance USDⓈ-M market data](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/market-data).

120 hari awal untuk kalibrasi, tiga test bergulir masing-masing 60 hari dengan training yang meluas, lalu 60 hari terakhir sebagai holdout. Pemilihan hanya membaca hasil sebelum periode test. Semua variasi memakai purge/embargo delapan M5 di batas. Ketidakpastian memakai bootstrap blok kalender tujuh hari, seluruh pair dalam blok yang sama, 1.000 draw deterministik.

Entry model pada open berikutnya, bukan close yang sudah terlihat. Biaya dasar per sisi: fee 5 bps + slippage 2 bps; stress: 7,5 + 5 bps. Funding historis disertakan, waktu charge/credit ambigu ditangani konservatif. Stop/target intrabar ambigu memakai stop terlebih dahulu. Ini model event OHLC, bukan simulasi order book, return akun, atau eksekusi nyata.

## Hasil yang menentukan

Pilihan training di ketiga fold dan sebelum final holdout: **SWEEP_T1_4**. Dipilih tidak sama dengan lolos. Holdout akhir: **16 April → 15 Juni 2026 UTC**.

| Pengujian pada periode yang sama | Event selesai | Mean net R | Mean stress R | Profit factor |
| --- | ---: | ---: | ---: | ---: |
| Baseline / final holdout | 25 | -0,563761 | -0,791384 | 0,0776 |
| SWEEP_T1_4 / final holdout | 76 | -0,227142 | -0,402329 | 0,4793 |
| Pilihan training bergulir / tiga fold | 347 | -0,232118 | — | 0,4827 |

Kerugian rata-rata per event memang lebih kecil daripada baseline. Namun jumlah event lebih banyak: penjumlahan unit event R pada holdout adalah **-17,263R** untuk pilihan baru dibanding **-14,094R** baseline. Penjumlahan itu pun bukan hasil portfolio/modal karena jarak stop, kejadian simultan, dan sizing berbeda. Jangan mengubah perbandingan mean menjadi klaim return lebih baik.

Tiga fold masing-masing: 159 event / -0,175342R; 92 / -0,223670R; 96 / -0,334249R. Tidak satu pun positif. Batas bawah satu sisi 95% mean net holdout: **-0,316885R**, stress **-0,491191R**. Win rate holdout 42,1%; 43 event berhenti karena batas waktu, 17 target, 16 stop. Fee saja rata-rata mengonsumsi 0,181013R per event; gerakan yang ditangkap belum cukup menghasilkan net positif.

Gagal bukan hanya karena 76 event belum mencapai syarat 100: mean training, mean holdout, stress, profit factor, dan seluruh fold juga gagal. Memperlonggar batas jumlah sampel tidak mengubah angka kerugiannya.

## Semua variasi, bukan hanya pilihan terbaik

Tabel seluruh 360 hari di bawah adalah diagnostik setelah uji, **bukan dasar memilih strategi pada holdout**. Angka dibulatkan.

| Variasi | Horizon M5 | N | Mean net R | Mean stress R |
| --- | ---: | ---: | ---: | ---: |
| Baseline | 4 | 163 | -0,4248 | -0,6515 |
| P15_T1_4 | 4 | 151 | -0,2672 | -0,4271 |
| P15_T15_4 | 4 | 151 | -0,2501 | -0,4109 |
| P60_T1_4 | 4 | 302 | -0,2521 | -0,4122 |
| RETEST_T1_4 | 4 | 105 | -0,3041 | -0,4603 |
| SWEEP_T1_4 | 4 | 658 | -0,2187 | -0,3919 |
| SWEEP_T15_4 | 4 | 658 | -0,2224 | -0,3956 |
| P15_T1_8 | 8 | 150 | -0,2720 | -0,4323 |
| RETEST_T1_8 | 8 | 104 | -0,3218 | -0,4774 |

Memperpanjang dua hipotesis dari empat menjadi delapan candle tidak menyelamatkan hasil. Subkelompok SOL pada holdout sweep kebetulan positif (+0,094352R, 19 event), tetapi itu temuan setelah membuka holdout dan sampelnya kecil. Mengambil SOL saja sekarang bukan validasi strategi baru.

## Yang masih belum dibuktikan

Studi ini retrospektif pada universe yang dipilih saat ini, bukan pengamatan masa depan. Hanya delapan hipotesis ini yang diuji; kegagalannya tidak membuktikan semua pendekatan trading mustahil. Data tick/order book, fill/latency nyata, tick rounding, portfolio/sizing, liquidation dan pengelolaan order belum ada. Funding live belum direkonsiliasi sehingga live `net_r` tetap null; hasil sebelum funding dilabeli terpisah. Tes restart deterministik bukan soak beberapa hari.

Langkah berikutnya memerlukan hipotesis sebab-akibat baru dan rencana validasi yang didaftarkan sebelum hasilnya dilihat. Jika ingin meneliti informasi likuiditas/aliran transaksi atau respons lintas-pair, tentukan dahulu data yang tersedia secara kausal dan biaya mendapatkannya; jangan menambahkan fitur hanya karena kompleks. Seluruh 450 hari yang kini telah dianalisis harus dianggap sudah terlihat. Observasi baru tetap direkam tanpa mempromosikan variasi gagal.

## Reproduksi dan provenance

- Trial final: `lab-e98337744f4aefe8ba8625fc`.
- Dataset: `binance-1750377600000-1781481600000-v1`.
- Protocol SHA-256: `a570409b895121e151a5790a63345f16cf8711945aa2eb784b22cdfabbabddaf`.
- Config SHA-256: `8e5f7eedaba88cb2b8942ae905c311de4d70a375d4392f45ab568d0744f9e17a`.
- JSON ekspor: `C:\Users\yuri\Desktop\Workspace\outputs\binance-sniper-phase3-360d.json`.
- JSON SHA-256: `29c8a64533de478397822e5e889c10765dc3d89dc88663aaadfad20aaf75fd13`.

Hash sumber per file dan manifest feed/funding ada di JSON. Trial sebelumnya `lab-6b33792f035df1f24d6cbbac` dipertahankan; pengulangan menambahkan binding config yang kurang pada provenance, bukan menyesuaikan strategi. Validasi, baseline dan semua hasil variasi identik (ID run berbeda). Jangan menghitungnya sebagai dua sampel independen.

Kompatibilitas model: kode exit lama `FOUR_BAR_CLOSE` masih tersimpan untuk time stop; `costs.horizon` adalah horizon sebenarnya, diuji numerik dan ditampilkan sebagai `TIME STOP / 4 M5` atau `TIME STOP / 8 M5`. Perbaikan funding tepat di boundary exit-gap diuji dan tidak mengarang credit yang waktunya tidak pasti.

Ulangi perintah di README untuk dataset/trial yang sama. Tidak ada perubahan API whitelist, kredensial, leverage, margin, posisi, atau order.
