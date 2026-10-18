/* Read-only projection. No signal or execution logic lives in the browser. */
(() => {
  'use strict';
  const M=window.ObserverModel, $=id=>document.getElementById(id);
  let data=null,selected=null,received=0,connected=false;
  const put=(id,value)=>{$(id).textContent=value;};
  const date=ms=>ms?new Date(ms).toLocaleString('id-ID',{timeZone:'Asia/Jakarta',day:'2-digit',month:'short',hour:'2-digit',minute:'2-digit',second:'2-digit',hour12:false})+' WIB':'—';
  function render() {
    if(!data?.rows?.length){put('connection',connected?'OBSERVER STARTING':'KONEKSI TERPUTUS');return;}
    const row=data.rows.find(r=>r.id===selected)||data.rows[0];selected=row.id;
    const elapsed=Date.now()-received,now=data.server_now+elapsed,live=M.live(data,row,elapsed,connected);
    document.body.classList.toggle('stale',!live);
    put('connection',live?'● OBSERVASI AKTIF / REST':connected?'● DATA DEGRADED / PERIKSA STATUS':'● KONEKSI DASHBOARD TERPUTUS');
    put('network',`${data.network?.public_network_route||'—'} · WS bukan sumber aktif`);
    put('clock',date(now));put('started',date(data.started_at));
    put('modelName',`${row.config.name} · H4 · EKSPERIMENTAL`);put('symbol',row.symbol);put('price',M.number(row.price));
    put('quoteAge',`${row.price_fresh && connected && row.price_age_ms+elapsed<=9000?'Harga futures terkini':'Harga terakhir / stale'} · usia ${M.number((row.price_age_ms+elapsed)/1000,1)} dtk`);
    put('countdown',M.duration(row.next_close_at-now));put('nextClose',date(row.next_close_at));
    put('signal',!connected?'Koneksi ke dashboard terputus. Angka di bawah adalah snapshot lama.':M.reason(row));$('signal').classList.toggle('warning',!live);
    put('entryHigh',M.number(row.channels?.entry_high));put('entryLow',M.number(row.channels?.entry_low));
    const p=row.position_model_at_last_close;put('position',p?(p.direction===1?'LONG MODEL':'SHORT MODEL'):'FLAT');
    put('positionDetail',p?`Entry model ${M.number(p.entry_fill)}`:row.pending_model?'Sinyal pending · belum settle':'Belum ada posisi model');
    put('stop',M.number(row.stop_for_next_bar));put('atr',`ATR(20) · ${M.number(row.atr)}`);
    put('bars',`${row.observed_closed_bars} · warmup ${row.warmup_bars}`);put('delayed',row.delayed_closed_bars);
    put('chartInfo',`H4 · close terakhir ${date(row.last_closed_at)}`);put('tradeCount',`${row.trade_count} TRADE MODEL`);
    put('updated',`Snapshot ${date(data.updated_at)} · input H4 ${date(row.last_received_at)}`);
    put('health',`TASK ${data.task?.status||'—'} · H4 ${row.h4_transport||'—'}`);
    put('integrity',`ENGINE ${data.code_identity} · FUNDING: MISSING / NET LENGKAP: —`);
    const buttons=$('models');
    if(buttons.children.length!==data.rows.length){buttons.replaceChildren();for(const item of data.rows){const b=document.createElement('button');b.type='button';b.dataset.id=item.id;b.append(document.createElement('strong'),document.createElement('span'));b.onclick=()=>{selected=item.id;render();};buttons.append(b);}}
    [...buttons.children].forEach((b,i)=>{const item=data.rows[i];b.classList.toggle('selected',item.id===selected);b.setAttribute('aria-pressed',String(item.id===selected));b.children[0].textContent=item.symbol;b.children[1].textContent=`${item.config.entry_window}/${item.config.exit_window} · ${item.status==='OBSERVING'?'MENGAMATI':item.status}`;});
    const ledger=$('ledger');ledger.replaceChildren();
    if(!row.trades.length){const tr=document.createElement('tr'),td=document.createElement('td');td.colSpan=7;td.textContent='Belum ada trade model setelah aktivasi. Tidak membuat transaksi historis saat startup.';tr.append(td);ledger.append(tr);}
    for(const t of [...row.trades].reverse()){const tr=document.createElement('tr');for(const value of [date(t.recorded_at),t.direction===1?'LONG':'SHORT',M.number(t.entry),M.number(t.exit),t.exit_reason,M.number(t.index_after_fee_slippage_before_funding,6),t.origin]){const td=document.createElement('td');td.textContent=value;tr.append(td);}ledger.append(tr);}
    draw(row,live);
  }
  function draw(row,live){
    const canvas=$('marketChart'),bounds=canvas.getBoundingClientRect(),ratio=devicePixelRatio||1;
    canvas.width=Math.round(bounds.width*ratio);canvas.height=Math.round(bounds.height*ratio);
    const c=canvas.getContext('2d');c.scale(ratio,ratio);const w=bounds.width,h=bounds.height;
    const bars=row.chart||[];$('chartEmpty').hidden=bars.length>0;if(!bars.length)return;
    const levels=[['BUY close >',row.channels?.entry_high,'#5bc7b3'],['SELL close <',row.channels?.entry_low,'#ee8f95'],['STOP model',row.stop_for_next_bar,'#edb65c']].filter(x=>typeof x[1]==='number');
    let lo=Math.min(...bars.map(b=>b.low),...levels.map(x=>x[1])),hi=Math.max(...bars.map(b=>b.high),...levels.map(x=>x[1]));
    const pad=(hi-lo)*.08||1;lo-=pad;hi+=pad;const left=12,right=w-88,top=25,bottom=h-28;
    const y=p=>bottom-(p-lo)/(hi-lo)*(bottom-top),step=(right-left)/bars.length;
    c.font='10px Consolas,monospace';c.textAlign='left';
    for(let i=0;i<=5;i++){const p=lo+(hi-lo)*i/5,py=y(p);c.strokeStyle='#26303d';c.beginPath();c.moveTo(left,py);c.lineTo(right,py);c.stroke();c.fillStyle='#8b98ac';c.fillText(M.number(p),right+8,py+3);}
    bars.forEach((b,i)=>{const x=left+(i+.5)*step;c.globalAlpha=b.closed?1:.45;c.strokeStyle=c.fillStyle=b.close>=b.open?'#5bc7b3':'#ee8f95';c.beginPath();c.moveTo(x,y(b.high));c.lineTo(x,y(b.low));c.stroke();c.fillRect(x-Math.max(1,step*.57)/2,Math.min(y(b.open),y(b.close)),Math.max(1,step*.57),Math.max(1,Math.abs(y(b.open)-y(b.close))));});
    c.globalAlpha=1;for(const [label,p,color]of levels){c.strokeStyle=c.fillStyle=color;c.setLineDash([5,4]);c.beginPath();c.moveTo(left,y(p));c.lineTo(right,y(p));c.stroke();c.setLineDash([]);c.fillText(`${label} ${M.number(p)} · level sekarang`,left+8,y(p)-6);}
    c.fillStyle='#8b98ac';c.fillText(date(bars[0].open_time),left,h-9);c.textAlign='right';c.fillText('CANDLE BERJALAN / PREVIEW',right,h-9);c.textAlign='left';
    if(!live){c.fillStyle='rgba(11,14,19,.3)';c.fillRect(left,top,right-left,bottom-top);c.fillStyle='#edb65c';c.fillText('DATA BELUM LIVE / SNAPSHOT TERAKHIR',left+8,top+14);}
  }
  async function poll(){try{const response=await fetch('/api/observer',{cache:'no-store',signal:AbortSignal.timeout(5000)});if(!response.ok)throw Error('HTTP');const next=await response.json();if(!Array.isArray(next.rows))throw Error('SCHEMA');data=next;received=Date.now();connected=true;}catch{connected=false;}render();setTimeout(poll,2000);}
  new ResizeObserver(()=>{if(data)render();}).observe($('marketChart'));
  setInterval(render,1000);poll();
})();
