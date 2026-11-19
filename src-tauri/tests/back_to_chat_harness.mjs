import fs from 'node:fs'; import assert from 'node:assert/strict';
const src = fs.readFileSync(process.argv[2],'utf8').replace(/__HOST__/g,'evolvepreneuriq.app');
function run(host,path){
  const added=[]; const g=globalThis; g.window=g; g.location={hostname:host,pathname:path};
  g.document={getElementById:(id)=>added.find(e=>e.id===id)||null,body:{appendChild:(e)=>added.push(e)},createElement:()=>({style:{},setAttribute(){}}),documentElement:{appendChild:(e)=>added.push(e)}};
  (0,eval)(src); return added;
}
let n=0; const ok=(m)=>{n++;console.log('ok',m)};
for (const p of ['/profile','/profile/edit','/notifications/preferences','/projects','/projects/12','/tasks','/tasks/5','/integrations/library']) { const a=run('evolvepreneuriq.app',p); assert.equal(a.length,1,p); assert.equal(a[0].href,'/chat/desktop'); ok('bar on '+p); }
for (const p of ['/chat','/chat/desktop','/chat/desktop/live-agent','/login','/2fa/check','/logout','/forgot','/reset/x','/register']) { assert.equal(run('evolvepreneuriq.app',p).length,0,p); ok('no bar on '+p); }
assert.equal(run('evil.test','/profile').length,0); ok('no bar on another host');
console.log(n,'passed');
