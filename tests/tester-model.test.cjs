const test=require('node:test'),assert=require('node:assert/strict');
const M=require('../web/assets/tester-model.js');
test('missing results are not displayed as zero profit',()=>{
  for(const value of [null,undefined,NaN,Infinity,'12'])assert.equal(M.format(value),'—');
  assert.equal(M.format(0),'0.00');assert.equal(M.format(-219.09),'-219.09');
});
test('only reconciled balance is plotted, never invented equity',()=>{
  assert.deepEqual(M.balancePoints(null),[]);
  const run={report:{validation:'RECONCILIATION_FAILED',availability:{balance:false},deals:[{time:'2026-01-01T00:00:00',balance:1000,deal:1}]}};
  assert.deepEqual(M.balancePoints(run),[]);
  run.report.validation='REPORT_RECONCILED';run.report.availability.balance=true;
  assert.deepEqual(M.balancePoints(run),[{time:'2026-01-01T00:00:00',balance:1000,deal:1}]);
  assert.equal(M.balancePoints(run)[0].equity,undefined);
});
test('legacy cannot become candidate by a display name',()=>{
  assert.equal(M.isCandidate({context:{engine_id:'legacy-sniperfx'},expert:'Range Breakout EA with Range Filters'}),false);
  assert.equal(M.isCandidate({context:{engine_id:'jimmy-range-122237'}}),true);
});
test('ledger page boundaries preserve all data',()=>{
  const rows=Array.from({length:53},(_,i)=>i);
  assert.deepEqual(M.pageRows(rows,2).rows,[50,51,52]);
  assert.equal(M.pageRows(rows,999).index,2);
  assert.equal(M.pageRows(rows,-1).index,0);
  assert.equal(M.pageRows([],10).total,0);
});
