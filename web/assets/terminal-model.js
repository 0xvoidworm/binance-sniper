(function(root){
  'use strict';
  const terminalStates=new Set(['COMPLETED','EXPIRED','INVALIDATED']);
  const intervals={'5m':300000,'15m':900000,'1h':3600000,'4h':14400000};
  function number(v){return v===null||v===undefined||v===''?null:(Number.isFinite(Number(v))?Number(v):null)}
  function symbolFor(data,preferred){const keys=Object.keys(data.symbols||{});return keys.includes(preferred)?preferred:(keys[0]||'')}
  function selectedSetup(data,symbol,candidateId=null){
    const original=data.symbols?.[symbol]||{};
    const explicit=candidateId?(data.candidates||[]).find(c=>c.candidate_id===candidateId&&c.symbol===symbol):null;
    const zone=explicit?data.zones?.[explicit.zone_id]:original.zone;
    const candidate=(data.candidates||[]).filter(c=>c.symbol===symbol&&zone&&c.zone_id===zone.zone_id&&(!candidateId||c.candidate_id===candidateId)).sort((a,b)=>b.decision_time-a.decision_time)[0]||null;
    const outcome=candidate?data.outcomes?.[candidate.candidate_id]||null:null;
    const isArchive=Boolean(zone&&terminalStates.has(zone.state));
    const matches=zone&&original.zone?.zone_id===zone.zone_id;
    const impulse=matches?original.impulse:(data.events||[]).find(e=>e.impulse?.id===zone?.source_impulse)?.impulse;
    return {symbol:original,zone:zone||null,candidate,outcome,impulse:impulse&&(!zone||impulse.id===zone.source_impulse)?impulse:null,trigger:candidate?{time:candidate.decision_time,price:candidate.entry_reference}:matches?original.trigger:null,decision:!explicit||matches?original.decision:null,isArchive};
  }
  function connectionView(data,receivedAt,now,online=true,channels={}){
    const elapsed=receivedAt===null?Infinity:Math.max(0,now-receivedAt);
    if(!online||(!channels.wsOpen&&channels.pollFailed))return {label:'DISCONNECTED',tone:'down',elapsed};
    if(receivedAt===null)return {label:'CONNECTING',tone:'amber',elapsed};
    if(elapsed>8000)return {label:'STALE',tone:'down',elapsed};
    const transport=data.transport_state||'STARTING';
    return {label:transport==='LIVE_REST_FALLBACK'?'LIVE / REST':transport==='LIVE_WS'?'LIVE / WS':transport,tone:transport.startsWith('LIVE')?'up':transport==='DEGRADED'?'amber':'down',elapsed};
  }
  function feedAge(q,elapsed,key='success_age_ms'){const age=number(q?.[key]);return age===null?null:age+Math.max(0,elapsed)}
  function lastPrice(data,symbol){const series=data.charts?.[symbol+':5m'];return series?.preview?.close??series?.closed?.at(-1)?.close??null}
  function zoneDistance(last,zone){if(number(last)===null||!zone)return null;const distance=last<zone.lower?zone.lower-last:last>zone.upper?last-zone.upper:0;return {price:distance,percent:distance/last*100}}
  function eventName(code){return ({REGIME:'Arah H4 terkonfirmasi',IMPULSE:'Impulse H1 terkonfirmasi',DETECTED:'Area pullback terdeteksi',ARMED:'Menunggu harga di area',TOUCHED:'Harga menyentuh area',TRIGGERED:'Pemicu M5 terkonfirmasi',CANDIDATE:'Sinyal observasi tercatat',COMPLETED:'Observasi empat candle selesai',OUTCOME:'Hasil observasi tersedia',INVALIDATED:'Setup gugur',EXPIRED:'Area kedaluwarsa'})[code]||code||'—'}
  const model={number,symbolFor,selectedSetup,connectionView,feedAge,lastPrice,zoneDistance,eventName,terminalStates,intervals};
  if(typeof module!=='undefined'&&module.exports)module.exports=model;
  root.TerminalModel=model;
})(typeof globalThis!=='undefined'?globalThis:this);
