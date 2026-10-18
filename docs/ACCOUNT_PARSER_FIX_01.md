# Perbaikan parser akun 01 — 16 September 2026

Mulai sekitar 18:11 WIB. Pengguna memasukkan HMAC secara sesi-only dan menerima
pesan "Data akun Binance belum lengkap atau formatnya berubah". Kredensial
tidak tersedia di KWallet; jangan mengambilnya dari memori/clipboard, meminta
di chat, atau mengubah preferensi penyimpanannya.

## Lingkup dan batas sebelum perubahan

- Cocokkan parser dengan schema resmi serta metadata pasar publik; reproduksi
  bug dari fixture publik/nonrahasia. Satu putaran maksimal 60 menit aktif.
- Perbaiki hanya penanganan respons/diagnosis yang terbukti perlu. Diagnosis
  boleh menyebut endpoint dan field yang diizinkan, tipe/kategori kegagalan;
  tidak boleh mencatat response mentah, nilai akun, key/secret/signed URL.
- Tetap signed GET empat endpoint akun; tanpa order/perubahan akun/risk/strategi.
- Verifikasi regresi, format/Clippy/test/release dan native UI; pasang patch dan
  buka jendela baru tanpa menutup sesi pengguna yang menyimpan key di memori.
- Penerimaan: fixture valid diterima, data rusak tetap ditolak, diagnosis aman.
  Konfirmasi live perlu pengguna mencoba lagi di versi patch atau KWallet bila
  pengguna memilih menyimpan sendiri. Jangan klaim akar masalah akun terkonfirmasi
  hanya dari contoh publik; catat hasil live terpisah.
- Bukti: `Workspace/outputs/binance-account-parser-fix01-20260916`.

Keputusan LANJUT jika patch software terverifikasi; penerimaan akun live tetap
TERHAMBAT bila belum dapat diuji. Hindari loop patch spekulatif tanpa diagnosis.

## Hasil

- Metadata resmi `/fapi/v1/exchangeInfo` memuat 897 simbol, lima non-ASCII.
  Validasi ASCII-only di parser seluruh symbolConfig menyebabkan fixture publik
  ditolak. Regresi gagal sebelum patch, lulus dengan validasi Unicode huruf/angka
  dan underscore, tanpa mengubah nama bursa. Data/control/duplikat invalid tetap
  ditolak. Tidak ada field hilang yang diubah menjadi saldo nol.
- 0.3.1 menambahkan diagnosis endpoint/check statis dan log transisi status saja,
  tanpa respons mentah/nilai akun/secret. 34 tes dan pipeline penuh/audit lulus.
- KWallet kemudian tersedia dari input pengguna. **Worker UI dan CLI probe
  terpasang masing-masing berhasil membaca dua snapshot akun nyata**. Capture
  native terhubung diperiksa dengan data akun disamarkan. Sesi lama tidak ditutup.
- Keputusan **LANJUT: parser dan pembacaan akun live terverifikasi**, tanpa
  aktivasi order, perubahan akun/leverage, strategi atau klaim profit.
- [Paket bukti](../../../outputs/binance-account-parser-fix01-20260916/DECISION.md).

## Sumber

- Metadata publik resmi: `https://fapi.binance.com/fapi/v1/exchangeInfo`.
- [Schema akun/simbol Binance](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/account).
- [Schema posisi Binance](https://developers.binance.com/en/docs/catalog/core-trading-derivatives-trading-usd-s-m-futures/api/rest-api/trade).
