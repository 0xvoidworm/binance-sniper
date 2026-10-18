/* No trading controls. All report-derived text goes through textContent. */
(() => {
  'use strict';
  const $=id=>document.getElementById(id), M=window.TesterModel;
  let summary=null, selected=null, archive=false, page=0, requestVersion=0, points=[];
  const put=(id,value)=>{$(id).textContent=value??'—';};
  const warningText={
    BALANCE_CURVE_IS_NOT_EQUITY:'Grafik saldo terealisasi, bukan equity termasuk posisi terbuka.',
    HISTORY_QUALITY_IS_NOT_REAL_TICK_COVERAGE:'History quality bukan persentase cakupan real ticks.',
    REPORT_CONSISTENCY_IS_NOT_AUTHENTICITY_OR_PROFITABILITY:'Rekonsiliasi laporan bukan bukti keaslian laporan atau jaminan profit.',
    ZERO_REPORTED_COMMISSION_NOT_VERIFIED_BROKER_COST:'Komisi laporan = 0; belum berarti biaya broker sesungguhnya sudah terwakili.',
    REPORT_INPUTS_NOT_AVAILABLE:'Input EA tidak tercantum di laporan.',SERVER_BUILD_NOT_AVAILABLE:'Server/build MT5 tidak tercantum.'
  };
  async function get(url){const response=await fetch(url,{cache:'no-store',signal:AbortSignal.timeout(10000)});if(!response.ok)throw Error(`HTTP ${response.status}`);return response.json();}
  function option(select,value,label){const el=document.createElement('option');el.value=value;el.textContent=label;select.append(el);}
  function runLabel(run){return `${run.context.run_label||run.symbol} · ${run.start}–${run.end} · ${run.id.slice(0,8)}`;}
  function provenance(rows){$('provenance').replaceChildren();for(const [key,value] of rows){const dt=document.createElement('dt'),dd=document.createElement('dd');dt.textContent=key;dd.textContent=value??'Tidak tersedia';$('provenance').append(dt,dd);}}
  function ledger(){
    const data=M.pageRows(selected?.report.deals||[],page);page=data.index;$('deals').replaceChildren();
    for(const row of data.rows){const tr=document.createElement('tr');for(const value of [row.time.replace('T',' '),row.deal,row.symbol||'—',`${row.type} ${row.direction}`,M.format(row.volume,2),M.format(row.price,5),M.format(row.commission),M.format(row.swap),M.format(row.profit),M.format(row.balance)]){const td=document.createElement('td');td.textContent=value;tr.append(td);}$('deals').append(tr);}
    if(!data.total){const tr=document.createElement('tr'),td=document.createElement('td');td.colSpan=10;td.textContent='Belum ada laporan terpilih.';tr.append(td);$('deals').append(tr);}
    put('dealCount',`${data.total} BARIS · TERMASUK DEPOSIT AWAL`);put('page',data.total?`${data.index+1} / ${data.pages}`:'—');$('previous').disabled=page===0;$('next').disabled=page+1>=data.pages;
  }
  function draw(){
    const canvas=$('balanceChart');if(!points.length||canvas.hidden)return;
    const {width:w,height:h}=canvas.getBoundingClientRect();if(!w||!h)return;
    const ratio=window.devicePixelRatio||1;canvas.width=Math.round(w*ratio);canvas.height=Math.round(h*ratio);
    const ctx=canvas.getContext('2d');ctx.scale(ratio,ratio);ctx.clearRect(0,0,w,h);
    const left=78,right=w-22,top=22,bottom=h-32;let low=Infinity,high=-Infinity;
    for(const p of points){low=Math.min(low,p.balance);high=Math.max(high,p.balance);}
    const padding=Math.max((high-low)*.12,Math.abs(high)*.0001,.01);low-=padding;high+=padding;
    // Z is used only for relative chart spacing, never displayed as a UTC conversion.
    const time=p=>Date.parse(p.time+'Z'),first=time(points[0]),last=time(points.at(-1));
    const x=p=>left+(time(p)-first)/Math.max(1,last-first)*(right-left),y=v=>bottom-(v-low)/(high-low)*(bottom-top);
    ctx.font='10px Consolas, monospace';ctx.lineWidth=1;
    for(let i=0;i<5;i++){const v=low+(high-low)*i/4,at=y(v);ctx.strokeStyle='#26303d';ctx.beginPath();ctx.moveTo(left,at);ctx.lineTo(right,at);ctx.stroke();ctx.fillStyle='#8b98ac';ctx.textAlign='right';ctx.fillText(M.format(v),left-10,at+4);}
    ctx.fillStyle='#8b98ac';ctx.textAlign='left';ctx.fillText(points[0].time.slice(0,10),left,h-12);ctx.textAlign='right';ctx.fillText(points.at(-1).time.slice(0,10),right,h-12);
    ctx.strokeStyle=points.at(-1).balance>=points[0].balance?'#5bc7b3':'#ee8f95';ctx.lineWidth=1.7;ctx.beginPath();ctx.moveTo(x(points[0]),y(points[0].balance));
    for(let i=1;i<points.length;i++){ctx.lineTo(x(points[i]),y(points[i-1].balance));ctx.lineTo(x(points[i]),y(points[i].balance));}ctx.stroke();
    if(points.length===1){ctx.fillStyle=ctx.strokeStyle;ctx.beginPath();ctx.arc(left,y(points[0].balance),3,0,Math.PI*2);ctx.fill();}
  }
  function render(){
    const r=selected?.report,m=r?.metrics||{};points=M.balancePoints(selected);
    $('archiveBanner').hidden=!archive;$('invalidBanner').hidden=!r||r.validation==='REPORT_RECONCILED';
    put('net',M.format(m.net_profit));$('net').className=M.number(m.net_profit)?(m.net_profit<0?'negative':'positive'):'';
    put('currency',r?`${r.currency} · deposit ${M.format(m.initial_deposit)}`:'Belum ada hasil');put('pf',M.format(m.profit_factor));put('dd',m.equity_dd_maximal);
    put('trades',r?`${m.total_trades} / ${m.total_deals}`:'—');put('period',r?`${r.start} → ${r.end}`:'Periode belum tersedia');put('validation',r?.validation||'NO REPORT');put('chartUnit',r?`${r.currency} / BROKER TIME`:'TIDAK ADA DATA');
    $('balanceChart').hidden=!points.length;$('emptyChart').hidden=!!points.length;
    $('emptyChart').querySelector('h3').textContent=r?'Grafik tidak tersedia untuk laporan ini.':archive?'Pilih laporan arsip untuk memeriksa pipeline.':'Belum ada hasil uji EA ini.';
    put('chartReadout',r?'Saldo setelah deal · waktu broker, tanpa konversi WIB':'Waktu broker belum dikonversi ke WIB.');
    const warnings=[];
    if(r){
      provenance([['Run ID',selected.id],['Expert / versi',`${r.expert} / ${selected.context.version||'tidak tercatat'}`],['Server / build',r.server_build],['Periode / TF',`${r.start}–${r.end} / ${r.timeframe}`],['Simbol diperdagangkan',r.traded_symbols.join(', ')||'Tidak ada'],['Model tester',selected.context.tester_model||'Tidak tercatat'],['Timezone broker',selected.context.broker_timezone||'Tidak tercatat'],['Biaya / delay',`${selected.context.cost_model||'Tidak tercatat'} / ${selected.context.execution_delay||'Tidak tercatat'}`],['History quality',m.history_quality],['Komisi / swap',`${M.format(m.commission)} / ${M.format(m.swap)} ${r.currency}`],['Report SHA-256',r.source_sha256],['Input SHA-256',r.inputs_sha256],['Binary SHA-256',selected.artifacts.binary||'Tidak tersedia'],['Sumber konteks','Dinyatakan operator; bukan attestasi independen']]);
      warnings.push(...r.errors,...r.warnings,...(selected.repeat_missing.length?[`Repeat belum lengkap: ${selected.repeat_missing.join(', ')}`]:[]));
    }else{
      provenance([['Hasil kandidat lokal',summary?.candidate_runs??'—'],['Status',summary?.status||'Belum tersedia'],['Candle / zona / equity curve','Tidak tersedia dalam laporan HTML'],['Trading live','Tidak ada kontrol eksekusi']]);
      warnings.push('Riwayat publik bukan hasil uji mesin ini di akun lo.','Uji kompatibilitas FBS tidak sama dengan replikasi IC Markets.','Laporan valid tidak otomatis berarti strategi layak live.');
    }
    $('warnings').replaceChildren();for(const message of warnings){const li=document.createElement('li');li.textContent=warningText[message]||message;$('warnings').append(li);}
    $('compareRun').replaceChildren();option($('compareRun'),'','Pilih run pembanding');
    for(const run of summary?.runs||[]){if(run.id!==selected?.id&&M.isCandidate(run)!==archive)option($('compareRun'),run.id,runLabel(run));}
    $('compare').disabled=true;put('comparison','Bandingkan laporan berbeda dengan binary, setting, periode, data, biaya, dan kondisi tester sepadan. Perubahan profit bukan otomatis bukti perbaikan strategi.');ledger();draw();
  }
  async function choose(id){
    const version=++requestVersion;selected=null;page=0;render();if(!id)return;
    try{const run=await get(`/api/tester/runs/${encodeURIComponent(id)}`);if(version!==requestVersion)return;selected=run;render();}
    catch(error){if(version===requestVersion){put('connection',`GAGAL MEMUAT RUN · ${error.message}`);put('validation','UNAVAILABLE');}}
  }
  function populate(preserve=true){
    const previous=preserve?$('run').value:'';$('run').replaceChildren();const runs=(summary?.runs||[]).filter(r=>M.isCandidate(r)!==archive);
    if(!runs.length)option($('run'),'','Belum ada laporan untuk sumber ini');for(const run of runs)option($('run'),run.id,runLabel(run));
    if(runs.some(r=>r.id===previous))$('run').value=previous;return choose($('run').value);
  }
  async function refresh(){
    const button=$('refresh');button.disabled=true;put('connection','MEMBACA BUKTI LOKAL');
    try{
      summary=await get('/api/tester');put('candidateCount',summary.candidate_runs);put('legacyCount',summary.legacy_runs);
      put('headline',summary.candidate_runs?'Laporan kandidat tersedia untuk diperiksa':'Menunggu laporan EA asli');
      put('headlineNote',summary.candidate_runs?'Hasil tester lokal, bukan posisi akun atau jaminan hasil live.':'Pipeline siap menerima laporan. Demo EA belum memiliki hasil uji lokal yang tercatat.');
      put('pipeReports',`${summary.candidate_runs} kandidat · ${summary.legacy_runs} arsip`);
      const valid=summary.runs.filter(r=>M.isCandidate(r)&&r.validation==='REPORT_RECONCILED').length;put('pipeValid',`${valid} laporan kandidat cocok`);put('pipeRepeat',summary.candidate_runs>=2?'Pilih 2 run untuk diperiksa':'Butuh ≥2 run sepadan');
      put('connection','EVIDENCE CONNECTED');put('updated',`Snapshot bukti ${new Date().toLocaleTimeString('id-ID',{timeZone:'Asia/Jakarta'})} WIB · refresh manual`);await populate();
    }catch(error){
      requestVersion++;summary=null;selected=null;render();put('connection',`BUKTI TIDAK TERSEDIA · ${error.message}`);put('headline','Bukti lokal belum bisa dibaca');put('headlineNote','Hasil dikosongkan; tidak memakai snapshot lama sebagai data terkini.');
      for(const id of ['candidateCount','legacyCount','pipeReports','pipeValid','pipeRepeat'])put(id,'—');$('run').replaceChildren();option($('run'),'','Bukti tidak tersedia');
    }finally{button.disabled=false;}
  }
  $('refresh').addEventListener('click',refresh);$('run').addEventListener('change',()=>choose($('run').value));
  for(const [id,value] of [['candidateTab',false],['legacyTab',true]])$(id).addEventListener('click',()=>{archive=value;$('candidateTab').classList.toggle('selected',!archive);$('legacyTab').classList.toggle('selected',archive);populate(false);});
  $('previous').addEventListener('click',()=>{page--;ledger();});$('next').addEventListener('click',()=>{page++;ledger();});$('compareRun').addEventListener('change',()=>{$('compare').disabled=!selected||!$('compareRun').value;});
  $('compare').addEventListener('click',async()=>{
    const left=selected?.id,right=$('compareRun').value,version=requestVersion;if(!left||!right)return;$('compare').disabled=true;
    try{const result=await get(`/api/tester/compare?left=${encodeURIComponent(left)}&right=${encodeURIComponent(right)}`);if(version!==requestVersion||right!==$('compareRun').value)return;put('comparison',`${result.status} · ${result.reasons.join(' / ')||'Angka dan urutan deal dibandingkan.'} · Δ net kanan−kiri ${M.format(result.delta_right_minus_left.net_profit)} ${selected.report.currency}. Kesepadanan konteks bukan bukti independensi atau profitabilitas.`);}
    catch(error){if(version===requestVersion&&right===$('compareRun').value)put('comparison',`Perbandingan gagal: ${error.message}`);}
    finally{if(version===requestVersion)$('compare').disabled=false;}
  });
  $('balanceChart').addEventListener('mousemove',event=>{
    if(!points.length)return;const rect=event.currentTarget.getBoundingClientRect(),fraction=Math.max(0,Math.min(1,(event.clientX-rect.left-78)/(rect.width-100)));
    const first=Date.parse(points[0].time+'Z'),last=Date.parse(points.at(-1).time+'Z'),target=first+fraction*(last-first);let lo=0,hi=points.length-1;
    while(lo<hi){const mid=Math.ceil((lo+hi)/2);if(Date.parse(points[mid].time+'Z')<=target)lo=mid;else hi=mid-1;}
    const p=points[lo];put('chartReadout',`${p.time.replace('T',' ')} broker · deal ${p.deal} · saldo ${M.format(p.balance)} ${selected.report.currency}`);
  });
  new ResizeObserver(draw).observe($('balanceChart').parentElement);refresh();
})();
