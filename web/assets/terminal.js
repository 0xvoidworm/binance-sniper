'use strict';
const M=TerminalModel,$=id=>document.getElementById(id);
const ui={mode:'LIVE',symbol:'BTCUSDT',tf:'5m',tab:'journal',candidateId:null,live:{},data:{},replay:null,frame:0,receivedAt:null,receivedStamp:null,account:null,accountAt:null,pollFailed:false,wsOpen:false,viewCount:65,offset:0,crosshair:true,pointer:null,drag:null,playing:null};
let socket,retry=1000,polling=false,accountPolling=false,replayLoading=false,drawQueued=false;
let researchData=null,researchPolling=false;
let labData=null,labPolling=false;
async function loadLab(){if(labPolling)return;labPolling=true;try{labData=await json('/api/lab')}catch{labData={...(labData||{}),status:'UNAVAILABLE'}}finally{labPolling=false;if(ui.tab==='lab')renderTerminal()}}
async function loadResearch(){if(researchPolling)return;researchPolling=true;try{researchData=await json('/api/research')}catch{researchData={...(researchData||{}),status:'UNAVAILABLE'}}finally{researchPolling=false;if(ui.tab==='research')renderTerminal()}}
const tfName={'5m':'M5','15m':'M15','1h':'H1','4h':'H4'},stageName={WAIT_H4:'Tunggu H4',WAIT_H1:'Tunggu H1',WAIT_M15:'Tunggu M15',ARMED:'Tunggu area',TOUCHED:'Tunggu trigger',TRIGGERED:'Tracking',COMPLETED:'Selesai',INVALIDATED:'Gugur',EXPIRED:'Kedaluwarsa',DETECTED:'Area terdeteksi'};
const dateFormat=new Intl.DateTimeFormat('id-ID',{timeZone:'Asia/Jakarta',day:'2-digit',month:'short',hour:'2-digit',minute:'2-digit',hourCycle:'h23'});
const timeFormat=new Intl.DateTimeFormat('id-ID',{timeZone:'Asia/Jakarta',hour:'2-digit',minute:'2-digit',second:'2-digit',hourCycle:'h23'});
const time=v=>M.number(v)===null?'—':dateFormat.format(new Date(v));
const age=v=>!Number.isFinite(v)?'—':v<1000?`${Math.round(v)}ms`:v<60000?`${(v/1000).toFixed(1)}s`:`${Math.floor(v/60000)}m ${Math.floor(v/1000%60)}s`;
function price(v,symbol=ui.symbol){const n=M.number(v);if(n===null)return '—';const tick=ui.data.instrument_meta?.[symbol]?.tick_size;const digits=tick?Math.min(8,(String(tick).replace(/0+$/,'').split('.')[1]||'').length):n>=10000?1:n>=100?2:n>=1?3:5;return n.toLocaleString('en-US',{minimumFractionDigits:digits,maximumFractionDigits:digits})}
const rvalue=v=>M.number(v)===null?'—':Number(v).toFixed(2)+'R';
function el(tag,value,className){const n=document.createElement(tag);if(value!==undefined&&value!==null)n.textContent=String(value);if(className)n.className=className;return n}
function put(id,value){$(id).textContent=String(value??'—')}
function resetView(){ui.offset=0;ui.viewCount=65;ui.pointer=null;queueDraw()}
function chooseSymbol(symbol){ui.symbol=M.symbolFor(ui.data,symbol);ui.candidateId=null;resetView();render()}
function chooseTf(tf){ui.tf=tf;resetView();render()}
function receive(data){
  if(!data||typeof data.symbols!=='object'||!data.charts||!Number.isFinite(data.snapshot_at))return;
  // A repeated or older snapshot must never reset the freshness clock.
  if(ui.receivedStamp!==null&&data.snapshot_at<ui.receivedStamp)return;
  if(ui.receivedStamp!==data.snapshot_at){ui.receivedAt=performance.now();ui.receivedStamp=data.snapshot_at}
  ui.live=data;
  if(ui.mode==='LIVE'){ui.data=data;ui.symbol=M.symbolFor(data,ui.symbol);render()}
}
async function json(url){const response=await fetch(url,{cache:'no-store',signal:AbortSignal.timeout(5000)});if(!response.ok)throw new Error('HTTP '+response.status);return response.json()}
async function poll(){if(polling)return;polling=true;try{const data=await json('/api/state');ui.pollFailed=false;receive(data)}catch{ui.pollFailed=true;renderStatus()}finally{polling=false}}
async function loadAccount(){if(accountPolling)return;accountPolling=true;try{ui.account=await json('/api/account');ui.accountAt=performance.now()}catch{if(ui.account)ui.account={...ui.account,auth:'STALE'}}finally{accountPolling=false;if(ui.tab==='account')renderTerminal()}}
function connect(){
  socket=new WebSocket(`${location.protocol==='https:'?'wss':'ws'}://${location.host}/ws`);
  socket.onopen=()=>{ui.wsOpen=true;retry=1000;renderStatus()};
  socket.onmessage=event=>{try{receive(JSON.parse(event.data))}catch{/* Ignore malformed browser messages without refreshing the data clock. */}};
  socket.onerror=()=>socket.close();
  socket.onclose=()=>{ui.wsOpen=false;renderStatus();setTimeout(connect,retry+Math.random()*Math.min(1000,retry*.2));retry=Math.min(30000,retry*2)};
}
function stopReplay(){if(ui.playing)clearInterval(ui.playing);ui.playing=null;put('replayPlay','Putar')}
function setFrame(index){const frames=ui.replay?.frames||[];if(!frames.length)return;ui.frame=Math.max(0,Math.min(frames.length-1,index));ui.data=frames[ui.frame];ui.symbol=M.symbolFor(ui.data,ui.symbol);ui.candidateId=null;ui.pointer=null;$('replayRange').value=String(ui.frame);put('replayProgress',`${ui.frame+1}/${frames.length} · ${time(ui.data.snapshot_at)} WIB`);render();if(ui.frame===frames.length-1)stopReplay()}
$('liveButton').onclick=()=>{stopReplay();ui.mode='LIVE';ui.data=ui.live;ui.symbol=M.symbolFor(ui.data,ui.symbol);ui.candidateId=null;resetView();render()};
$('replayButton').onclick=async()=>{if(replayLoading)return;replayLoading=true;try{ui.replay=ui.replay||await json('/api/replay');stopReplay();ui.mode='REPLAY';ui.symbol=M.symbolFor(ui.replay,ui.symbol);$('replayRange').max=String(ui.replay.frames.length-1);resetView();setFrame(0)}catch{put('footerMessage','Replay gagal dimuat. Klik Replay untuk mencoba lagi.')}finally{replayLoading=false}};
$('replayRange').oninput=e=>{stopReplay();setFrame(Number(e.target.value))};
$('replayStep').onclick=()=>{stopReplay();setFrame(ui.frame+1)};
$('replayPlay').onclick=()=>{if(ui.playing){stopReplay();return}if(ui.frame===ui.replay.frames.length-1)setFrame(0);put('replayPlay','Jeda');ui.playing=setInterval(()=>setFrame(ui.frame+1),1200)};
$('symbol').onchange=e=>chooseSymbol(e.target.value);
document.querySelectorAll('[data-tf]').forEach(button=>button.onclick=()=>chooseTf(button.dataset.tf));
$('crosshair').onclick=()=>{ui.crosshair=!ui.crosshair;$('crosshair').setAttribute('aria-pressed',String(ui.crosshair));ui.pointer=null;queueDraw()};
$('zoomIn').onclick=()=>{ui.viewCount=Math.max(16,ui.viewCount-10);queueDraw()};
$('zoomOut').onclick=()=>{ui.viewCount=Math.min(80,ui.viewCount+10);queueDraw()};
$('resetChart').onclick=()=>{ui.candidateId=null;resetView();render()};
document.querySelectorAll('[data-tab]').forEach(button=>button.onclick=()=>{ui.tab=button.dataset.tab;renderTerminal()});
window.addEventListener('online',()=>{poll();renderStatus()});window.addEventListener('offline',renderStatus);

