(function(root) {
  'use strict';
  const api = {
    duration(ms) { const s=Math.max(0,Math.floor(ms/1000));return [Math.floor(s/3600),Math.floor(s/60)%60,s%60].map(n=>String(n).padStart(2,'0')).join(':'); },
    live(snapshot,row,elapsed,connected) { return Boolean(connected && snapshot?.status==='OBSERVING_LIVE_MARKET' && row?.status==='OBSERVING' && row.price_fresh && row.price_age_ms+elapsed<=9000 && row.h4_success_age_ms+elapsed<=90000); },
    reason(row) {
      if (row.status!=='OBSERVING') return `OBSERVASI TERTAHAN · ${row.error || row.status}`;
      if (row.pending_model) return `SINYAL ${row.pending_model.direction===1?'BUY':'SELL'} TERCATAT · model next-open akan direkonstruksi saat H4 berikutnya selesai; belum merupakan fill live.`;
      if (row.position_model_at_last_close) return `HOLD ${row.position_model_at_last_close.direction===1?'LONG':'SHORT'} · posisi model pada close terakhir; menunggu evaluasi kanal/stop H4 berikutnya.`;
      return 'OBSERVASI AKTIF · belum ada posisi. Menunggu candle H4 selesai di luar batas kanal buy/sell. Harga menyentuh garis saja belum menjadi sinyal.';
    },
    number(n,digits=2) { return typeof n==='number' && Number.isFinite(n) ? n.toLocaleString('en-US',{minimumFractionDigits:digits,maximumFractionDigits:digits}) : '—'; }
  };
  if(typeof module!=='undefined' && module.exports) module.exports=api;
  root.ObserverModel=api;
})(typeof globalThis!=='undefined'?globalThis:this);
