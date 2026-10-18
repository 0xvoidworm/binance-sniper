(function(root){
  'use strict';
  const number = value => typeof value === 'number' && Number.isFinite(value);
  const format = (value, digits=2) => number(value) ? value.toLocaleString('en-US',{minimumFractionDigits:digits,maximumFractionDigits:digits}) : '—';
  const isCandidate = run => run.context?.engine_id === 'jimmy-range-122237';
  function balancePoints(run){
    if(run?.report?.validation !== 'REPORT_RECONCILED' || !run.report.availability?.balance) return [];
    return (run.report.deals || []).filter(d=>number(d.balance) && typeof d.time === 'string').map(d=>({time:d.time,balance:d.balance,deal:d.deal}));
  }
  function pageRows(rows,page,size=25){
    const pages=Math.max(1,Math.ceil(rows.length/size));
    const index=Math.max(0,Math.min(pages-1,page));
    return {rows:rows.slice(index*size,(index+1)*size),index,pages,total:rows.length};
  }
  const api={number,format,isCandidate,balancePoints,pageRows};
  if(typeof module!=='undefined'&&module.exports) module.exports=api;
  root.TesterModel=api;
})(typeof globalThis!=='undefined'?globalThis:this);