function health(){return M.connectionView(ui.live,ui.receivedAt,performance.now(),navigator.onLine,{wsOpen:ui.wsOpen,pollFailed:ui.pollFailed})}
function elapsed(){return ui.mode==='REPLAY'?0:health().elapsed}
function feedSummary(){const values=Object.values(ui.live.feed_quality||{}),e=health().elapsed;return {total:values.length,good:values.filter(q=>q.transport?.startsWith('LIVE')&&(M.feedAge(q,e)??Infinity)<(q.stale_after_ms||90000)).length}}
function renderStatus(){
  const h=health(),f=feedSummary(),replay=ui.mode==='REPLAY';
  put('connection',replay?'REPLAY / SYNTHETIC':h.label);$('connection').className=replay?'amber':h.tone;
  put('clock',timeFormat.format(new Date())+' WIB');
  put('snapshotAge',replay?'WAKTU REPLAY':`SNAPSHOT ${age(h.elapsed)}`);
  put('feedCount',replay?'4 TF sintetis':`${f.good}/${f.total} feed`);
  put('footerMode',replay?'SYNTHETIC REPLAY':'LIVE OBSERVER');
  put('footerMessage',replay?'Contoh sintetis · maju satu candle untuk melihat proses':`${f.good}/${f.total} feed · ${ui.live.transport_state||'STARTING'} · browser ${ui.wsOpen?'WS':'HTTP'} · Binance WS ${ui.live.ws_state||'STARTING'}`);
  const decision=ui.data.symbols?.[ui.symbol]?.decision;
  const problem=!replay&&!['up'].includes(h.tone);
  put('chartWatermark',problem?`${h.label} · snapshot terakhir`:'');
  if(problem){put('decisionTitle','Data belum siap');put('decisionReason','Pembacaan setup ditampilkan dari snapshot terakhir. Tunggu data segar sebelum menilai kondisi sekarang.')}
  else if(decision&&!ui.candidateId){put('decisionTitle',decision.headline);put('decisionReason',decision.next_condition)}
  const series=ui.data.charts?.[ui.symbol+':'+ui.tf],preview=series?.preview;
  const exchangeNow=replay?ui.data.snapshot_at:Date.now()+(ui.live.server_offset_ms||0);
  put('barCountdown',preview&&!problem?`${tfName[ui.tf]} tutup ${age(Math.max(0,preview.close_time+1-exchangeNow))} · belum final`:replay?'Candle replay tertutup':problem?'Data tertunda':'Candle tertutup');
}
function render(){
  const keys=Object.keys(ui.data.symbols||{}),selector=$('symbol');
  if([...selector.options].map(v=>v.value).join('|')!==keys.join('|')){selector.replaceChildren(...keys.map(key=>{const option=el('option',key);option.value=key;return option}))}
  selector.value=ui.symbol;
  $('modeBanner').hidden=ui.mode!=='REPLAY';
  $('liveButton').classList.toggle('selected',ui.mode==='LIVE');$('replayButton').classList.toggle('selected',ui.mode==='REPLAY');
  document.querySelectorAll('[data-tf]').forEach(b=>b.classList.toggle('selected',b.dataset.tf===ui.tf));
  renderWatch();renderDecision();renderTerminal();renderStatus();queueDraw();
}
function renderWatch(){
  const rank={TRIGGERED:0,TOUCHED:1,ARMED:2,DETECTED:3,WAIT_M15:4,WAIT_H1:5,WAIT_H4:6};
  const pairs=Object.entries(ui.data.symbols||{}).sort(([a,x],[b,y])=>(rank[x.decision?.stage]??7)-(rank[y.decision?.stage]??7)||a.localeCompare(b));
  $('watchlist').replaceChildren(...pairs.map(([symbol,s])=>{const button=el('button',null,'watch-row'+(symbol===ui.symbol?' active':''));button.setAttribute('aria-label','Pilih '+symbol);button.onclick=()=>chooseSymbol(symbol);button.append(el('b',symbol),el('span',price(M.lastPrice(ui.data,symbol),symbol),'last'));const direction=s.bias==='BULLISH'?'↑ ':s.bias==='BEARISH'?'↓ ':'• ';button.append(el('small',direction+(stageName[s.decision?.stage]||s.status||'Menunggu'),s.bias==='BULLISH'?'up':s.bias==='BEARISH'?'down':'muted'));return button}));
  put('pairCount',`${pairs.length} PAIRS`);const active=pairs.filter(([,s])=>['ARMED','TOUCHED','TRIGGERED'].includes(s.zone?.state)).length;put('scanSummary',active?`${active} pair punya setup aktif`:'Mencari rangkaian setup baru');
}
function renderDecision(){
  const view=M.selectedSetup(ui.data,ui.symbol,ui.candidateId),s=view.symbol,d=view.decision,z=view.zone,c=view.candidate;
  const label=d?.headline||(view.isArchive?'Arsip setup':ui.symbol?'Menunggu data mesin':'Dataset belum tersedia');
  put('stageCode',d?.stage||z?.state||'WAITING');put('decisionTitle',label);put('decisionReason',d?.next_condition||'Referensi setup pilihan. Reset view untuk kembali ke kondisi sekarang.');
  put('chartSymbol',ui.symbol||'—');$('chartSymbol').append(el('span',tfName[ui.tf]));put('biasBadge',s.bias||'NEUTRAL');
  const last=M.lastPrice(ui.data,ui.symbol),series=ui.data.charts?.[ui.symbol+':5m'],bar=series?.preview||series?.closed?.at(-1);put('quote',price(last));
  if(bar&&bar.open){const change=(bar.close/bar.open-1)*100;$('quote').append(el('small',`${change>=0?'+':''}${change.toFixed(2)}% / M5`,change>=0?'up':'down'))}
  const gates=d?.gates||[];
  $('gates').replaceChildren(...gates.map(g=>{const row=el('div',null,'gate'),head=el('div',null,'gate-head');head.append(el('b',tfName[g.tf]),el('span',g.label),el('span',({PASS:'LOLOS',WAIT:'TUNGGU',BLOCKED:'BELUM',FAIL:'GUGUR'})[g.state],g.state==='PASS'?'gate-state up':g.state==='FAIL'?'gate-state down':g.state==='WAIT'?'gate-state amber':'gate-state muted'));row.append(head,el('p',g.detail));return row}));
  $('contextStrip').replaceChildren(...['4h','1h','15m','5m'].map(tf=>{const g=gates.find(v=>v.tf===tf);const b=el('button',null,'context-card'+(ui.tf===tf?' current':''));b.onclick=()=>chooseTf(tf);b.append(el('strong',tfName[tf]+' / '+({PASS:'LOLOS',WAIT:'TUNGGU',BLOCKED:'BELUM',FAIL:'GUGUR'})[g?.state||'BLOCKED'],g?.state==='PASS'?'up':g?.state==='WAIT'?'amber':'muted'),el('small',g?.label||'Konteks setup'));return b}));
  const distance=M.zoneDistance(last,z);const values=[['Area incaran',z?`${price(z.lower)} – ${price(z.upper)}`:'Belum terbentuk'],['Jarak ke area',distance?`${distance.percent.toFixed(2)}%`:'—'],['Invalidasi',price(z?.invalidation),'down'],['Entry referensi',price(c?.entry_reference),'amber'],['Target referensi',price(c?.target_reference),'up']];
  $('levels').replaceChildren(...values.map(([key,value,tone])=>{const row=el('div',null,'level');row.append(el('dt',key),el('dd',value,tone));return row}));
  put('setupKind',view.isArchive?'ARSIP':z?'AKTIF':'WAIT');
  put('setupMeta',z?`Dibentuk ${time(z.created_at)} WIB · batas ${time(z.expires_at)} WIB`:`Evaluasi M5: ${time(d?.evaluated_bar_at)} WIB`);
  $('outcomeBox').hidden=!view.outcome;
  if(view.outcome){const o=view.outcome;$('outcomeBox').replaceChildren(el('strong',`OBSERVASI ${o.bars}/4 · ${o.status==='COMPLETED'?'SELESAI':'BERJALAN'}`),el('div',`MFE ${rvalue(o.mfe)} / MAE ${rvalue(o.mae)} / akhir ${rvalue(o.final_r)}`),el('div',`Hit pertama: ${o.first_hit?.kind||'belum'} · hasil sebelum biaya`))}
}
function table(headers,rows){const t=el('table'),head=el('thead'),tr=el('tr');tr.append(...headers.map(x=>el('th',x)));head.append(tr);const body=el('tbody');for(const row of rows){const r=el('tr');for(const cell of row){const td=el('td');td.append(cell instanceof Node?cell:document.createTextNode(String(cell??'—')));r.append(td)}body.append(r)}t.append(head,body);return t}
function empty(title,detail){const wrapper=el('div',null,'empty-terminal'),text=el('div');text.append(el('strong',title),el('p',detail));wrapper.append(el('b','—'),text);return wrapper}
function renderTerminal(){
  document.querySelectorAll('[data-tab]').forEach(b=>{b.classList.toggle('selected',b.dataset.tab===ui.tab);b.setAttribute('aria-selected',String(b.dataset.tab===ui.tab))});
  const host=$('terminalBody'),data=ui.data,view=M.selectedSetup(data,ui.symbol,ui.candidateId);
  host.classList.toggle('research-active',['research','lab'].includes(ui.tab));
  if(ui.tab==='lab'){put('terminalContext','EXPERIMENTAL / TANPA ORDER');LabPanel.render(host,labData);return}
  if(ui.tab==='research'){put('terminalContext','RESEARCH / TERPISAH DARI CHART LIVE');ResearchPanel.render(host,researchData);return}
  put('candidateCount',(data.candidates||[]).length);put('terminalContext',`${ui.symbol||'—'} / ${ui.mode==='REPLAY'?'SYNTHETIC':'LIVE'} / WIB`);
  if(ui.tab==='journal'){
    const events=(data.events||[]).filter(e=>e.symbol===ui.symbol&&(!ui.candidateId||e.zone_id===view.zone?.zone_id));
    host.replaceChildren(events.length?table(['WAKTU / WIB','SYMBOL','EVENT','KETERANGAN'],events.map(e=>[time(e.time),e.symbol,e.to,M.eventName(e.to)])):empty('Belum ada perubahan setup',data.symbols?.[ui.symbol]?.decision?.next_condition||'Menunggu snapshot market.'));return;
  }
  if(ui.tab==='setups'){
    const candidates=[...(data.candidates||[])].sort((a,b)=>b.decision_time-a.decision_time);
    const rows=candidates.map(c=>{const o=data.outcomes?.[c.candidate_id];const button=el('button',c.symbol);button.onclick=()=>{ui.symbol=c.symbol;ui.candidateId=c.candidate_id;resetView();render()};return [time(c.decision_time),button,c.direction,price(c.entry_reference,c.symbol),price(c.stop_reference,c.symbol),price(c.target_reference,c.symbol),Number(c.score).toFixed(3),o?`${o.bars}/4 · ${o.status}`:'—',rvalue(o?.final_r)]});
    if(rows.length)host.replaceChildren(table(['SINYAL / WIB','SYMBOL','ARAH','ENTRY REF','STOP REF','TARGET REF','SKOR','OBSERVASI','AKHIR / GROSS'],rows),el('p','Skor = penilaian aturan, bukan probabilitas menang. Hasil observasi belum dikurangi biaya.','muted'));
    else host.replaceChildren(empty('Belum ada sinyal observasi','Setup yang sudah terpicu akan tampil di sini. Skor adalah penilaian aturan, bukan probabilitas menang.'));return;
  }
  if(ui.tab==='health'){
    if(ui.mode==='REPLAY'){host.replaceChildren(empty('Dataset sintetis / replay','Status koneksi live tersedia saat kembali ke mode LIVE. Replay tidak memakai stream Binance.'));return}
    const e=health().elapsed;const rows=Object.entries(ui.live.feed_quality||{}).map(([key,q])=>[key,q.transport,age(M.feedAge(q,e)),age(M.feedAge(q,e,'preview_age_ms')),q.lag_bars,q.last_error_class||'—']);
    const rt=ui.live.rest_telemetry||{},tasks=Object.values(ui.live.task_health||{});const note=el('div',`Task ${tasks.filter(t=>t.status==='RUNNING').length}/${tasks.length} · REST weight ${rt.used_weight_1m??'—'}/${rt.weight_limit_1m??'—'} · Binance WS ${ui.live.ws_state||'—'}`,'muted');note.style.padding='8px 12px';host.replaceChildren(note,table(['FEED','TRANSPORT','UPDATE AGE','PREVIEW AGE','LAG BAR','ERROR'],rows));return;
  }
  if(ui.mode==='REPLAY'){host.replaceChildren(empty('Akun tidak ditampilkan dalam replay','Kembali ke LIVE untuk snapshot akun Binance yang sebenarnya.'));return}
  const a=ui.account;if(!a){host.replaceChildren(empty('Mengambil akun read-only','Pemindaian market berjalan terpisah dari pembacaan akun.'));return}
  const aAge=(M.number(a.snapshot_age_ms)??Infinity)+(ui.accountAt===null?0:performance.now()-ui.accountAt);const auth=aAge>45000?'STALE':a.auth;
  host.replaceChildren(table(['AUTH','MODE','BALANCE TERSEDIA','POSISI','REGULAR','CONDITIONAL','EXEC ALGO','TOTAL ORDER','SNAPSHOT AGE'],[[auth,a.position_mode,price(a.available_balance),a.positions,a.regular_orders,a.conditional_orders,a.execution_algo_orders,a.total_orders,age(aAge)]]),el('p',a.warning||a.note||'','muted'));
}

