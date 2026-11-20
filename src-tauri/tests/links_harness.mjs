// Runs src/links.js (the same-site rule and the back / forward / Home keys) in a fake page. node tests/links_harness.mjs src/links.js
import fs from 'node:fs'; import assert from 'node:assert/strict';
const src = fs.readFileSync(process.argv[2], 'utf8').replace(/__HOST__/g, 'evolvepreneuriq.app').replace(/__START__/g, '/chat/desktop');
function page(platform) {
  const g = globalThis; g.window = g; delete g.__evolve_messenger_links__;
  const calls = { external: [], back: 0, fwd: 0, assigned: [] };
  g.__TAURI_INTERNALS__ = { invoke: (c, a) => calls.external.push([c, a.url]) };
  g.history = { back: () => calls.back++, forward: () => calls.fwd++ };
  g.location = { assign: (u) => calls.assigned.push(u) };
  Object.defineProperty(g, 'navigator', { value: { platform }, configurable: true });
  const handlers = {}; g.document = { addEventListener: (t, f) => { handlers[t] = f; } };
  (0, eval)(src);
  return { calls, handlers };
}
const ev = (o) => ({ preventDefault() { this.prevented = true; }, prevented: false, ...o });
function click(p, href, target) { const e = ev({ target: { tagName: 'A', href, target: target || '', parentElement: null } }); p.handlers.click(e); return e; }
let n = 0; const ok = (m) => { n++; console.log('ok', m); };
const p = page('Win32');
for (const u of ['https://evolvepreneuriq.app/logout', 'https://evolvepreneuriq.app/email-manager', 'https://evolvepreneuriq.app/calendar', 'https://evolvepreneuriq.app/crm', 'https://evolvepreneuriq.app/app/preview/13?screen=inbox', 'https://evolvepreneuriq.app:443/crm', 'https://evolvepreneuriq.app/chat/desktop#x']) {
  const e = click(p, u); assert.equal(e.prevented, false, u); ok('stays in the window: ' + u);
}
assert.equal(p.calls.external.length, 0);
for (const u of ['https://evil.com/', 'https://evolvepreneuriq.app.evil.com/', 'https://evil-evolvepreneuriq.app/', 'https://xevolvepreneuriq.app/', 'https://evolvepreneuriq.app@evil.com/', 'https://evil.com@evolvepreneuriq.app/', 'https://user:pw@evolvepreneuriq.app/', 'http://evolvepreneuriq.app/', 'https://evolvepreneuriq.app:8443/', 'mailto:a@b.test']) {
  const before = p.calls.external.length; const e = click(p, u); assert.equal(e.prevented, true, u); assert.equal(p.calls.external.length, before + 1, u); ok('goes to the system browser: ' + u);
}
{ const before = p.calls.external.length; const e = click(p, 'https://evolvepreneuriq.app/crm', '_blank'); assert.equal(e.prevented, true); assert.equal(p.calls.external.length, before + 1); ok('target=_blank on a same-site link goes to the browser'); }
for (const u of ['javascript:alert(1)', 'file:///etc/passwd', 'ftp://x']) { const before = p.calls.external.length; click(p, u); assert.equal(p.calls.external.length, before, u); ok('not handed to the browser: ' + u); }
{ const e = ev({ altKey: true, key: 'ArrowLeft' }); p.handlers.keydown(e); assert.equal(p.calls.back, 1); assert.equal(e.prevented, true); ok('Alt+Left goes back'); }
{ p.handlers.keydown(ev({ altKey: true, key: 'ArrowRight' })); assert.equal(p.calls.fwd, 1); ok('Alt+Right goes forward'); }
{ p.handlers.keydown(ev({ altKey: true, key: 'Home' })); assert.deepEqual(p.calls.assigned, ['/chat/desktop']); ok('Alt+Home goes to the chat'); }
{ const b = p.calls.back; p.handlers.keydown(ev({ altKey: true, ctrlKey: true, key: 'ArrowLeft' })); p.handlers.keydown(ev({ key: 'ArrowLeft' })); assert.equal(p.calls.back, b); ok('other key combinations do nothing'); }
{ const e = ev({ button: 3 }); p.handlers.mouseup(e); assert.equal(p.calls.back, 2); assert.equal(e.prevented, true); p.handlers.mouseup(ev({ button: 4 })); assert.equal(p.calls.fwd, 2); p.handlers.mouseup(ev({ button: 0 })); assert.equal(p.calls.back, 2); ok('mouse back and forward buttons'); }
const m = page('MacIntel');
{ m.handlers.keydown(ev({ metaKey: true, key: '[' })); assert.equal(m.calls.back, 1); m.handlers.keydown(ev({ metaKey: true, key: ']' })); assert.equal(m.calls.fwd, 1); m.handlers.keydown(ev({ metaKey: true, shiftKey: true, key: 'H' })); assert.deepEqual(m.calls.assigned, ['/chat/desktop']); m.handlers.keydown(ev({ altKey: true, key: 'ArrowLeft' })); assert.equal(m.calls.back, 1, 'Option+Left stays a text shortcut on a Mac'); ok('Mac: Cmd+[ ] and Cmd+Shift+H'); }
{ const again = page('Win32'); const keys = Object.keys(again.handlers).sort().join(','); assert.equal(keys, 'click,keydown,mouseup'); ok('three listeners and one run per page'); }
{ const q = page('Win32'); q.handlers.keydown(ev({ ctrlKey: true, shiftKey: true, key: 'H' })); assert.deepEqual(q.calls.assigned, ['/chat/desktop']); ok('Ctrl+Shift+H goes to the chat'); }
console.log(n, 'passed');
