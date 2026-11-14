// Live unread number (EVO-BUG-20296, spec docs/specs/messenger/MESSENGER_UNREAD_LIVE_TOPIC.md in EIQ). Runs once per page load inside the messenger page.
// The server pushes {"type":"unread","total":N,...} on the person's own Mercure topic; the number then updates in seconds. Anything wrong (flag off, no
// sign-in, hub unreachable, stream cut) leaves the normal poll in charge: window.__evolve_live_ok is true ONLY while a stream is open.
(function() {
    if (!window.__TAURI_INTERNALS__ || location.hostname !== '__HOST__') return;
    if (/^\/(login|login_check|2fa|forgot|reset|register|logout)([\/?#]|$)/.test(location.pathname)) return;
    if (window.__evolve_live_started) return;
    window.__evolve_live_started = true;
    window.__evolve_live_ok = false;
    var DEAD = 0;

    function sleep(ms) { return new Promise(function(r) { setTimeout(r, ms); }); }

    // Parse one SSE chunk stream into events; calls onData(text) for each complete event.
    function makeParser(onData) {
        var buf = '', data = [];
        return function(chunk) {
            buf += chunk;
            var lines = buf.split(/\r\n|\n|\r/);
            buf = lines.pop();
            for (var i = 0; i < lines.length; i++) {
                var l = lines[i];
                if (l === '') { if (data.length) { onData(data.join('\n')); data = []; } }
                else if (l.indexOf('data:') === 0) { data.push(l.slice(5).replace(/^ /, '')); }
            }
        };
    }

    function handle(text) {
        var e;
        try { e = JSON.parse(text); } catch (x) { return; }
        if (!e || e.type !== 'unread') return;
        var t = e.total;
        if (typeof t !== 'number' || !isFinite(t) || t < 0 || Math.floor(t) !== t) return; // no usable total: ignore, the poll covers it
        window.__TAURI_INTERNALS__.invoke('report_unread', {count: t});
    }

    async function getKey() {
        var r = await fetch('/api/v1/desktop/live-auth', {credentials: 'include', headers: {'Accept': 'application/json', 'X-Requested-With': 'XMLHttpRequest'}});
        if (r.status === 404) return {off: true};          // server flag off: ask again much later
        if (r.status === 401 || r.status === 403) return {stop: true}; // signed out: the next page load tries again
        if (!r.ok) return {retry: true};
        var a = await r.json();
        if (!a || typeof a.url !== 'string' || typeof a.token !== 'string' || !/^chat\/user\/\d+$/.test(String(a.topic))) return {retry: true};
        if (a.url.indexOf('https://') !== 0) return {retry: true};
        return {auth: a};
    }

    async function listen(a) {
        var ctl = new AbortController();
        var until = Date.parse(a.expires_at);
        var life = isFinite(until) ? Math.max(60000, until - Date.now() - 5 * 60000) : 4 * 3600 * 1000; // renew the key 5 min before it expires
        var timer = setTimeout(function() { ctl.abort(); }, life);
        try {
            var resp = await fetch(a.url + (a.url.indexOf('?') < 0 ? '?' : '&') + 'topic=' + encodeURIComponent(a.topic), {
                headers: {'Authorization': 'Bearer ' + a.token, 'Accept': 'text/event-stream'}, signal: ctl.signal, cache: 'no-store'
            });
            if (!resp.ok || !resp.body) return false;
            window.__evolve_live_ok = true;
            var dec = new TextDecoder(), parse = makeParser(handle), rd = resp.body.getReader();
            for (;;) {
                var step = await rd.read();
                if (step.done) break;
                parse(dec.decode(step.value, {stream: true}));
            }
            return true;
        } catch (x) {
            return false;
        } finally {
            clearTimeout(timer);
            window.__evolve_live_ok = false;
        }
    }

    (async function run() {
        var wait = 5000;
        for (;;) {
            if (DEAD) return;
            var k;
            try { k = await getKey(); } catch (x) { k = {retry: true}; }
            if (k.stop) return;
            if (k.off) { await sleep(30 * 60000); continue; }
            if (k.auth) {
                var t0 = Date.now();
                await listen(k.auth);
                wait = Date.now() - t0 > 60000 ? 5000 : Math.min(wait * 2, 60000); // a stream that lived a minute resets the back-off
            } else {
                wait = Math.min(wait * 2, 60000);
            }
            await sleep(wait);
        }
    })();
})();
