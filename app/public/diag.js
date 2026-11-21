// Se l'interfaccia non parte (pagina bianca) mostra l'errore scritto, cosi' si capisce perche' senza aprire la console del browser.
(function () {
  var errors = [];
  function add(text) {
    errors.push(String(text).slice(0, 600));
  }
  window.addEventListener('error', function (e) {
    add((e.message || 'errore') + (e.filename ? '\n  in ' + e.filename + ':' + e.lineno : ''));
  });
  window.addEventListener('unhandledrejection', function (e) {
    add('promessa respinta: ' + (e.reason && (e.reason.stack || e.reason.message) ? e.reason.stack || e.reason.message : e.reason));
  });
  setTimeout(function () {
    var root = document.getElementById('root');
    var empty = !root || root.childElementCount === 0;
    if (!empty || errors.length === 0) return;
    var box = document.createElement('pre');
    box.setAttribute('style', 'position:fixed;inset:0;margin:0;padding:1.2rem;background:#fff;color:#111;font:13px/1.4 monospace;white-space:pre-wrap;overflow:auto;z-index:99999');
    box.textContent = "L'interfaccia di Aurora non e' partita. Errore del browser (" + navigator.userAgent + "):\n\n" + errors.join('\n\n');
    document.body.appendChild(box);
  }, 4000);
})();
