// Le tre varianti grafiche della B3: stessa struttura (modi in alto; a destra pagine del modo, Log/Mappa DX/Propagazione e menu ⚙),
// niente orario in testata, volume + cursore + tema in una barra sottile in fondo. Ogni pagina sceglie lo stile con window.STILE.
(function () {
  const IC = { Fox: '🦊', Contest: '🏆', Statistiche: '📊', QMAP: '〰', Macro: '⌨', Galleria: '🖼', Doppler: '↔', Mappa: '🗺' };
  window.PAGINE_IN = (s) => {
    const m = AUR.modo(s.modo);
    const st = window.STILE;
    const mie = PAG[s.modo].map((p) => `<span class="${st.mod}${p === s.pagina ? ' sel' : ''}" style="--c:${m.c}" onclick="pag(S.pagina==='${p}'?null:'${p}')">${st.icone ? (IC[p] || '') + ' ' : ''}${p}</span>`).join('');
    const com = COM.slice(0, 3).map((c) => `<span class="${st.com}">${st.icone ? c[0] + ' ' : ''}${c[1]}</span>`).join('');
    const altro = `<span class="${st.altro}" data-dd onclick="menuSotto(this, COM.slice(3).map((c)=>[c[0]+'  '+c[1], c[1]]))">⚙ ▾</span>`;
    const destra = st.gruppi ? `<div class="grp">${mie}</div><div class="grp">${com}${altro}</div>` : `${mie}<span class="sep"></span>${com}${altro}`;
    return { top: `<div class="top">${AUR.brand}<div class="nav">${modi()}</div><div class="icons">${destra}</div></div>${AUR.row2(s.modo)}` };
  };
  // barra in fondo: volume, cursore e tema (l'orario non c'e' piu')
  const piede = document.createElement('div');
  piede.className = 'piede';
  // la barra in fondo di oggi (decodifiche, periodo, WD, orario, versione) con la barra del periodo piu' corta: al suo posto volume e tema
  piede.innerHTML = `<span class="muted">Decodifiche: <b style="color:#fff">0</b></span><span class="periodo"><i></i></span><span class="muted mono">2/15</span><span class="muted">WD: <b style="color:#fff">6m</b></span>
    <span class="vgrp"><span class="pbtn" title="Audio della finestra">🔊</span><input type="range" min="0" max="100" value="70" title="Volume"><span class="pbtn" title="Tema chiaro/scuro" onclick="this.textContent=this.textContent==='🌙'?'☀':'🌙'">🌙</span></span>
    <span class="mono ora">14:46:46 <small>UTC</small></span><span class="ver">Aurora v0.11.227 — By IU3WJQ - Tiziano Cassone</span>`;
  document.addEventListener('DOMContentLoaded', () => document.body.appendChild(piede));
})();
