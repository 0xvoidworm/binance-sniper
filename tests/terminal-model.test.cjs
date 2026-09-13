const {test}=require('node:test');
const assert=require('node:assert/strict');
const M=require('../web/assets/terminal-model.js');
const live={transport_state:'LIVE_REST_FALLBACK'};
test('offline data cannot be repainted as live',()=>{
  assert.equal(M.connectionView(live,1000,2000,false,{wsOpen:true}).label,'DISCONNECTED');
  assert.equal(M.connectionView(live,1000,2000,true,{wsOpen:false,pollFailed:true}).label,'DISCONNECTED');
});
test('unchanged cached snapshot ages into stale regardless of open socket',()=>{
  assert.equal(M.connectionView(live,1000,10000,true,{wsOpen:true}).label,'STALE');
});
test('working HTTP snapshots remain usable when browser websocket disconnects',()=>{
  assert.equal(M.connectionView(live,1000,2000,true,{wsOpen:false,pollFailed:false}).label,'LIVE / REST');
});
test('replay symbol selection follows its own dataset',()=>{
  assert.equal(M.symbolFor({symbols:{BTCUSDT:{}}},'ETHUSDT'),'BTCUSDT');
  assert.equal(M.symbolFor({symbols:{}},'ETHUSDT'),'');
});
const data={symbols:{BTCUSDT:{zone:{zone_id:'current',state:'ARMED'}}},zones:{old:{zone_id:'old',state:'COMPLETED'}},candidates:[{symbol:'BTCUSDT',zone_id:'old',candidate_id:'old-c',score:1,decision_time:10},{symbol:'BTCUSDT',zone_id:'current',candidate_id:'new-c',score:.5,decision_time:20}],outcomes:{'old-c':{status:'COMPLETED'}}};
test('highest historical score cannot replace levels of the visible zone',()=>{
  const selected=M.selectedSetup(data,'BTCUSDT');
  assert.equal(selected.candidate.candidate_id,'new-c');
  assert.equal(selected.zone.zone_id,selected.candidate.zone_id);
  assert.equal(selected.isArchive,false);
});
test('no matching zone means no reference levels',()=>{
  const selected=M.selectedSetup({...data,symbols:{BTCUSDT:{zone:{zone_id:'not-found'}}}},'BTCUSDT');
  assert.equal(selected.candidate,null);
});
test('explicit archive selection ties zone, candidate and outcome together',()=>{
  const selected=M.selectedSetup(data,'BTCUSDT','old-c');
  assert.equal(selected.zone.zone_id,'old');assert.equal(selected.candidate.candidate_id,'old-c');
  assert.equal(selected.outcome.status,'COMPLETED');assert.equal(selected.isArchive,true);
});
test('zone distance and feed ages do not invent unavailable data',()=>{
  assert.equal(M.zoneDistance(null,{lower:100,upper:110}),null);
  assert.equal(M.zoneDistance(105,{lower:100,upper:110}).price,0);
  assert.equal(M.feedAge({preview_age_ms:null},2000,'preview_age_ms'),null);
  assert.equal(M.feedAge({success_age_ms:100},2000),2100);
});
