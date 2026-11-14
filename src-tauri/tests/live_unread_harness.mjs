import fs from 'node:fs'; import assert from 'node:assert/strict';
const src = fs.readFileSync(process.argv[2],'utf8').replace(/__HOST__/g,'evolvepreneuriq.app');
const results=[]; const ok=(n)=>{results.push(n);console.log('ok',n)};
function setup(routes, path='/chat/desktop'){
  const reported=[]; const g=globalThis;
  g.window=g; g.location={hostname:'evolvepreneuriq.app',pathname:path};
  delete g.__evolve_live_started; delete g.__evolve_live_ok;
  g.__TAURI_INTERNALS__={invoke:(c,a)=>{reported.push([c,a]);return Promise.resolve();}};
  const calls=[];
  g.fetch=async(url,opt={})=>{calls.push([url,opt]);const h=routes(url,opt);return h;};
  return {reported,calls};
}
const sse=(chunks,hold=false)=>{const enc=new TextEncoder();return {ok:true,status:200,body:new ReadableStream({async start(c){for(const ch of chunks){c.enqueue(enc.encode(ch));await new Promise(r=>setTimeout(r,5));}if(!hold)c.close();}})};};
const key=(o={})=>({ok:true,status:200,json:async()=>({url:'https://hub.example/.well-known/mercure',topic:'chat/user/42',token:'T0K',expires_at:new Date(Date.now()+8*3600e3).toISOString(),...o})});
const run=()=>{(0,eval)(src);};
const wait=(ms)=>new Promise(r=>setTimeout(r,ms));
// 1 push updates the number, header sent, live flag on while open
{ const s=setup((u)=>u.includes('live-auth')?key():sse(['data: {"type":"unread","channel_id":"a","unread":1,"total":7}\n\n'],true)); run(); await wait(120);
  assert.deepEqual(s.reported,[['report_unread',{count:7}]]); const hub=s.calls.find(c=>c[0].includes('topic='));
  assert.equal(hub[1].headers.Authorization,'Bearer T0K'); assert.ok(hub[0].includes('topic=chat%2Fuser%2F42')); assert.equal(globalThis.__evolve_live_ok,true); ok('push sets the number, bearer header, live flag on'); }
// 2 split chunks + CRLF + multiple events
{ const s=setup((u)=>u.includes('live-auth')?key():sse(['data: {"type":"unread","tot','al":3}\r\n\r\nda','ta: {"type":"unread","total":0}\n\n'])); run(); await wait(150);
  assert.deepEqual(s.reported.map(r=>r[1].count),[3,0]); ok('split chunks, CRLF, two events'); }
// 3 ignore: no total, wrong type, negative, fractional, string
{ const ev=['{"type":"unread"}','{"type":"other","total":5}','{"type":"unread","total":-1}','{"type":"unread","total":1.5}','{"type":"unread","total":"9"}','not json'];
  const s=setup((u)=>u.includes('live-auth')?key():sse(ev.map(e=>'data: '+e+'\n\n'))); run(); await wait(150); assert.deepEqual(s.reported,[]); ok('events without a usable total are ignored'); }
// 4 flag off (404): no stream; 401: stops; bad topic/url: no stream
for (const [name,resp] of [['404',{ok:false,status:404}],['401',{ok:false,status:401}],['bad topic',key({topic:'chat/user/../x'})],['http url',key({url:'http://hub.example/m'})]]) {
  const s=setup((u)=>resp); run(); await wait(80); assert.ok(!s.calls.some(c=>c[0].includes('topic='))); assert.equal(globalThis.__evolve_live_ok,false); ok('no stream on '+name); }
// 5 sign-in page and wrong host and no tauri: nothing runs
{ const s=setup(()=>key(),'/login'); run(); await wait(40); assert.equal(s.calls.length,0); ok('sign-in page: no request'); }
{ const s=setup(()=>key()); location.hostname='evil.test'; run(); await wait(40); assert.equal(s.calls.length,0); ok('other host: no request'); }
// 6 live flag drops when the stream ends
{ const s=setup((u)=>u.includes('live-auth')?key():sse(['data: {"type":"unread","total":1}\n\n'])); run(); await wait(150); assert.equal(globalThis.__evolve_live_ok,false); ok('flag off after the stream ends (poll resumes)'); }
// 7 started once per page
{ const s=setup((u)=>u.includes('live-auth')?key():sse([],true)); run(); run(); await wait(80); assert.equal(s.calls.filter(c=>c[0].includes('live-auth')).length,1); ok('runs once per page'); }
console.log(results.length,'passed'); process.exit(0);
