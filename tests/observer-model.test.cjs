const test=require('node:test'),assert=require('node:assert/strict');
const M=require('../web/assets/observer-model.js');
test('countdown clamps to zero and shows H4 clock',()=>{assert.equal(M.duration(14400000),'04:00:00');assert.equal(M.duration(-10),'00:00:00');});
test('browser cannot keep LIVE after failure or aging',()=>{const s={status:'OBSERVING_LIVE_MARKET'},r={status:'OBSERVING',price_fresh:true,price_age_ms:100,h4_success_age_ms:100};assert.ok(M.live(s,r,1,true));assert.ok(!M.live(s,r,1,false));assert.ok(!M.live(s,r,10000,true));assert.ok(!M.live(s,{...r,status:'PAUSED_STALE_DATA'},1,true));});
test('warmup is observation and pending is not a live fill',()=>{assert.match(M.reason({status:'OBSERVING'}),/OBSERVASI AKTIF/);assert.match(M.reason({status:'OBSERVING',pending_model:{direction:1}}),/belum merupakan fill live/);assert.match(M.reason({status:'PAUSED_INTEGRITY_ERROR'}),/TERTAHAN/);});
test('missing net is not rendered as zero',()=>{assert.equal(M.number(null),'—');assert.equal(M.number(NaN),'—');assert.equal(M.number(0),'0.00');});
