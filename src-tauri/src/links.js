// Evolve Messenger 1.0.8 (EVO-BUG-20300): links and the keys that move around the window. Runs once per page load on the app host.
// Rule: a link stays in the window when its address is https, EXACTLY the app host, no user name or password, and the default port; every path of the site then opens here (Email, Calendar, CRM, Profile...).
// Everything else (other hosts, http, mailto, target=_blank) opens in the system browser. The same rule is guard::is_same_site in Rust; tests/links_harness.mjs runs this file.
// Back / forward / Home: Alt+Left, Alt+Right and the mouse back and forward buttons move through the window's own history; Alt+Home or Ctrl+Shift+H (Cmd+Shift+H on a Mac) goes to the chat.
(function() {
    if (window.__evolve_messenger_links__) return;
    window.__evolve_messenger_links__ = true;
    function sameSite(u) {
        return u.protocol === 'https:' && u.hostname === '__HOST__' && !u.username && !u.password && (u.port === '' || u.port === '443');
    }
    document.addEventListener('click', function(e) {
        var el = e.target;
        while (el && el.tagName !== 'A') el = el.parentElement;
        if (!el || !el.href) return;
        try {
            var u = new URL(el.href);
            if (!sameSite(u) || el.target === '_blank') {
                if (u.protocol === 'https:' || u.protocol === 'http:' || u.protocol === 'mailto:') {
                    e.preventDefault();
                    window.__TAURI_INTERNALS__.invoke('open_external_url', {url: el.href});
                }
            }
        } catch (_) {}
    }, true);
    var isMac = /Mac/.test(navigator.platform || '');
    document.addEventListener('keydown', function(e) {
        var back = isMac ? (e.metaKey && !e.altKey && !e.ctrlKey && !e.shiftKey && e.key === '[') : (e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey && e.key === 'ArrowLeft');
        var fwd = isMac ? (e.metaKey && !e.altKey && !e.ctrlKey && !e.shiftKey && e.key === ']') : (e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey && e.key === 'ArrowRight');
        var home = isMac ? (e.metaKey && e.shiftKey && !e.altKey && !e.ctrlKey && (e.key === 'h' || e.key === 'H')) : ((e.altKey && !e.metaKey && !e.ctrlKey && !e.shiftKey && e.key === 'Home') || (e.ctrlKey && e.shiftKey && !e.altKey && !e.metaKey && (e.key === 'h' || e.key === 'H')));
        if (back) { e.preventDefault(); history.back(); }
        else if (fwd) { e.preventDefault(); history.forward(); }
        else if (home) { e.preventDefault(); location.assign('__START__'); }
    }, true);
    document.addEventListener('mouseup', function(e) {
        if (e.button === 3) { e.preventDefault(); history.back(); }
        else if (e.button === 4) { e.preventDefault(); history.forward(); }
    }, true);
})();
