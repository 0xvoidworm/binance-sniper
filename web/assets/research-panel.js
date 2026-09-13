'use strict';
window.ResearchPanel=(()=>{
  let mode='HISTORICAL',dialog,lastPayload,lastMode,lastStatus;
  const node=(tag,text,cls)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=String(text);if(cls)e.className=cls;return e};
  const number=v=>typeof v==='number'&&Number.isFinite(v)?v:null;
  const fmt=(v,suffix='')=>number(v)===null?'—':v.toFixed(3)+suffix;
  const price=v=>number(v)===null?'—':Number(v.toPrecision(9)).toString();
  const date=v=>number(v)===null?'—':new Date(v).toISOString().replace('T',' ').slice(0,16)+' UTC';
  function table(headers,rows){const t=node('table'),head=node('thead'),tr=node('tr'),body=node('tbody');tr.append(...headers.map(x=>node('th',x)));head.append(tr);for(const values of rows){const row=node('tr');for(const value of values){const cell=node('td');cell.append(value instanceof Node?value:document.createTextNode(String(value??'—')));row.append(cell)}body.append(row)}t.append(head,body);return t}
  function render(host,payload){
    const status=!navigator.onLine?'OFFLINE':payload?.status==='CAPTURING'&&Date.now()-(payload.updated_at||0)>65000?'STALE':payload?.status||'STARTING';
    if(host.firstElementChild?.classList.contains('phase2-toolbar')&&lastPayload===payload&&lastMode===mode&&lastStatus===status)return;
    lastPayload=payload;lastMode=mode;lastStatus=status;
    const expanded=host.querySelector('details')?.open,scroll=host.scrollTop;
    const tabs=node('div',undefined,'research-toolbar phase2-toolbar');
    for(const [value,label] of [['HISTORICAL','HISTORICAL'],['LIVE','LIVE JOURNAL']]){const b=node('button',label,mode===value?'selected':'');b.onclick=()=>{mode=value;render(host,payload)};tabs.append(b)}
    tabs.append(node('span','Recorder: '+status,'muted'));
    const entry=payload?.reports?.find(r=>r.mode===mode),r=entry?.report;
    host.replaceChildren(tabs);
    if(!r){host.append(node('p','Laporan belum tersedia untuk sumber ini. Riwayat tidak digabung dengan observasi live.','research-note'));return}
    const head=node('div',undefined,'research-summary');
    head.append(node('strong',`${mode==='HISTORICAL'?'PUBLIC MARKET EVENT STUDY':'OBSERVASI SINYAL LIVE'} · ${r.signal_count} sinyal`));
    head.append(node('p',`${date(r.start)} → ${date(r.end)} · ${r.decisions.toLocaleString('en-US')} evaluasi M5 · ${r.symbols.length} pair`));
    const costs=r.specification.costs,stress=r.specification.stress_costs;
    head.append(node('p',`NEXT OPEN / ${costs.horizon} BAR MAX · fee ${costs.fee_bps} bps + slip ${costs.slippage_bps} bps per sisi · stress ${stress.fee_bps} + ${stress.slippage_bps} bps`,'muted'));
    if(status!=='CAPTURING')head.append(node('p','Recorder tidak segar/aktif. Laporan yang tersimpan tetap dapat dibaca.','down'));
    host.append(head);
    const rows=r.groups.map(g=>{const b=mode==='LIVE'?g.before_funding:g.base;return [g.partition,g.signals,b.n,fmt(b.mean_r,'R'),number(b.win_rate)===null?'—':(b.win_rate*100).toFixed(1)+'%',fmt(b.profit_factor),fmt(g.stress.mean_r,'R'),b.ambiguous,b.sample_warning]});
    host.append(table(['PERIODE','SINYAL','N SELESAI',mode==='LIVE'?'MEAN R / FUNDING BELUM':'MEAN NET R','WIN RATE','PROFIT FACTOR','STRESS NET R','AMBIGU','KUALITAS SAMPEL'],rows));
    if(mode==='LIVE')host.append(node('p',r.funding_note+' '+r.execution_note,'research-note amber'));
    else host.append(node('p',`Split ${date(r.split)} · purge/embargo ${costs.horizon} candle di batas · optimasi parameter: TIDAK · dikecualikan: ${Object.entries(r.excluded).map(([k,v])=>k+' '+v).join(', ')||'0'}`,'research-note'));
    const details=node('details'),summary=node('summary','Gate, pair, dan asumsi');details.append(summary);
    details.open=Boolean(expanded);
    details.append(node('p','Gate: observasi per candle, bukan trade yang gagal. Zona lama dapat tetap dipantau; lihat stage dan snapshot gate pada dataset.','research-note'));
    details.append(table(['GATE PERTAMA TIDAK LOLOS','OBSERVASI'],r.gate_counts.map(g=>[g.reason,g.count.toLocaleString('en-US')])));
    details.append(table(['PAIR / SEMUA PERIODE','N NET','MEAN NET R','KUALITAS'],r.by_symbol.map(s=>[s.symbol,s.n,fmt(s.mean_r,'R'),s.sample_warning])));
    for(const limit of r.limits)details.append(node('p','• '+limit,'research-note'));
    details.append(node('p',`Strategy ${r.strategy_hash} · dataset ${r.dataset} · dibuat ${date(r.generated_at)}`,'research-note muted'));
    host.append(details,node('p',r.interpretation,'research-note amber'));
    host.append(node('h3','EVENT LEDGER / 50 TERAKHIR','research-ledger-title'));
    host.append(table(['SINYAL / UTC','PAIR','ARAH','PERIODE','EXIT MODEL','NET R','FUNDING','BUKTI'],r.recent_outcomes.map(o=>{
      const b=o.base,button=node('button','Lihat candle');button.onclick=()=>inspect(r.run_id,b.candidate_id);
      return [date(b.decision_time),b.symbol,b.direction,o.partition,b.exit_reason==='FOUR_BAR_CLOSE'?`TIME STOP / ${b.costs.horizon} M5`:b.exit_reason||b.status,fmt(b.net_r,'R'),b.funding_status,button];
    })));
    if(!r.recent_outcomes.length)host.append(node('p','Belum ada outcome. Kosong bukan berarti profit nol atau strategi terbukti.','research-note'));
    host.scrollTop=scroll;
  }
  async function inspect(run,id){
    if(dialog)dialog.remove();const active=node('dialog',undefined,'research-dialog');dialog=active;
    const close=node('button','Tutup ×');close.onclick=()=>active.close();active.append(close,node('h3','MEMUAT BUKTI CANDLE'));document.body.append(active);active.showModal();
    try{
      const response=await fetch('/api/research/event?'+new URLSearchParams({run_id:run,candidate_id:id}),{cache:'no-store',signal:AbortSignal.timeout(5000)});
      if(!response.ok)throw new Error('HTTP '+response.status);
      const data=await response.json();if(!active.open||active!==dialog)return;
      active.replaceChildren(close,node('h3',data.signal.symbol+' / '+data.signal.direction+' / '+date(data.signal.decision_time)));
      active.append(node('p',data.note,'research-note amber'),chart(data));
      const b=data.outcome?.base;
      const exitLabel=b?.exit_reason==='FOUR_BAR_CLOSE'?`TIME STOP / ${b.costs.horizon} M5`:b?.exit_reason||b?.status||'—';
      active.append(node('p',`Sinyal close ${price(data.signal.entry_reference)} → modeled entry ${price(b?.entry)} → exit ${price(b?.exit)} · ${exitLabel} · net ${fmt(b?.net_r,'R')}`,'research-note'));
      active.append(node('p','Level dan area berasal dari sinyal tersimpan; area hanya digambar jika datanya tersedia. Area biru adalah candle setelah keputusan.','research-note muted'));
    }catch(error){active.append(node('p','Bukti gagal dimuat: '+error.message,'research-note down'))}
  }
  function chart(data){
    const ns='http://www.w3.org/2000/svg',svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 1000 300');svg.setAttribute('role','img');svg.setAttribute('aria-label','Candle M5 historis dengan entry, stop dan target referensi');
    const bars=data.bars,s=data.signal;if(!bars.length)return node('p','Candle arsip tidak tersedia.');
    let lo=Math.min(...bars.map(b=>b.low),s.stop_reference,s.target_reference),hi=Math.max(...bars.map(b=>b.high),s.stop_reference,s.target_reference);const pad=(hi-lo)*.08||1;lo-=pad;hi+=pad;
    const x=t=>20+(t-bars[0].open_time)/(bars.at(-1).close_time-bars[0].open_time+300000)*850,y=p=>20+(hi-p)/(hi-lo)*230;
    const shape=(tag,attrs,text)=>{const e=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attrs))e.setAttribute(k,v);if(text)e.textContent=text;svg.append(e);return e};
    shape('rect',{x:x(s.decision_time+1),y:20,width:Math.max(0,870-x(s.decision_time+1)),height:230,fill:'#72b9dc18'});
    if(s.area){const a=s.area,left=Math.max(20,x(a.created_at)),right=Math.min(880,x(s.decision_time));shape('rect',{x:left,y:y(a.upper),width:Math.max(1,right-left),height:Math.max(1,y(a.lower)-y(a.upper)),fill:'#ffb23e22',stroke:'#ffb23e','stroke-dasharray':'4 3'})}
    for(let i=0;i<=4;i++){const p=hi-(hi-lo)*i/4;shape('line',{x1:20,x2:880,y1:y(p),y2:y(p),stroke:'#26343f'});shape('text',{x:885,y:y(p)+4,fill:'#8695a4','font-size':12},p.toPrecision(7))}
    for(const b of bars){const xx=x(b.open_time+150000),color=b.close>=b.open?'#50d6a0':'#f47785';shape('line',{x1:xx,x2:xx,y1:y(b.high),y2:y(b.low),stroke:color});shape('rect',{x:xx-4,y:Math.min(y(b.open),y(b.close)),width:8,height:Math.max(1,Math.abs(y(b.open)-y(b.close))),fill:color})}
    for(const [label,p,color] of [['ENTRY REF',s.entry_reference,'#72b9dc'],['STOP REF',s.stop_reference,'#f47785'],['TARGET REF',s.target_reference,'#50d6a0']]){shape('line',{x1:x(s.decision_time),x2:880,y1:y(p),y2:y(p),stroke:color,'stroke-dasharray':'4 3'});shape('text',{x:20,y:y(p)-5,fill:color,'font-size':12},label)}
    shape('line',{x1:x(s.decision_time),x2:x(s.decision_time),y1:20,y2:260,stroke:'#ffb23e','stroke-dasharray':'3 3'});
    shape('text',{x:20,y:285,fill:'#8695a4','font-size':12},date(bars[0].open_time));shape('text',{x:630,y:285,fill:'#8695a4','font-size':12},date(bars.at(-1).close_time));
    return svg;
  }
  return {render,inspect};
})();
