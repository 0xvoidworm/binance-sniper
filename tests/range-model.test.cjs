const test=require('node:test');
const assert=require('node:assert/strict');
const R=require('../web/assets/range-model.js');
test('freshness cannot mistake a missing age for zero',()=>{assert.equal(R.fresh({data_age_ms:null}),false);assert.equal(R.fresh({data_age_ms:1000},25000),false);assert.equal(R.fresh({data_age_ms:1000},500),true)});
test('missing net funding never becomes zero all-cost profit',()=>{assert.deepEqual(R.positionValue({status:'CLOSED',net_pct:null,after_cost_pct:.2}),{value:.2,label:'Setelah fee + slippage / belum funding'});assert.equal(R.positionValue({status:'CLOSED',net_pct:0,after_cost_pct:.2}).value,0)});
test('preview chart dedupe does not mutate closed bars',()=>{const data={charts:{'BTCUSDT:5m':{closed:[{open_time:0,closed:true}],preview:{open_time:0,closed:false}}}};assert.equal(R.series(data,'BTCUSDT','5m').length,1);assert.equal(data.charts['BTCUSDT:5m'].closed[0].closed,true)});
test('holding has no four candle completion semantics',()=>{assert.equal(R.stateLabel('HOLDING'),'Pertahankan posisi');assert.match(R.explanation({reason:'HOLD_UNTIL_STOP_OR_SESSION_END'}),/tanpa TP tetap/);assert.equal(R.stageIndex('HOLDING'),2)});
test('ambiguous direction has an explicit non-profit explanation',()=>{assert.match(R.explanation({reason:'BOTH_BREAKOUTS_SAME_BAR'}),/tidak mengarang entry atau profit/)});
