"""Generate a passive, standalone terminal viewer from stored engine projections."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from binance_sniper.round01.data import sha, write_json


HTML = r'''<!doctype html>
<html lang="id"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<title>ROUND 01 · Channel Research Terminal</title>
<style>
:root{color-scheme:dark;font-family:Consolas,"Cascadia Code",monospace;font-size:13px;background:#090f16;color:#c8d3df}
*{box-sizing:border-box}body{margin:0}header{border-bottom:1px solid #293a49;padding:18px 22px;background:#101a24}h1{font-size:18px;font-weight:600;letter-spacing:1px;margin:0 0 9px}h2{font-size:13px;font-weight:400;color:#85a6ba;margin:0 0 12px}.muted{color:#8a9daf}.tag{color:#efb75a}main{padding:18px 22px;max-width:1600px;margin:auto}button,select{background:#14232f;color:#d5e4ef;border:1px solid #365066;border-radius:2px;padding:8px;font:inherit;cursor:pointer}button:focus,select:focus{outline:2px solid #48cfc2}button:hover{background:#234253}button:disabled{opacity:.4;cursor:default}.toolbar{display:flex;flex-wrap:wrap;gap:8px;align-items:center;margin:14px 0}.panel{background:#0e1822;border:1px solid #253a4d;padding:14px;margin-bottom:14px}.cards{display:grid;grid-template-columns:repeat(6,minmax(100px,1fr));gap:10px}.card{background:#121f2b;padding:11px;line-height:1.8}.value{display:block;color:#60ddd0;font-size:17px}.negative{color:#ff818b}.grid{display:grid;grid-template-columns:2fr 1fr;gap:14px}.detail{line-height:1.85;overflow-wrap:anywhere}canvas{width:100%;height:370px;display:block}#runId{font-size:11px;overflow-wrap:anywhere}input[type=range]{flex:1;min-width:150px;accent-color:#54c9bc}table{width:100%;border-collapse:collapse;font-size:11px}td,th{padding:9px 6px;text-align:right;border-bottom:1px solid #213141}th:first-child,td:first-child{text-align:left}.scroll{overflow-x:auto}.notice{border-left:3px solid #b78942;padding:10px 13px;background:#1d211f;line-height:1.7;margin-top:12px}a{color:#80cce6}#error{color:#ff818b}#reason{color:#70d3c9}#clock{color:#dceaf6}#eventLog{white-space:pre-wrap;line-height:1.7;font-size:12px}.status{color:#f2bb65}footer{padding:5px 0 20px;color:#8a9daf;font-size:11px}@media(max-width:900px){.grid{grid-template-columns:1fr}.cards{grid-template-columns:repeat(3,1fr)}main,header{padding:13px}canvas{height:300px}}
</style>
<header><h1>ROUND 01 / CHANNEL RESEARCH TERMINAL</h1><div class="tag">HISTORICAL_REPLAY · EKSPLORATIF · SIMULASI TANPA ORDER</div></header>
<main><div id="error" role="alert"></div><div class="panel"><h2>KEPUTUSAN PUTARAN</h2><strong id="decision" class="status">Memuat bukti...</strong><div class="notice">Indeks eksposur riset, bukan return/equity akun futures. Nominal 0.5 per simbol; fee, slippage dan funding sesuai model. Data sudah pernah dilihat. Hasil ini milik kode mandiri; hasil vendor Jimmy tidak ditampilkan sebagai hasil mesin ini.</div></div>
<div class="toolbar"><label>Run <select id="run" aria-label="Run replay"></select></label><label>Simbol <select id="symbol" aria-label="Simbol"><option>BTCUSDT</option><option>ETHUSDT</option></select></label></div>
<div class="panel"><h2>RINGKASAN SELURUH RUN / HINDSIGHT · BTC + ETH · BASE</h2><div class="cards" id="metrics"></div><p class="muted" id="costs"></p><div id="runId"></div></div>
<div class="toolbar"><button id="first">Awal</button><button id="prev">← Bar</button><button id="play">Putar</button><button id="next">Bar →</button><button id="event">Event berikut</button><input id="seek" aria-label="Bar replay" type="range" min="0" value="0"><span id="step"></span></div>
<div class="grid"><div class="panel"><h2>H4 CLOSED / HANYA PREFIX YANG SUDAH TERSEDIA</h2><div id="clock"></div><canvas id="chart" aria-label="Grafik harga dan stop dari engine"></canvas><div class="muted">Candle naik hijau, turun merah · stop aktif amber · entry cyan · kanal entry abu-abu. Waktu UTC.</div></div><div class="panel detail"><h2>STATE DARI ENGINE PYTHON</h2><div id="reason"></div><div id="state"></div><h2 style="margin-top:20px">EVENT PADA BAR INI</h2><div id="eventLog"></div></div></div>
<div class="panel scroll"><h2>TRADE TERTUTUP HINGGA BAR REPLAY / KONTRIBUSI INDEKS (BPS)</h2><table><thead><tr><th>Trade / arah</th><th>Entry</th><th>Exit</th><th>Alasan</th><th>Gross</th><th>Fee</th><th>Slip</th><th>Funding</th><th>Net</th></tr></thead><tbody id="trades"></tbody></table></div>
<footer id="provenance"></footer></main>
<script>
'use strict';
const $=id=>document.getElementById(id),fmt=(x,n=4)=>x==null?'—':Number(x).toFixed(n),bps=x=>fmt(x*10000,2),time=x=>new Date(x).toISOString().replace('T',' ').replace('.000Z',' UTC');
let evidence,runs,selected,frames=[],index=0,timer=null;
function text(id,value){$(id).textContent=value}
function stop(){if(timer)clearInterval(timer);timer=null;text('play','Putar')}
function choose(){stop();selected=runs[Number($('run').value)];frames=selected.frames.filter(f=>f.symbol===$('symbol').value);index=0;$('seek').max=frames.length-1;renderSummary();render()}
function renderSummary(){const m=selected.metrics,items=[['Net indeks / bps',bps(m.net)],['Gross / bps',bps(m.gross)],['PF net',fmt(m.pf,3)],['DD indeks / %',fmt(m.max_mtm_drawdown_fraction*100,2)],['Trade / hari',fmt(m.trade_per_day,3)],['Hari tanpa entry',m.days_without_entry+'/'+m.calendar_days]];$('metrics').replaceChildren(...items.map(([label,value])=>{const div=document.createElement('div');div.className='card';div.append(document.createTextNode(label));const span=document.createElement('span');span.className='value';span.textContent=value;div.append(span);return div}));text('costs',`Fee ${bps(m.fee_cost)} · slippage ${bps(m.slippage_cost)} · funding ${bps(m.funding_cost)} bps indeks | trade ${m.trade_count} | STRESS net ${bps(selected.stress.net)} bps / PF ${fmt(selected.stress.pf,3)}`);text('runId',`RUN ${selected.run_id} / ${selected.partition} / SOURCE ${evidence.identity.source_sha256}`)}
function line(ctx,price,label,color,y,width){if(price==null)return;ctx.strokeStyle=color;ctx.setLineDash([5,5]);ctx.beginPath();ctx.moveTo(8,y(price));ctx.lineTo(width-68,y(price));ctx.stroke();ctx.setLineDash([]);ctx.fillStyle=color;ctx.fillText(label+' '+fmt(price,2),12,y(price)-5)}
function draw(){const cv=$('chart'),rect=cv.getBoundingClientRect(),dpr=devicePixelRatio||1;cv.width=Math.round(rect.width*dpr);cv.height=Math.round(rect.height*dpr);const ctx=cv.getContext('2d');ctx.scale(dpr,dpr);const w=rect.width,h=rect.height;ctx.clearRect(0,0,w,h);const visible=frames.slice(Math.max(0,index-79),index+1),f=frames[index],pos=f.position;const prices=visible.flatMap(x=>[x.bar.low,x.bar.high]);if(f.stop_applied_this_bar!=null)prices.push(f.stop_applied_this_bar);if(pos)prices.push(pos.raw_entry);let lo=Math.min(...prices),hi=Math.max(...prices),pad=(hi-lo)*.08||1;lo-=pad;hi+=pad;const y=p=>h-28-(p-lo)/(hi-lo)*(h-48),dx=(w-78)/Math.max(visible.length,20);ctx.font='11px Consolas';for(let i=0;i<=4;i++){const p=lo+(hi-lo)*i/4;ctx.strokeStyle='#203444';ctx.beginPath();ctx.moveTo(8,y(p));ctx.lineTo(w-65,y(p));ctx.stroke();ctx.fillStyle='#8dabbc';ctx.fillText(fmt(p,1),w-60,y(p)+4)}visible.forEach((f,i)=>{const b=f.bar,x=12+(i+.5)*dx;ctx.strokeStyle=ctx.fillStyle=b.close>=b.open?'#55cdbb':'#f07987';ctx.beginPath();ctx.moveTo(x,y(b.high));ctx.lineTo(x,y(b.low));ctx.stroke();ctx.fillRect(x-dx*.3,Math.min(y(b.open),y(b.close)),Math.max(1,dx*.6),Math.max(1,Math.abs(y(b.close)-y(b.open))))});line(ctx,f.stop_applied_this_bar,'STOP AKTIF','#e8b862',y,w);if(pos)line(ctx,pos.raw_entry,'ENTRY RAW','#60dfe4',y,w);if(f.channels_known_before_bar){line(ctx,f.channels_known_before_bar.entry_high,'KANAL HIGH','#637b8d',y,w);line(ctx,f.channels_known_before_bar.entry_low,'KANAL LOW','#637b8d',y,w)}ctx.fillStyle='#8dabbc';ctx.fillText(time(visible[0].bar.open_time).slice(0,16),10,h-7);ctx.fillText(time(f.bar.open_time).slice(0,16),Math.max(10,w-195),h-7)}
function render(){const f=frames[index];if(!f)return;text('clock',time(f.data_available_at));text('step',`${index+1} / ${frames.length}`);$('seek').value=index;text('reason',f.reason);const p=f.position;text('state',`Simbol: ${f.symbol}\nPosisi: ${p?(p.direction===1?'LONG':'SHORT'):'FLAT'}\nEntry fill: ${p?fmt(p.entry_fill):'—'}\nStop bar ini: ${fmt(f.stop_applied_this_bar)}\nStop bar berikut: ${fmt(f.stop_for_next_bar)}\nMTM indeks simbol: ${bps(f.index_pnl_mtm)} bps\nBiaya model: lengkap\nModel akun: belum lengkap`);$('state').style.whiteSpace='pre-line';text('eventLog',f.events.length?f.events.map(e=>`${e.kind} | ${e.exit_reason||e.reason||''}${e.net!=null?' | net '+bps(e.net)+' bps':''}\n${e.signal_at?'signal '+time(e.signal_at):''}${e.entry_at?' / fill '+time(e.entry_at):''}`).join('\n\n'):'Tidak ada event baru.');const trades=selected.trades.filter(t=>t.symbol===f.symbol&&t.exit_recorded_at<=f.decision_at).slice(-10);$('trades').replaceChildren(...trades.map(t=>{const tr=document.createElement('tr');[t.trade_id+' / '+(t.direction===1?'L':'S'),fmt(t.entry_fill,2),fmt(t.exit_fill,2),t.exit_reason,bps(t.gross),bps(t.fee_cost),bps(t.slippage_cost),bps(t.funding_cost),bps(t.net)].forEach(v=>{const td=document.createElement('td');td.textContent=v;tr.append(td)});return tr}));$('prev').disabled=index===0;$('next').disabled=index===frames.length-1;draw();window.round01Visible={run_id:selected.run_id,index,symbol:f.symbol,decision_at:f.decision_at,reason:f.reason,events:f.events,stop:f.stop_applied_this_bar,position:f.position}}
$('first').onclick=()=>{stop();index=0;render()};$('prev').onclick=()=>{stop();index=Math.max(0,index-1);render()};$('next').onclick=()=>{stop();index=Math.min(frames.length-1,index+1);render()};$('event').onclick=()=>{stop();const n=frames.findIndex((f,i)=>i>index&&f.events.length);if(n>=0){index=n;render()}};$('seek').oninput=()=>{stop();index=Number($('seek').value);render()};$('run').onchange=choose;$('symbol').onchange=choose;$('play').onclick=()=>{if(timer){stop();return}text('play','Jeda');timer=setInterval(()=>{if(index>=frames.length-1){stop();return}index++;render()},250)};window.addEventListener('resize',()=>{if(frames.length)draw()});
fetch('evidence.json').then(r=>{if(!r.ok)throw Error('Bukti tidak tersedia');return r.json()}).then(data=>{evidence=data;runs=data.runs;text('decision',data.decision+' · FINAL: '+data.final_status);text('provenance',`PROTOCOL ${data.identity.protocol_sha256} | DATA ${data.identity.data_sha256} | Semua harga, stop, event dan alasan berasal dari frame engine tersimpan. Viewer standalone; observer strategi baru tidak diaktifkan.`);runs.forEach((r,i)=>{const op=document.createElement('option');op.value=i;op.textContent=r.config.name+' / '+r.partition;$('run').append(op)});choose()}).catch(e=>text('error',e.message));
</script></html>'''


def build_viewer(study: Path, output: Path):
    summary = json.loads((study / "SUMMARY.json").read_text(encoding="utf-8"))
    runs = []
    for config in summary["reports"].values():
        for pair in config.values():
            base = pair["BASE"]
            if sha(study / base["file"]) != base["sha256"]:
                raise ValueError("TRIAL_CHANGED")
            r = json.loads((study / base["file"]).read_text(encoding="utf-8"))
            runs.append({k: r[k] for k in ("run_id", "config", "partition", "metrics", "frames", "trades")})
            runs[-1]["stress"] = pair["STRESS"]
    if summary["final"]:
        pair = summary["final"]
        r = json.loads((study / pair["BASE"]["file"]).read_text(encoding="utf-8"))
        runs.append({k: r[k] for k in ("run_id", "config", "partition", "metrics", "frames", "trades")})
        runs[-1]["stress"] = pair["STRESS"]
    output.mkdir(parents=True, exist_ok=False)
    write_json(output / "evidence.json", {k: summary[k] for k in ("decision", "final_status", "identity")} | {"runs": runs})
    (output / "index.html").write_text(HTML, encoding="utf-8")
    return {"runs": len(runs), "frames": sum(len(r["frames"]) for r in runs), "html_sha256": sha(output / "index.html"),
            "evidence_sha256": sha(output / "evidence.json")}


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--study", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    print(json.dumps(build_viewer(args.study, args.output)))
