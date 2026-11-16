// Evolve Messenger 1.0.6: the site pages that open INSIDE the window besides the chat (Profile, Notification settings) are the normal EIQ pages with no way back to the chat.
// This puts one small "Back to chat" bar at the top of those pages only; chat and sign-in pages never get it. Runs once per page load.
(function() {
    if (location.hostname !== '__HOST__') return;
    if (/^\/(chat|login|login_check|2fa|forgot|reset|register|logout)([\/?#]|$)/.test(location.pathname)) return;
    if (document.getElementById('evolve-back-to-chat')) return;
    var bar = document.createElement('a');
    bar.id = 'evolve-back-to-chat';
    bar.href = '/chat/desktop';
    bar.setAttribute('aria-label', 'Back to chat');
    bar.textContent = '← Back to chat';
    bar.style.cssText = 'position:fixed;top:8px;left:8px;z-index:2147483647;padding:6px 12px;border-radius:8px;background:#111827;color:#fff;font:600 13px/1.2 system-ui,sans-serif;text-decoration:none;box-shadow:0 2px 8px rgba(0,0,0,.35)';
    (document.body || document.documentElement).appendChild(bar);
})();