const canvas=$('chart'),wrap=$('chartWrap');let chartGeometry=null;
function queueDraw(){if(drawQueued)return;drawQueued=true;requestAnimationFrame(()=>{drawQueued=false;draw()})}
new ResizeObserver(queueDraw).observe(wrap);
canvas.addEventListener('wheel',e=>{e.preventDefault();ui.viewCount=Math.max(16,Math.min(80,ui.viewCount+(e.deltaY>0?5:-5)));queueDraw()},{passive:false});
canvas.addEventListener('pointerdown',e=>{ui.drag={x:e.clientX,offset:ui.offset};canvas.setPointerCapture(e.pointerId)});
canvas.addEventListener('pointermove',e=>{const rect=canvas.getBoundingClientRect();ui.pointer={x:e.clientX-rect.left,y:e.clientY-rect.top};if(ui.drag&&chartGeometry){ui.offset=Math.max(0,Math.min(chartGeometry.total-ui.viewCount,ui.drag.offset+Math.round((e.clientX-ui.drag.x)/chartGeometry.step)));}queueDraw()});
canvas.addEventListener('pointerup',()=>{ui.drag=null});canvas.addEventListener('pointercancel',()=>{ui.drag=null});canvas.addEventListener('pointerleave',()=>{if(!ui.drag){ui.pointer=null;queueDraw()}});
function draw(){
  const width=wrap.clientWidth,height=wrap.clientHeight,dpr=window.devicePixelRatio||1;if(!width||!height)return;
  if(canvas.width!==Math.round(width*dpr)||canvas.height!==Math.round(height*dpr)){canvas.width=Math.round(width*dpr);canvas.height=Math.round(height*dpr)}
  const ctx=canvas.getContext('2d');ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,width,height);ctx.font='12px Consolas, monospace';
  const view=M.selectedSetup(ui.data,ui.symbol,ui.candidateId),series=ui.data.charts?.[ui.symbol+':'+ui.tf]||{},closed=series.closed||[];
  const preview=series.preview&&series.preview.open_time>(closed.at(-1)?.open_time??-Infinity)?{...series.preview,closed:false}:null;
  const all=preview?[...closed,preview]:closed;ui.offset=Math.max(0,Math.min(ui.offset,Math.max(0,all.length-ui.viewCount)));
  const end=all.length-ui.offset,visible=all.slice(Math.max(0,end-ui.viewCount),end);$('chartEmpty').hidden=visible.length>0;
  if(!visible.length){chartGeometry=null;return}
  const right=width<400?72:86,box={l:9,r:width-right,t:20,b:height-35};const plotW=box.r-box.l,plotH=box.b-box.t;
  let lo=Math.min(...visible.map(c=>c.low)),hi=Math.max(...visible.map(c=>c.high));
  const minT=visible[0].open_time,maxT=visible.at(-1).close_time+M.intervals[ui.tf]*2;
  const z=view.zone,c=view.candidate;
  // Only the selected setup may contribute levels, and only in its visible time range.
  const zoneEnd=z?Math.min(z.expires_at,view.isArchive?z.transition_at||z.updated_at:z.expires_at):0;
  const zoneVisible=z&&z.created_at<=maxT&&zoneEnd>=minT;
  const candidateVisible=c&&c.decision_time>=minT&&c.decision_time<=maxT;
  const levelValues=[];if(zoneVisible)levelValues.push(z.lower,z.upper,z.invalidation);if(candidateVisible)levelValues.push(c.entry_reference,c.stop_reference,c.target_reference);
  if(levelValues.length){lo=Math.min(lo,...levelValues);hi=Math.max(hi,...levelValues)}const padding=(hi-lo)*.1||1;lo-=padding;hi+=padding;
  const x=t=>box.l+(t-minT)/(maxT-minT)*plotW;
  const y=v=>box.t+(hi-v)/(hi-lo)*plotH;
  const step=M.intervals[ui.tf]/(maxT-minT)*plotW;chartGeometry={total:all.length,step,minT,maxT,lo,hi,box,visible};
  ctx.lineWidth=1;ctx.strokeStyle='#1d2832';ctx.fillStyle='#8394a3';
  for(let i=0;i<=5;i++){const value=hi-i*(hi-lo)/5,yy=y(value);ctx.beginPath();ctx.moveTo(box.l,yy);ctx.lineTo(box.r,yy);ctx.stroke();ctx.fillText(price(value),box.r+7,yy+4)}
  const ticks=width<500?3:5;
  for(let i=0;i<ticks;i++){const t=minT+i*(maxT-minT)/(ticks-1),xx=x(t);ctx.beginPath();ctx.moveTo(xx,box.t);ctx.lineTo(xx,box.b);ctx.stroke();const label=time(t).replace(/\./g,':');ctx.font='11px Consolas, monospace';ctx.fillText(label,Math.max(3,Math.min(width-120,xx-35)),height-11)}ctx.font='12px Consolas, monospace';
  ctx.save();ctx.beginPath();ctx.rect(box.l,box.t,plotW,plotH);ctx.clip();
  if(zoneVisible){const left=x(Math.max(z.created_at,minT)),rightX=x(Math.min(zoneEnd,maxT));ctx.fillStyle=view.isArchive?'#ffb23e0b':'#ffb23e20';ctx.fillRect(left,y(z.upper),rightX-left,Math.max(2,y(z.lower)-y(z.upper)));ctx.strokeStyle=view.isArchive?'#886335':'#ffb23e';ctx.setLineDash([5,4]);ctx.strokeRect(left,y(z.upper),rightX-left,Math.max(2,y(z.lower)-y(z.upper)));ctx.setLineDash([])}
  const bw=Math.max(2,Math.min(12,step*.62));
  for(const bar of visible){const xx=x((bar.open_time+bar.close_time)/2),color=!bar.closed?'#ffb23e':bar.close>=bar.open?'#50d6a0':'#f47785';ctx.strokeStyle=color;ctx.fillStyle=color;ctx.setLineDash(bar.closed?[]:[3,2]);ctx.beginPath();ctx.moveTo(xx,y(bar.high));ctx.lineTo(xx,y(bar.low));ctx.stroke();ctx.setLineDash([]);const yy=Math.min(y(bar.open),y(bar.close)),hh=Math.max(1,Math.abs(y(bar.open)-y(bar.close)));if(bar.closed)ctx.fillRect(xx-bw/2,yy,bw,hh);else ctx.strokeRect(xx-bw/2,yy,bw,hh)}
  if(ui.tf==='4h')for(const pivot of view.symbol.pivots||[]){if(pivot[3]<minT||pivot[3]>maxT)continue;const xx=x(pivot[3]),yy=y(pivot[2]);ctx.fillStyle=pivot[0]==='HIGH'?'#f47785':'#50d6a0';ctx.beginPath();ctx.moveTo(xx,yy);ctx.lineTo(xx-4,yy+(pivot[0]==='HIGH'?-7:7));ctx.lineTo(xx+4,yy+(pivot[0]==='HIGH'?-7:7));ctx.closePath();ctx.fill()}
  const impulse=view.impulse;
  if(ui.tf==='1h'&&impulse&&impulse.end_time>=minT&&impulse.start_time<=maxT){ctx.strokeStyle='#72b9dc';ctx.lineWidth=2;ctx.beginPath();ctx.moveTo(x(impulse.start_time),y(impulse.origin));ctx.lineTo(x(impulse.end_time),y(impulse.end));ctx.stroke();ctx.lineWidth=1;ctx.fillStyle='#72b9dc';ctx.fillText('H1 IMPULSE',Math.max(box.l,x(impulse.end_time)-85),Math.max(box.t+15,y(impulse.end)-8))}
  if(candidateVisible){for(const [label,value,color] of [['ENTRY REF',c.entry_reference,'#72b9dc'],['STOP REF',c.stop_reference,'#f47785'],['TARGET REF',c.target_reference,'#50d6a0']]){ctx.strokeStyle=color;ctx.setLineDash([5,4]);ctx.beginPath();ctx.moveTo(x(c.decision_time),y(value));ctx.lineTo(box.r,y(value));ctx.stroke();ctx.setLineDash([]);ctx.fillStyle=color;ctx.fillText(label,box.r-87,Math.max(box.t+13,y(value)-7))}}
  if(zoneVisible){ctx.fillStyle='#ffb23e';ctx.fillText(`${view.isArchive?'ARSIP':'AREA'} / ${z.state}`,Math.max(box.l+3,x(z.created_at)+4),Math.max(box.t+13,y(z.upper)-8))}
  if(view.trigger&&view.trigger.time>=minT&&view.trigger.time<=maxT){ctx.fillStyle='#72b9dc';ctx.beginPath();ctx.arc(x(view.trigger.time),y(view.trigger.price),4,0,Math.PI*2);ctx.fill()}
  const last=visible.at(-1);ctx.strokeStyle='#658093';ctx.setLineDash([2,4]);ctx.beginPath();ctx.moveTo(box.l,y(last.close));ctx.lineTo(box.r,y(last.close));ctx.stroke();ctx.setLineDash([]);ctx.restore();
  const latestInView=ui.offset===0;ctx.fillStyle=latestInView?'#304034':'#27323c';ctx.fillRect(box.r+1,y(last.close)-9,right-2,18);ctx.fillStyle='#e0e7ec';ctx.fillText(price(last.close),box.r+6,y(last.close)+4);
  if(ui.crosshair&&ui.pointer&&ui.pointer.x>=box.l&&ui.pointer.x<=box.r&&ui.pointer.y>=box.t&&ui.pointer.y<=box.b){const p=ui.pointer;ctx.strokeStyle='#8496a6';ctx.setLineDash([3,3]);ctx.beginPath();ctx.moveTo(p.x,box.t);ctx.lineTo(p.x,box.b);ctx.moveTo(box.l,p.y);ctx.lineTo(box.r,p.y);ctx.stroke();ctx.setLineDash([]);const cursorT=minT+(p.x-box.l)/plotW*(maxT-minT),bar=visible.reduce((best,item)=>Math.abs(item.open_time-cursorT)<Math.abs(best.open_time-cursorT)?item:best,visible[0]);put('ohlc',`${time(bar.open_time)} · O ${price(bar.open)} H ${price(bar.high)} L ${price(bar.low)} C ${price(bar.close)}${bar.closed?'':' / PREVIEW'}`)}
  else put('ohlc',`O ${price(last.open)} H ${price(last.high)} L ${price(last.low)} C ${price(last.close)}${last.closed?'':' / PREVIEW'}`);
  const q=ui.data.feed_quality?.[ui.symbol+':'+ui.tf];put('chartDataStatus',ui.mode==='REPLAY'?'SYNTHETIC / WIB':`${visible.length} bars · update ${age(M.feedAge(q,elapsed()))} · WIB`);
}
connect();poll();loadAccount();setInterval(poll,4000);setInterval(loadAccount,15000);setInterval(()=>{renderStatus();if(['health','account'].includes(ui.tab))renderTerminal();queueDraw()},1000);
loadResearch();setInterval(loadResearch,30000);setInterval(()=>{if(ui.tab==='research')renderTerminal()},1000);
loadLab();setInterval(loadLab,15000);setInterval(()=>{if(ui.tab==='lab')renderTerminal()},1000);
if(new URLSearchParams(location.search).get('mode')==='replay')$('replayButton').click();
if(new URLSearchParams(location.search).get('tab')==='research')document.querySelector('[data-tab="research"]').click();
if(new URLSearchParams(location.search).get('tab')==='lab')document.querySelector('[data-tab="lab"]').click();
