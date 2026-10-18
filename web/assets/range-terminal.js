'use strict';
const R=RangeModel,$=id=>document.getElementById(id);
const ui={live:{},data:{},symbol:'BTCUSDT',tf:'5m',tab:'positions',mode:'LIVE',received:0,stamp:0,failed:false,ws:false,count:180,offset:0,pointer:null,drag:null,frame:0,demo:null,playing:null,account:null,archive:null};
let socket,retry=1000,polling=false,accountPolling=false,drawQueued=false;
const fmt=new Intl.DateTimeFormat('id-ID',{timeZone:'Asia/Jakarta',day:'2-digit',month:'short',hour:'2-digit',minute:'2-digit',hourCycle:'h23'});
const clockFmt=new Intl.DateTimeFormat('id-ID',{timeZone:'Asia/Jakarta',hour:'2-digit',minute:'2-digit',second:'2-digit',hourCycle:'h23'});
const time=v=>R.number(v)===null?'—':fmt.format(new Date(v));
const age=v=>!Number.isFinite(v)?'—':v<60000?`${Math.max(0,v/1000).toFixed(0)} dtk`:`${Math.floor(v/60000)} mnt`;
const pct=v=>R.number(v)===null?'—':`${v>=0?'+':''}${Number(v).toFixed(3)}%`;
const num=v=>R.number(v)===null?'—':Number(v).toFixed(3);
function price(v,symbol=ui.symbol){if(R.number(v)===null)return '—';const tick=ui.data.instrument_meta?.[symbol]?.tick_size;const digits=tick?(String(tick).replace(/0+$/,'').split('.')[1]||'').length:Math.abs(v)>=10000?1:Math.abs(v)>=100?2:Math.abs(v)>=1?3:5;return Number(v).toLocaleString('en-US',{minimumFractionDigits:digits,maximumFractionDigits:Math.min(8,digits)})}
function el(tag,text,cls){const n=document.createElement(tag);if(text!==undefined&&text!==null)n.textContent=String(text);if(cls)n.className=cls;return n}
function put(id,text){$(id).textContent=String(text??'—')}
function session(){return ui.data.reference?.sessions?.[ui.symbol]||{}}
function elapsed(){return ui.mode!=='LIVE'?0:ui.received?performance.now()-ui.received:Infinity}
function marketPrice(symbol){const rows=R.series(ui.data,symbol,'5m');return rows.at(-1)?.close}
function selectSymbol(symbol){ui.symbol=symbol;ui.offset=0;ui.pointer=null;render()}
async function json(url){const response=await fetch(url,{cache:'no-store',signal:AbortSignal.timeout(6000)});if(!response.ok)throw new Error('HTTP '+response.status);return response.json()}
function receive(data){if(!data?.charts||!Number.isFinite(data.snapshot_at)||data.snapshot_at<ui.stamp)return;if(data.snapshot_at!==ui.stamp){ui.received=performance.now();ui.stamp=data.snapshot_at}ui.live=data;ui.failed=false;if(ui.mode==='LIVE'){ui.data=data;render()}}
async function poll(){if(polling)return;polling=true;try{receive(await json('/api/state'))}catch{ui.failed=true;renderStatus()}finally{polling=false}}
function connect(){socket=new WebSocket(`${location.protocol==='https:'?'wss':'ws'}://${location.host}/ws`);socket.onopen=()=>{ui.ws=true;retry=1000};socket.onmessage=e=>{try{receive(JSON.parse(e.data))}catch{/* A malformed snapshot cannot refresh freshness. */}};socket.onerror=()=>socket.close();socket.onclose=()=>{ui.ws=false;renderStatus();setTimeout(connect,retry);retry=Math.min(30000,retry*2)}}
async function account(){if(accountPolling||ui.mode!=='LIVE')return;accountPolling=true;try{ui.account=await json('/api/account')}catch{ui.account={...(ui.account||{}),auth:'STALE'}}finally{accountPolling=false;if(ui.tab==='account')renderPanel()}}
async function archive(){try{ui.archive=await json('/api/lab')}catch{ui.archive={status:'UNAVAILABLE'}}if(ui.tab==='archive')renderPanel()}
function renderStatus(){
  const demo=ui.mode==='DEMO',historical=ui.mode==='HISTORY',recorded=demo||historical,e=elapsed(),s=session();
  const referenceAge=Math.max(0,Date.now()+(ui.live.server_offset_ms||0)-(ui.data.reference?.updated_at||0));
  const fresh=recorded||(!ui.failed&&navigator.onLine&&e<15000&&referenceAge<15000&&R.fresh(s,referenceAge)&&ui.data.reference?.status==='READ_ONLY_MODEL');
  put('connection',demo?'DEMO SINTETIS':historical?'REKONSTRUKSI HISTORIS':fresh?ui.live.transport_state||'MARKET LIVE':'DATA TERTUNDA');
  $('connection').className=fresh?'up':'amber';put('clock',clockFmt.format(new Date())+' WIB');
  put('snapshotAge',recorded?'DATA REKAMAN':`SNAPSHOT ${age(e)}`);
  const feeds=Object.values(ui.live.feed_quality||{});const good=feeds.filter(q=>q.transport?.startsWith('LIVE')&&(q.success_age_ms??Infinity)+e<(q.stale_after_ms||90000)).length;
  put('feedCount',recorded?'Data rekaman / bukan live':`${good}/${feeds.length} feed market`);
  put('footerMode',demo?'SYNTHETIC · BUKAN HASIL MARKET':historical?'HISTORICAL RECONSTRUCTION':'REFERENCE MODEL · TANPA ORDER');
  put('footerMessage',demo?'Contoh range → breakout → tahan → exit sesi':historical?'Hindsight: termasuk candle setelah exit. Bukan sinyal saat ini.':`${ui.live.ws_state||'STARTING'} · ${ui.live.transport_state||'STARTING'} · browser ${ui.ws?'WS':'HTTP'}`);
  put('chartWatermark',demo?'SYNTHETIC DEMO':historical?'HISTORICAL / HINDSIGHT':fresh?'': 'DATA TERTUNDA · jangan anggap state lama sebagai sinyal baru');
  const rows=R.series(ui.data,ui.symbol,ui.tf),preview=rows.at(-1),now=Date.now()+(ui.live.server_offset_ms||0);
  put('barCountdown',recorded?time(ui.data.snapshot_at)+' WIB':!fresh?'State terakhir / data belum segar':preview&&!preview.closed?`Candle tutup ${age(Math.max(0,preview.close_time+1-now))}`:'Candle tertutup');
  if(!fresh&&!demo){put('decisionTitle','Menunggu data segar');put('decisionReason','State di bawah adalah pembacaan terakhir. Mesin tidak memiliki kemampuan mengirim order.')}else{put('decisionTitle',R.stateLabel(s.status));put('decisionReason',R.explanation(s))}
}
function render(){
  const keys=Object.keys(ui.data.symbols||{});if(keys.length&&!keys.includes(ui.symbol))ui.symbol=keys[0];
  const selector=$('symbol');if([...selector.options].map(o=>o.value).join('|')!==keys.join('|'))selector.replaceChildren(...keys.map(s=>{const o=el('option',s);o.value=s;return o}));selector.value=ui.symbol;
  $('liveButton').classList.toggle('selected',ui.mode==='LIVE');$('demoButton').classList.toggle('selected',ui.mode==='DEMO');$('demoBanner').hidden=ui.mode!=='DEMO';
  $('historyBanner').hidden=ui.mode!=='HISTORY';
  document.querySelectorAll('[data-tf]').forEach(b=>{b.classList.toggle('selected',ui.tf===b.dataset.tf);b.disabled=ui.mode!=='LIVE'&&b.dataset.tf!=='5m'});
  put('pairCount',`${keys.length} PAIRS`);
  $('watchlist').replaceChildren(...keys.map(symbol=>{const s=ui.data.reference?.sessions?.[symbol]||{},b=el('button',null,'watch-row'+(symbol===ui.symbol?' active':''));b.setAttribute('aria-label','Pilih '+symbol);b.onclick=()=>selectSymbol(symbol);b.append(el('b',symbol),el('span',price(marketPrice(symbol),symbol),'last'),el('small',`${s.direction||'•'} · ${R.stateLabel(s.status)}`,s.status==='HOLDING'?'amber':'muted'));return b}));
  const s=session();put('chartSymbol',`${ui.symbol} / ${ui.tf.toUpperCase()}`);put('quote',price(marketPrice(ui.symbol)));put('chartMode',ui.mode==='DEMO'?'SYNTHETIC DEMO':`${R.stateLabel(s.status).toUpperCase()} / MODEL`);
  put('stageCode',s.status||'STARTING');put('origin',s.origin==='LIVE_OBSERVED'?'TERAMATI LIVE · BUKAN ORDER':s.origin==='SYNTHETIC'?'SINTETIS · BUKAN HASIL MARKET':'REKONSTRUKSI SESI · BUKAN ORDER');
  const current=R.stageIndex(s.status);$('progress').replaceChildren(...['RANGE','BREAKOUT','HOLD','SELESAI'].map((name,i)=>el('span',name,i<current?'done':i===current?'current':'')));
  const rows=[['Range high',price(s.upper),'amber'],['Range low',price(s.lower),'amber'],['Candle range',`${s.range_count??0}/${s.range_expected??36}`],['Entry model',s.entry?`${s.direction} / ${price(s.entry)}`:'Belum ada'],['Stop awal',price(s.stop),'down'],['Take profit','Tidak dipasang'],['Candle posisi',s.entry?`${s.held_bars??0} M5 / maks. ${(s.held_bars??0)*5} mnt`:'—'],['Exit sesi',time(s.session_end)+' WIB']];
  $('levels').replaceChildren(...rows.map(([label,value,tone])=>{const row=el('div',null,'level');row.append(el('dt',label),el('dd',value,tone));return row}));
  const result=R.positionValue(s);$('positionResult').replaceChildren();if(s.entry){$('positionResult').append(el('small',result.label),el('b',pct(result.value),result.value>=0?'up':'down'),el('small',s.exit_at?`${s.reason} · ${time(s.reason==='SESSION_END'?s.session_end:s.exit_at)} WIB`:'Posisi hipotetis, terpisah dari posisi akun.'))}
  put('modelNote',s.revision_warning?'PERINGATAN: data sesi berubah setelah hasil tersimpan. Hasil lama tidak ditulis ulang.':s.funding_status==='SUPPLIED'?'Stop tetap, tanpa trailing. Funding historis disertakan. Bukan order broker.':'Stop tetap, tanpa trailing. Funding live belum tersedia. Tidak ada order yang dikirim.');
  renderPanel();renderStatus();queueDraw();
}
function table(headers,rows){const t=el('table'),head=el('thead'),tr=el('tr');tr.append(...headers.map(h=>el('th',h)));head.append(tr);const body=el('tbody');rows.forEach(row=>{const r=el('tr');row.forEach(value=>{const td=el('td');td.append(value instanceof Node?value:document.createTextNode(String(value??'—')));r.append(td)});body.append(r)});t.append(head,body);return t}
function content(title,text){const n=el('div',null,'content');n.append(el('h3',title),el('p',text));return n}
function metric(label,value){const m=el('div',null,'metric');m.append(el('small',label),el('b',value));return m}
function renderPanel(){
  const host=$('terminalBody'),top=host.scrollTop,left=host.scrollLeft;
  paintPanel();host.scrollTop=top;host.scrollLeft=left;
}
function paintPanel(){
  document.querySelectorAll('[data-tab]').forEach(b=>{b.classList.toggle('selected',ui.tab===b.dataset.tab);b.setAttribute('aria-selected',String(ui.tab===b.dataset.tab))});
  const host=$('terminalBody'),ref=ui.data.reference||{};put('panelContext',`${ui.mode==='DEMO'?'SINTETIS':'MODEL'} / WIB`);
  if(ui.tab==='positions'){
    const rows=(ref.recent||[]).map(s=>{const button=el('button',s.symbol);button.onclick=()=>ui.mode==='DEMO'?selectSymbol(s.symbol):openSession(s);return [s.date+' UTC',button,R.stateLabel(s.status),s.direction||'—',price(s.entry,s.symbol),price(s.stop,s.symbol),s.entry?`${s.held_bars} M5`:'—',pct(R.positionValue(s).value),s.status==='CLOSED'?(s.net_pct===null?'Fee/slip; belum funding':'Net termasuk funding'):'Harga / sebelum biaya',s.origin||'RECONSTRUCTED']});
    host.replaceChildren(rows.length?table(['SESI','PAIR','STATUS MODEL','ARAH','ENTRY','STOP','DURASI','HASIL / % HARGA','CAKUPAN','ASAL'],rows):el('div','Belum ada sesi yang tercatat.','empty'));return;
  }
  if(ui.tab==='evidence'){
    if(ui.mode==='DEMO'){host.replaceChildren(content('Demo bukan bukti performa','Contoh sintetis hanya menguji urutan mesin dan tampilan. Kembali ke Market Live untuk melihat hasil historis.'));return}
    const h=ref.history;if(!h?.base){host.replaceChildren(content('Hasil versi ini belum tersedia',h?.note||'Belum ada laporan historis yang cocok dengan fingerprint mesin aktif. Tidak ada klaim profit.'));return}
    const n=content('Hasil adaptasi Binance · bukan hasil EA vendor',h.note),m=el('div',null,'metrics');
    m.append(metric('EVENT SELESAI',h.base.closed),metric('RATA-RATA NET / R',num(h.base.mean_net_r)),metric('STRESS / R',num(h.stress.mean_net_r)),metric('PROFIT FACTOR',num(h.base.profit_factor)),metric('AMBIGU / GAP',(h.base.status_counts.AMBIGUOUS||0)+' / '+(h.base.status_counts.DATA_GAP||0)));
    n.append(m,el('p','R adalah unit risiko event, bukan target profit atau return modal. Statistik hanya event yang dapat diselesaikan model. Hasil yang ambigu tidak boleh dianggap menang.','notice'));
    n.append(table(['PAIR','EVENT','NET / R','DURASI RATA-RATA / M5'],Object.entries(h.by_symbol||{}).map(([s,v])=>[s,v.closed,num(v.mean_net_r),num(v.mean_held_bars)])));
    n.append(el('h3','Sesi terakhir pada dataset · bukan pilihan pemenang'),table(['SESI UTC','PAIR','HASIL NET / % HARGA','BUKTI'],(h.recent||[]).slice(0,12).map(s=>{const b=el('button','Lihat chart sesi');b.onclick=()=>openSession(s);return [s.date,s.symbol,pct(s.net_pct),b]})));host.replaceChildren(n);return;
  }
  if(ui.tab==='rules'){
    const n=content('Satu metode acuan, tanpa optimasi parameter','Range 03:00–06:00 UTC. Breakout pertama sampai 18:00 UTC. Stop di sisi berlawanan. Tidak ada TP, trailing, recovery, atau exit empat candle.');
    const a=el('a','Lihat penjelasan sumber / 03:36');a.href='https://www.youtube.com/watch?v=zYTdc0Q1BCs&t=216s';a.target='_blank';a.rel='noopener noreferrer';n.append(a,el('p','Adaptasi eksplisit: Binance perpetual, jam UTC, enam pair, termasuk akhir pekan, dan model M5. Zona waktu broker sumber belum dipastikan; hasilnya bukan replikasi performa MT5.','notice'),el('p',`Versi: ${ref.specification?.id||'memuat'} · Fee/slippage model 5/2 bps per sisi; bukan tarif akun yang telah diverifikasi.`));host.replaceChildren(n);return;
  }
  if(ui.tab==='archive'){
    const n=content('Strategi lama dihentikan sebagai mesin utama','H4 → H1 → M15 → M5, delapan variasi, target 1R/1,5R dan exit 4/8 candle disimpan untuk reproduksi. Loop observasinya tidak dijalankan lagi.');
    n.append(el('p','Hasil Fase 3 lama: kandidat terpilih 76 event holdout, mean net −0,227142R. Ini hasil strategi lama, bukan hasil Range Breakout.','notice'),el('p',`Akses arsip: ${ui.archive?.status||'memuat'} · Dokumen dan database lama dipertahankan.`));host.replaceChildren(n);return;
  }
  if(ui.tab==='health'){
    if(ui.mode!=='LIVE'){host.replaceChildren(content('Koneksi live terpisah dari rekaman','Kembali ke Market Live untuk melihat transport saat ini.'));return}
    const telemetry=ui.live.rest_telemetry||{},tasks=Object.values(ui.live.task_health||{}),n=content('Transport market dan kesehatan mesin',`${tasks.filter(t=>t.status==='RUNNING').length}/${tasks.length} task · REST weight ${telemetry.used_weight_1m??'—'}/${telemetry.weight_limit_1m??'—'} · Binance WS ${ui.live.ws_state||'—'}`);
    n.append(table(['FEED','TRANSPORT','LAG / BAR','ERROR'],Object.entries(ui.live.feed_quality||{}).map(([key,q])=>[key,q.transport,q.lag_bars,q.last_error_class||'—'])));host.replaceChildren(n);return;
  }
  if(ui.tab==='account'){
    if(ui.mode!=='LIVE'){host.replaceChildren(content('Akun disembunyikan dalam rekaman','Posisi model dalam rekaman tidak terkait dengan akun Binance.'));return}
    const a=ui.account||{},n=content('Akun Binance · akses baca saja','Posisi model di chart bukan posisi pada akun ini. Pengaturan API whitelist tidak diubah.');
    n.append(table(['KONEKSI','POSISI AKTUAL','TOTAL ORDER','SNAPSHOT AGE','IP RESTRICT'],[[a.auth||'Memuat',a.positions??'—',a.total_orders??'—',age(a.snapshot_age_ms),a.ip_restrict??'—']]),el('p','Panel sengaja tidak menampilkan kredensial. Tidak tersedia tombol order/cancel/leverage.'));host.replaceChildren(n);
  }
}
function queueDraw(){if(drawQueued)return;drawQueued=true;requestAnimationFrame(()=>{drawQueued=false;drawChart()})}
async function openSession(s){
  try{const query=new URLSearchParams({symbol:s.symbol,day:String(s.day)});if(s.dataset)query.set('dataset',s.dataset);const data=await json('/api/reference/session?'+query);stopDemo();ui.mode='HISTORY';ui.data=data;ui.symbol=s.symbol;ui.tf='5m';ui.offset=0;ui.count=288;ui.pointer=null;put('historyDate',s.date+' UTC');render();}
  catch{put('footerMessage','Rekaman sesi tidak tersedia untuk versi/data ini. Tidak mengganti chart dengan data lain.');}
}
function drawChart(){
  const wrap=$('chartWrap'),canvas=$('chart'),rect=wrap.getBoundingClientRect(),dpr=window.devicePixelRatio||1,w=rect.width,h=rect.height;
  if(w<20||h<20)return;canvas.width=Math.round(w*dpr);canvas.height=Math.round(h*dpr);const ctx=canvas.getContext('2d');ctx.setTransform(dpr,0,0,dpr,0,0);ctx.clearRect(0,0,w,h);ctx.fillStyle='#0b1118';ctx.fillRect(0,0,w,h);
  const all=R.series(ui.data,ui.symbol,ui.tf);ui.offset=Math.min(ui.offset,Math.max(0,all.length-16));const end=Math.max(1,all.length-ui.offset),bars=all.slice(Math.max(0,end-ui.count),end);$('chartEmpty').hidden=bars.length>0;if(!bars.length)return;
  const s=session(),interval=({'5m':300000,'15m':900000,'1h':3600000,'4h':14400000})[ui.tf],left=12,right=88,top=24,bottom=28,pw=w-left-right,ph=h-top-bottom;
  const tmin=bars[0].open_time-interval,tmax=bars.at(-1).open_time+3*interval;
  const extra=[s.upper,s.lower,s.stop,s.entry].filter(v=>R.number(v)!==null);
  let lo=Math.min(...bars.map(b=>b.low),...extra),hi=Math.max(...bars.map(b=>b.high),...extra),pad=(hi-lo||Math.abs(hi)*.01||1)*.12;lo-=pad;hi+=pad;
  const x=t=>left+(t-tmin)/(tmax-tmin)*pw;
  const y=v=>top+(hi-v)/(hi-lo)*ph;
  ctx.font='10px Consolas';ctx.textBaseline='middle';
  for(let i=0;i<=5;i++){const value=lo+(hi-lo)*i/5,yy=y(value);ctx.strokeStyle='#1b2631';ctx.beginPath();ctx.moveTo(left,yy);ctx.lineTo(w-right,yy);ctx.stroke();ctx.fillStyle='#8294a6';ctx.fillText(price(value),w-right+8,yy)}
  ctx.save();ctx.beginPath();ctx.rect(left,top,pw,ph);ctx.clip();
  if(s.upper!==null&&s.lower!==null&&s.range_start){const x1=x(s.range_start),x2=x(s.range_end);ctx.fillStyle='#ffc06313';ctx.fillRect(x1,y(s.upper),x2-x1,y(s.lower)-y(s.upper));ctx.strokeStyle='#ffc06380';ctx.strokeRect(x1,y(s.upper),x2-x1,y(s.lower)-y(s.upper));ctx.fillStyle='#d9ad64';ctx.fillText('SESSION RANGE',Math.max(left+5,x1+5),Math.max(top+10,y(s.upper)-9))}
  const bw=Math.max(1,Math.min(10,pw/(bars.length+3)*.65));
  for(const b of bars){ctx.globalAlpha=b.closed===false?.45:1;const xx=x(b.open_time),color=b.close>=b.open?'#59c9a6':'#ec7886';ctx.strokeStyle=color;ctx.fillStyle=color;ctx.beginPath();ctx.moveTo(xx,y(b.high));ctx.lineTo(xx,y(b.low));ctx.stroke();ctx.fillRect(xx-bw/2,Math.min(y(b.open),y(b.close)),bw,Math.max(1,Math.abs(y(b.open)-y(b.close))))}ctx.globalAlpha=1;
  function level(value,color,start){if(R.number(value)===null)return;ctx.strokeStyle=color;ctx.setLineDash([4,4]);ctx.beginPath();ctx.moveTo(Math.max(left,x(start||bars[0].open_time)),y(value));ctx.lineTo(w-right,y(value));ctx.stroke();ctx.setLineDash([])}
  level(s.upper,'#d4ab62',s.range_end);level(s.lower,'#d4ab62',s.range_end);level(s.entry,'#72bfe4',s.entry_bar_open);level(s.stop,'#fa8195',s.entry_bar_open);
  if(s.entry){const xx=x(s.entry_bar_open),yy=y(s.entry);ctx.fillStyle='#83d3f5';ctx.beginPath();ctx.moveTo(xx,yy);ctx.lineTo(xx-5,yy-10);ctx.lineTo(xx+5,yy-10);ctx.closePath();ctx.fill();ctx.fillText(s.direction+' MODEL',xx+8,yy-13)}
  if(s.exit_at&&R.number(s.exit)!==null){ctx.fillStyle='#ffc063';ctx.beginPath();ctx.arc(x(s.exit_at),y(s.exit),4,0,Math.PI*2);ctx.fill()}
  ctx.restore();
  [['HIGH',s.upper,'#d4ab62'],['LOW',s.lower,'#d4ab62'],['ENTRY',s.entry,'#72bfe4'],['STOP',s.stop,'#fa8195']].forEach(([name,v,color])=>{if(R.number(v)===null)return;ctx.fillStyle='#111a23';ctx.fillRect(w-right+2,y(v)-9,right-3,18);ctx.fillStyle=color;ctx.font='9px Consolas';ctx.fillText(name+' '+price(v),w-right+5,y(v))});
  ctx.font='10px Consolas';ctx.fillStyle='#8294a6';for(let i=0;i<5;i++){const index=Math.min(bars.length-1,Math.floor(i*(bars.length-1)/4)),b=bars[index];ctx.fillText(clockFmt.format(new Date(b.open_time)).slice(0,5),Math.max(left,Math.min(w-right-40,x(b.open_time)-16)),h-12)}
  if(ui.pointer&&ui.pointer.x>left&&ui.pointer.x<w-right){const p=ui.pointer;let nearest=bars[0];for(const b of bars)if(Math.abs(x(b.open_time)-p.x)<Math.abs(x(nearest.open_time)-p.x))nearest=b;ctx.strokeStyle='#7c8f9c80';ctx.setLineDash([3,4]);ctx.beginPath();ctx.moveTo(x(nearest.open_time),top);ctx.lineTo(x(nearest.open_time),h-bottom);ctx.moveTo(left,p.y);ctx.lineTo(w-right,p.y);ctx.stroke();ctx.setLineDash([]);put('ohlc',`${time(nearest.open_time)} WIB · O ${price(nearest.open)} H ${price(nearest.high)} L ${price(nearest.low)} C ${price(nearest.close)}${nearest.closed===false?' · PREVIEW':''}`)}
}
function stopDemo(){if(ui.playing)clearInterval(ui.playing);ui.playing=null;put('demoPlay','Putar')}
function demoFrame(index){ui.frame=Math.max(0,Math.min(ui.demo.frames.length-1,index));ui.data=ui.demo.frames[ui.frame];ui.symbol=Object.keys(ui.data.symbols)[0];ui.tf='5m';ui.offset=0;ui.pointer=null;$('demoRange').value=String(ui.frame);put('demoProgress',`${ui.frame+1}/${ui.demo.frames.length} · ${time(ui.data.snapshot_at)} WIB`);render();if(ui.frame===ui.demo.frames.length-1)stopDemo()}
$('liveButton').onclick=()=>{stopDemo();ui.mode='LIVE';ui.data=ui.live;ui.offset=0;ui.pointer=null;render()};
$('returnLive').onclick=()=>$('liveButton').click();
$('demoButton').onclick=async()=>{try{ui.demo=ui.demo||await json('/api/reference/replay');stopDemo();ui.mode='DEMO';demoFrame(0)}catch{put('footerMessage','Demo gagal dimuat. Coba kembali.')}};
$('demoNext').onclick=()=>{stopDemo();demoFrame(ui.frame+1)};$('demoRange').oninput=e=>{stopDemo();demoFrame(Number(e.target.value))};
$('demoPlay').onclick=()=>{if(ui.playing){stopDemo();return}if(ui.frame===ui.demo.frames.length-1)demoFrame(0);put('demoPlay','Jeda');ui.playing=setInterval(()=>demoFrame(ui.frame+1),1500)};
$('symbol').onchange=e=>selectSymbol(e.target.value);
document.querySelectorAll('[data-tf]').forEach(b=>b.onclick=()=>{ui.tf=b.dataset.tf;ui.offset=0;ui.pointer=null;render()});
document.querySelectorAll('[data-tab]').forEach(b=>b.onclick=()=>{ui.tab=b.dataset.tab;if(ui.tab==='account')account();if(ui.tab==='archive')archive();renderPanel()});
$('zoomIn').onclick=()=>{ui.count=Math.max(20,ui.count-25);queueDraw()};$('zoomOut').onclick=()=>{ui.count=Math.min(400,ui.count+25);queueDraw()};$('resetChart').onclick=()=>{ui.count=180;ui.offset=0;ui.pointer=null;queueDraw()};
$('chart').addEventListener('pointermove',e=>{const rect=$('chart').getBoundingClientRect();ui.pointer={x:e.clientX-rect.left,y:e.clientY-rect.top};if(ui.drag){ui.offset=Math.max(0,ui.drag.offset+Math.round((e.clientX-ui.drag.x)/rect.width*ui.count));}queueDraw()});
$('chart').addEventListener('pointerdown',e=>{ui.drag={x:e.clientX,offset:ui.offset};$('chart').setPointerCapture(e.pointerId)});
$('chart').addEventListener('pointerup',()=>{ui.drag=null});$('chart').addEventListener('pointercancel',()=>{ui.drag=null});$('chart').addEventListener('pointerleave',()=>{ui.pointer=null;queueDraw()});
$('chart').addEventListener('wheel',e=>{e.preventDefault();ui.count=Math.max(20,Math.min(400,ui.count+(e.deltaY>0?10:-10)));queueDraw()},{passive:false});
new ResizeObserver(queueDraw).observe($('chartWrap'));
window.addEventListener('online',poll);window.addEventListener('offline',renderStatus);
const tab=new URLSearchParams(location.search).get('tab');if(['positions','evidence','rules','archive','health','account'].includes(tab))ui.tab=tab;
poll();connect();account();archive();setInterval(poll,5000);setInterval(account,30000);setInterval(renderStatus,1000);
