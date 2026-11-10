// Parte comune alle tre varianti della proposta 1 (modi in alto, nessuna seconda riga).
// Ogni variante definisce window.PAGINE_IN(stato) per mettere Fox, Contest, Statistiche dove vuole.
(function () {
  const A = window.AUR;
  window.PAG = { ft8: ['Fox', 'Contest', 'Statistiche'], vhf: ['QMAP', 'Contest', 'Statistiche'], cw: ['Macro', 'Contest'], rtty: ['Macro', 'Contest'], psk: ['Macro'], sstv: ['Galleria'], sat: ['Doppler', 'Mappa'] };
  window.COM = [['📒', 'Log'], ['🗺', 'Mappa DX'], ['☀', 'Propagazione'], ['⚙', 'Impostazioni'], ['?', 'Guida'], ['★', 'Novità']];
  window.S = { modo: 'ft8', pagina: null };
  window.vai = (id) => { S.modo = id; S.pagina = null; chiudiMenu(); ridisegna(); };
  window.pag = (p) => { S.pagina = p; chiudiMenu(); ridisegna(); };
  window.chiudiMenu = () => document.querySelectorAll('.dd').forEach((d) => d.remove());
  window.modi = () => A.MODI.map((m) => `<span class="navp mode${m.id === S.modo ? ' sel' : ''}${m.pronto ? '' : ' soon'}" style="--c:${m.c}" onclick="vai('${m.id}')">${m.nome}</span>`).join('');
  window.icone = (extra = '') => `<div class="icons">${extra}${COM.map((c) => `<span class="ico" title="${c[1]}">${c[0]}</span>`).join('')}</div>`;
  window.ridisegna = () => {
    const parti = window.PAGINE_IN(S);
    document.getElementById('top').innerHTML = parti.top;
    const b = document.getElementById('board');
    b.innerHTML = S.pagina
      ? `<div class="board"><div class="p" style="grid-column:1/5;grid-row:1/4"><h3>${A.modo(S.modo).nome} · ${S.pagina}<span class="r"><span class="mini" style="cursor:pointer" onclick="pag(null)">← Stazione</span></span></h3><div class="b"><div class="empty">Qui la pagina «${S.pagina}» di oggi, uguale</div></div></div></div>`
      : A.board(S.modo);
  };
  document.addEventListener('click', (e) => { if (!e.target.closest('.dd') && !e.target.closest('[data-dd]')) chiudiMenu(); });
  window.menuSotto = (el, voci) => {
    chiudiMenu();
    const r = el.getBoundingClientRect();
    const d = document.createElement('div');
    d.className = 'dd';
    // allineato al bordo destro del pulsante: vicino al bordo dello schermo il menu restava fuori e il testo non si vedeva
    d.style.cssText = `position:absolute;top:${r.bottom + 4}px;min-width:${Math.max(180, r.width)}px;padding:6px;background:#141c33;color:#e6edf6;border:1px solid var(--line2);border-radius:10px;z-index:60;box-shadow:0 12px 30px rgb(0 0 0/.6);font-weight:600;white-space:nowrap`;
    d.innerHTML = voci.map((v) => `<div style="padding:7px 12px;border-radius:6px;cursor:pointer" onmouseover="this.style.background='#26306e'" onmouseout="this.style.background=''" onclick="pag(${v[1] === null ? 'null' : `'${v[1]}'`})">${v[0]}</div>`).join('');
    document.body.appendChild(d);
    const w = d.getBoundingClientRect().width;
    d.style.left = `${Math.max(8, Math.min(r.left, window.innerWidth - w - 8))}px`;
  };
})();
