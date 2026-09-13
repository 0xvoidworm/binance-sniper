'use strict';
window.LabPanel=(()=>{
  let source='RESULTS',variant='SWEEP_T1_4',lastPayload,lastSource,lastVariant,lastStatus;
  const eventCache=new Map();
  const node=(tag,text,cls)=>{const e=document.createElement(tag);if(text!==undefined)e.textContent=String(text);if(cls)e.className=cls;return e};
  const val=v=>typeof v==='number'&&Number.isFinite(v)?v:null;
  const fmt=(v,s='')=>val(v)===null?'—':v.toFixed(3)+s;
  const date=v=>val(v)===null?'—':new Date(v).toISOString().replace('T',' ').slice(0,16)+' UTC';
  const exitLabel=o=>o?.exit_reason==='FOUR_BAR_CLOSE'?`TIME STOP / ${o.costs.horizon} M5`:o?.exit_reason||o?.status;
  const table=(headers,rows)=>{const t=node('table'),thead=node('thead'),tr=node('tr'),tbody=node('tbody');tr.append(...headers.map(h=>node('th',h)));thead.append(tr);for(const values of rows){const row=node('tr');for(const value of values){const td=node('td');td.append(value instanceof Node?value:document.createTextNode(String(value??'—')));row.append(td)}tbody.append(row)}t.append(thead,tbody);return t};
  function render(host,payload){
    const status=!navigator.onLine?'OFFLINE':payload?.status==='EXPERIMENTAL_MONITOR'&&Date.now()-(payload.updated_at||0)>45000?'STALE':payload?.status||'STARTING';
    if(host.firstElementChild?.classList.contains('lab-toolbar')&&lastPayload===payload&&lastSource===source&&lastVariant===variant&&lastStatus===status)return;
    lastPayload=payload;lastSource=source;lastVariant=variant;lastStatus=status;
    const scroll=host.scrollTop,expanded=host.querySelector('details')?.open;
    const tools=node('div',undefined,'research-toolbar lab-toolbar');
    for(const [s,label] of [['RESULTS','HASIL UJI'],['SHADOW','OBSERVASI BARU']]){const b=node('button',label,s===source?'selected':'');b.onclick=()=>{source=s;render(host,payload)};tools.append(b)}
    tools.append(node('span',status,'muted'));host.replaceChildren(tools);
    if(source==='SHADOW'){renderShadow(host,payload);host.scrollTop=scroll;return}
    const report=payload?.report;
    if(!report){host.append(node('p','Studi belum selesai. Hasil tidak diganti angka contoh.','research-note'));return}
    const v=report.validation,selected=v.selection.candidate,h=v.holdout,b=report.baseline.holdout;
    const intro=node('div',undefined,'research-summary');
    intro.append(node('strong',v.promotion==='NO_PROMOTION'?'BELUM LOLOS VALIDASI':'LAYAK OBSERVASI LANJUTAN'),node('p',`Pilihan training: ${selected||'tidak ada'} · 8 variasi + baseline · 360 hari / 6 pair`));
    intro.append(node('p',`${date(report.start)} → ${date(report.end)} · 3 walk-forward + 60 hari holdout`,'muted'));
    intro.append(node('p','Chart utama tetap BASELINE. Area delapan hipotesis terpisah ada di bukti historis dan OBSERVASI BARU.','muted'));
    host.append(intro);
    const rows=[['BASELINE / HOLDOUT',b.base.n,fmt(b.base.mean_r,'R'),fmt(b.stress.mean_r,'R'),fmt(b.lower95_net_r,'R')],
      [selected+' / HOLDOUT',h?.base.n??0,fmt(h?.base.mean_r,'R'),fmt(h?.stress.mean_r,'R'),fmt(h?.lower95_net_r,'R')],
      ['PILIHAN BERGULIR / WALK-FORWARD',v.walk_forward.n,fmt(v.walk_forward.mean_r,'R'),'—','—']];
    host.append(table(['PENGUJIAN','N','MEAN NET R','STRESS R','BATAS BAWAH 95%'],rows));
    host.append(node('p','R adalah unit jarak risiko per event, bukan persen modal. Lebih banyak sinyal dan rugi per event yang lebih kecil belum berarti untung.','research-note amber'));
    const folds=v.folds.map((f,i)=>[i+1,f.selection.candidate,date(f.test?.start),f.test?.base.n??0,fmt(f.test?.base.mean_r,'R')]);
    host.append(table(['FOLD','DIPILIH SEBELUM TEST','MULAI TEST','N','MEAN NET R'],folds));
    const details=node('details');details.open=Boolean(expanded);details.append(node('summary','Semua variasi, alasan gagal, dan batas studi'));
    details.append(node('p','Tabel seluruh periode berikut adalah diagnostik setelah uji, bukan dasar pemilihan holdout.','research-note'));
    details.append(table(['VARIASI','HORIZON M5','N / 360 HARI','MEAN NET R','STRESS R'],report.variants.map(x=>[x.variant.name,x.variant.horizon,x.full_posthoc.base.n,fmt(x.full_posthoc.base.mean_r,'R'),fmt(x.full_posthoc.stress.mean_r,'R')])));
    for(const reason of v.reasons)details.append(node('p',reason,'research-note down'));
    for(const limit of report.limitations)details.append(node('p',limit,'research-note muted'));
    details.append(node('p','Trial: '+report.trial,'research-note muted'));host.append(details);
    const button=node('button','Lihat observasi area sekarang');button.className='lab-action';button.onclick=()=>{source='SHADOW';render(host,payload)};host.append(button);
    const chooser=node('select');chooser.setAttribute('aria-label','Pilih bukti historis');for(const x of report.variants){const option=node('option',x.variant.name);option.value=x.variant.name;chooser.append(option)}chooser.value=variant;chooser.onchange=()=>{variant=chooser.value;render(host,payload)};
    const ledger=node('div'),label=node('label','Bukti historis ');label.append(chooser);label.className='research-note';host.append(label,ledger);loadEvents(ledger,report.trial,variant);
    host.scrollTop=scroll;
  }
  async function loadEvents(host,trial,name){
    const key=trial+':'+name;host.append(node('p','Memuat sinyal tersimpan…','research-note'));
    try{let data=eventCache.get(key);if(!data){const response=await fetch('/api/lab/events?'+new URLSearchParams({trial,variant:name}),{cache:'no-store',signal:AbortSignal.timeout(5000)});if(!response.ok)throw new Error('HTTP '+response.status);data=await response.json();eventCache.set(key,data)}if(!host.isConnected)return;host.replaceChildren(table(['SINYAL / UTC','PAIR','ARAH','EXIT','NET R','BUKTI'],data.events.map(e=>{const b=node('button','Lihat candle & area');b.onclick=()=>ResearchPanel.inspect(data.run_id,e.id);return [date(e.signal.decision_time),e.signal.symbol,e.signal.direction,exitLabel(e.outcome?.base),fmt(e.outcome?.base.net_r,'R'),b]})));if(!data.events.length)host.append(node('p','Tidak ada sinyal pada variasi ini.','research-note'));}catch(e){host.replaceChildren(node('p','Bukti belum tersedia: '+e.message,'research-note down'))}
  }
  function renderShadow(host,payload){
    const shadow=payload?.shadow||{},controls=node('div',undefined,'research-summary'),label=node('label','Hipotesis '),select=node('select');select.setAttribute('aria-label','Pilih hipotesis eksperimen');
    for(const v of shadow.variants||[]){const option=node('option',v.name+' / '+v.horizon+' M5');option.value=v.name;select.append(option)}
    select.value=variant;select.onchange=()=>{variant=select.value;render(host,payload)};label.append(select);controls.append(label,node('p',payload?.note||'Hipotesis eksperimental; tidak ada order.','amber'));
    host.append(controls);
    host.append(node('p','Chart utama = BASELINE. Gunakan Lihat area di bawah untuk membaca hipotesis yang dipilih.','research-note muted'));
    const states=shadow.states?.[variant]||{};
    const symbols=payload?.report?.symbols||Object.keys(states);
    host.append(table(['PAIR','TAHAP','AREA','DIPERIKSA / UTC','BUKTI'],symbols.map(symbol=>{const s=states[symbol]||{},a=s.area,b=node('button','Lihat area');b.onclick=()=>showArea(variant,symbol);return [symbol,s.stage||'WAIT_NEW_M5',a?`${a.lower.toPrecision(6)} – ${a.upper.toPrecision(6)}`:'—',date(s.at),b]})));
    host.append(node('p',`Observasi mulai ${date(shadow.started_at)} · pending outcome ${shadow.pending??0}. Funding live belum direkonsiliasi; hasil live hanya setelah fee/slippage.`,'research-note'));
    host.append(table(['SINYAL / UTC','HIPOTESIS','PAIR','SETELAH FEE/SLIP','BUKTI'],(shadow.recent||[]).filter(e=>e.signal.variant===variant).map(e=>{const b=node('button','Lihat sinyal');b.onclick=()=>ResearchPanel.inspect(e.run_id,e.signal.candidate_id);return [date(e.signal.decision_time),e.signal.variant,e.signal.symbol,fmt(e.outcome?.base.after_fee_slippage_r,'R'),b]})));
  }
  async function showArea(variant,symbol){
    const dialog=node('dialog',undefined,'research-dialog'),close=node('button','Tutup ×');close.onclick=()=>dialog.close();dialog.onclose=()=>dialog.remove();dialog.append(close,node('h3',variant+' / '+symbol));document.body.append(dialog);dialog.showModal();
    try{const response=await fetch('/api/lab/chart?'+new URLSearchParams({variant,symbol}),{cache:'no-store',signal:AbortSignal.timeout(5000)});if(!response.ok)throw new Error('HTTP '+response.status);const data=await response.json();if(!dialog.open)return;dialog.append(node('p',data.note,'research-note amber'),areaChart(data),node('p',`${data.state.stage||'WAIT_NEW_M5'} · ${date(data.at)}`,'research-note'));}catch(e){dialog.append(node('p','Area belum tersedia: '+e.message,'research-note'))}
  }
  function areaChart(data){
    const bars=data.bars,a=data.state.area;if(!bars.length)return node('p','Menunggu candle baru.');
    const ns='http://www.w3.org/2000/svg',svg=document.createElementNS(ns,'svg');svg.setAttribute('viewBox','0 0 1000 310');svg.setAttribute('role','img');svg.setAttribute('aria-label','Candle observasi dengan area eksperimen yang benar-benar tercatat');
    const visible=a&&a.expires_at>=bars[0].open_time;
    let lo=Math.min(...bars.map(b=>b.low),...(visible?[a.lower,a.invalidation]:[])),hi=Math.max(...bars.map(b=>b.high),...(visible?[a.upper,a.invalidation]:[]));const pad=(hi-lo)*.08||1;lo-=pad;hi+=pad;
    const x=t=>20+(t-bars[0].open_time)/(bars.at(-1).close_time-bars[0].open_time+300000)*850,y=p=>20+(hi-p)/(hi-lo)*230;
    const shape=(tag,attrs,text)=>{const n=document.createElementNS(ns,tag);for(const [k,v] of Object.entries(attrs))n.setAttribute(k,v);if(text)n.textContent=text;svg.append(n)};
    for(let i=0;i<5;i++){const p=hi-(hi-lo)*i/4;shape('line',{x1:20,x2:880,y1:y(p),y2:y(p),stroke:'#26343f'});shape('text',{x:885,y:y(p)+4,fill:'#8695a4','font-size':12},p.toPrecision(7))}
    if(visible){const left=Math.max(20,x(a.created_at)),right=Math.min(880,x(a.expires_at));shape('rect',{x:left,y:y(a.upper),width:Math.max(1,right-left),height:Math.max(1,y(a.lower)-y(a.upper)),stroke:'#ffb23e',fill:'#ffb23e22','stroke-dasharray':'4 3'});shape('text',{x:25,y:y(a.upper)-6,fill:'#ffb23e','font-size':12},a.state+' / AREA');shape('line',{x1:left,x2:880,y1:y(a.invalidation),y2:y(a.invalidation),stroke:'#f47785','stroke-dasharray':'4 3'})}
    for(const b of bars){const xx=x(b.open_time+150000),color=b.close>=b.open?'#50d6a0':'#f47785';shape('line',{x1:xx,x2:xx,y1:y(b.high),y2:y(b.low),stroke:color});shape('rect',{x:xx-3,y:Math.min(y(b.open),y(b.close)),width:6,height:Math.max(1,Math.abs(y(b.open)-y(b.close))),fill:color})}
    shape('text',{x:20,y:285,fill:'#8695a4','font-size':12},date(bars[0].open_time));shape('text',{x:620,y:285,fill:'#8695a4','font-size':12},date(bars.at(-1).close_time));return svg;
  }
  return {render};
})();
