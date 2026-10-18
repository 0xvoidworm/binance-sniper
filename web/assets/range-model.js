(function(root){
  'use strict';
  const labels={WAIT_SESSION:'Tunggu sesi',BUILDING_RANGE:'Membentuk range',ARMED:'Menunggu breakout',HOLDING:'Pertahankan posisi',CLOSED:'Posisi selesai',NO_BREAKOUT:'Tidak ada breakout',AMBIGUOUS:'Arah pertama tidak pasti',DATA_GAP:'Data belum lengkap',INVALID_RANGE:'Range tidak valid'};
  const reasons={WAIT_RANGE_START:'Range mulai 10:00 WIB. Belum ada entry yang dicari.',WAIT_RANGE_COMPLETE:'High dan low masih dibentuk. Entry menunggu range lengkap.',WAIT_FIRST_BREAKOUT:'Range sudah terkunci. Tunggu sentuhan pertama pada salah satu batas.',HOLD_UNTIL_STOP_OR_SESSION_END:'Stop belum tersentuh. Posisi model dipertahankan sampai stop atau akhir sesi; tanpa TP tetap.',STOP:'Posisi model berakhir karena stop di sisi lain range.',STOP_GAP:'Harga membuka melewati stop. Hasil memakai harga gap, bukan harga stop ideal.',SESSION_END:'Posisi model ditutup pada akhir sesi, 18:00 UTC / 01:00 WIB.',SESSION_FINISHED_NO_ENTRY:'Sesi berakhir tanpa breakout. Tidak ada posisi model.',BOTH_BREAKOUTS_SAME_BAR:'Kedua batas tersentuh dalam satu M5. Urutan tidak diketahui; tidak mengarang entry atau profit.',RANGE_DATA_INCOMPLETE:'Candle pembentuk range belum lengkap. Tidak menyimpulkan entry.',EXECUTION_DATA_INCOMPLETE:'Ada candle hilang setelah range. Hasil posisi belum dapat dipastikan.',INVALID_OR_REVISED_DATA:'Data candle tidak valid atau memiliki revisi yang bertentangan.',ZERO_WIDTH_RANGE:'Range tidak memiliki jarak harga yang valid.'};
  function number(v){return v===null||v===undefined||v===''?null:Number.isFinite(Number(v))?Number(v):null}
  function stateLabel(s){return labels[s]||s||'Menunggu data'}
  function explanation(s){return reasons[s?.reason]||'Menunggu snapshot mesin.'}
  function series(data,symbol,tf){const s=data?.charts?.[symbol+':'+tf]||{};const rows=[...(s.closed||[])];if(s.preview&&!rows.some(b=>b.open_time===s.preview.open_time))rows.push(s.preview);return rows.sort((a,b)=>a.open_time-b.open_time)}
  function fresh(session,elapsed=0){return number(session?.data_age_ms)!==null&&session.data_age_ms+elapsed<20000}
  function stageIndex(s){return s==='BUILDING_RANGE'?0:s==='ARMED'?1:s==='HOLDING'?2:['CLOSED','NO_BREAKOUT','AMBIGUOUS'].includes(s)?3:-1}
  function positionValue(s){if(s?.status==='CLOSED')return {value:number(s.net_pct)??number(s.after_cost_pct),label:number(s.net_pct)!==null?'Net model / termasuk funding':'Setelah fee + slippage / belum funding'};return {value:number(s?.unrealized_price_pct),label:'Gerak harga posisi model / sebelum biaya'}}
  const api={number,stateLabel,explanation,series,fresh,stageIndex,positionValue};
  if(typeof module!=='undefined'&&module.exports)module.exports=api;
  root.RangeModel=api;
})(typeof globalThis!=='undefined'?globalThis:this);
