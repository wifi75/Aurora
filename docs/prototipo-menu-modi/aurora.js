// La Stazione di Aurora com'e' oggi (dati della schermata del 2026-11-10), riusata dai cinque prototipi. Il modo scelto cambia solo la parte
// centrale (Attivita' di banda, Finestra RX, Trasmissione, Cascata); DX Cluster, Bande aperte, Log, Comandi e CQ/DXpedition restano in comune.
window.AUR = (function () {
  const MODI = [
    { id: 'ft8', nome: 'FT8', sotto: 'FT8 · FT4', c: '#2cf5a4', pronto: true },
    { id: 'vhf', nome: 'VHF', sotto: 'MSK144 · Q65 · JT65', c: '#2cf5a4', pronto: true },
    { id: 'cw', nome: 'CW', sotto: 'Morse', c: '#ffcf5c', pronto: false },
    { id: 'rtty', nome: 'RTTY', sotto: '45,45 baud', c: '#ff8a5c', pronto: false },
    { id: 'psk', nome: 'PSK', sotto: 'PSK31 · PSK63', c: '#5cc8ff', pronto: false },
    { id: 'sstv', nome: 'SSTV', sotto: 'immagini', c: '#e07bff', pronto: false },
    { id: 'sat', nome: 'Satelliti', sotto: 'RS-44 · Doppler', c: '#8e7bff', pronto: false },
  ];
  const CONDIVISI = ['Fox', 'Contest', 'Statistiche', 'Mappa DX', 'Log', 'Propagazione'];
  const ALTRO = ['Impostazioni', 'Guida', 'Novità'];
  const CLUSTER = [
    ['SA7SVR', '24.915', 'SV9MBH', '🇬🇷', 'Crete', 'FT8', 'FT8 QRV HELLAS Zone N...'], ['F4CXO', '50.475', 'ED7YAD/B', '🇪🇸', 'Spain', 'CW', 'CW 519'],
    ['DC8MH', '7.122', 'DK80NDS', '🇩🇪', 'Fed. Rep. of Germany', 'SSB', 'ssb 80th State of Lower ...'], ['RA4FP', '24.916', 'K0DEQ', '🇺🇸', 'United States', 'FT8', 'FT8 -18 dB 1605 Hz'],
    ['GW7SMV', '50.313', 'CT1BOL', '🇵🇹', 'Portugal', 'FT8', 'IO81LO<ES>IN51QD FT...'], ['TA2BU', '28.515', 'NP2OZ/MM', '', '', 'SSB', 'ssb CQ'],
    ['YC1JEL', '14.074', 'R8PG', '🇷🇺', 'Asiatic Russia', 'FT8', 'FT8 -20dB 949Hz'], ['N0YO', '24.916', 'OH3JF', '🇫🇮', 'Finland', 'FT8', ''],
    ['HA1DAE', '14.026', 'IZ8SYI', '🇮🇹', 'Italy', '', ''], ['IK2GQQ', '24.917', 'VP2MCV', '🇲🇸', 'Montserrat', 'FT8', 'CCC VP2M* DXCC 96 DX...', true],
    ['UA3ARC', '24.915', 'K2ZJ', '🇺🇸', 'United States', 'FT8', 'FT8 -2 dB 486 Hz'], ['UA3ARC', '24.915', 'AA1V', '🇺🇸', 'United States', 'FT8', 'FT8 +10 dB 1640 Hz'],
    ['MM7JNT', '7.146', 'M8HPI', '🏴', 'England', '', 'WWBOTA B/G-1896'], ['UA3ARC', '24.915', 'VE3XVC', '🇨🇦', 'Canada', 'FT8', 'FT8 -4 dB 1739 Hz'],
    ['PD4BBQ', '14.238', 'ZL1HX', '🇳🇿', 'New Zealand', '', 'Tnx for the qso 73'], ['PD0TDK', '28.180', 'N4WLO', '🇺🇸', 'United States', 'FT4', 'FT4'],
  ];
  const DXPED = [
    ['5R8IC', '🇲🇬', 'Madagascar', '20/12', '28.008', 'CW', 'spot 3 min fa', 'F8BJJ'], ['KH8A', '🇦🇸', 'American Samoa', '12/11', '7.192', 'SSB', 'spot 4 min fa', 'DL1RWN'],
    ['9X5KM', '🇷🇼', 'Rwanda', '11/11', '21.005', 'CW', 'spot 2 min fa', 'DL9UO'], ['FW3GT', '🇼🇫', 'Wallis & Futuna', '14/11', '24.915', 'FT8', 'spot 6 min fa', 'JA1XYZ'],
  ];
  const BANDE = [['80m-40m', 'Discreta', 'Buona', 2], ['30m-20m', 'Buona', 'Buona', 3], ['17m-15m', 'Buona', 'Buona', 3], ['12m-10m', 'Discreta', 'Scarsa', 2]];
  const bars = (n) => [6, 9, 12].map((h, i) => `<span class="bar" style="height:${h}px;${i < n ? '' : 'opacity:.25'}"></span>`).join('');
  const modo = (id) => MODI.find((m) => m.id === id) || MODI[0];

  function cluster() {
    return `<div class="p" style="grid-column:3;grid-row:1"><h3>DX Cluster<span class="r"><span class="muted">Collegato a 3 su 3</span></span></h3><div class="b"><table><tr><th>DE</th><th>Freq</th><th>DX</th><th>Paese</th><th>Modo</th><th>Commenti</th></tr>${CLUSTER.map((c) => `<tr class="${c[7] ? 'hl' : ''}"><td class="mono muted">${c[0]}</td><td class="mono">${c[1]}</td><td class="mono"><b>${c[2]}</b></td><td>${c[3]} ${c[4]}</td><td><b>${c[5]}</b></td><td>${c[6]}</td></tr>`).join('')}</table></div></div>`;
  }
  function bande() {
    return `<div class="p" style="grid-column:4;grid-row:1"><h3>Bande aperte</h3><div class="b"><table class="bands"><tr><th></th><th>☀ Giorno</th><th>● Notte</th></tr>${BANDE.map((b) => `<tr><td class="mono"><b>${b[0]}</b></td><td>${bars(b[3])} ${b[1]}</td><td>${bars(b[2] === 'Scarsa' ? 1 : 3)} ${b[2]}</td></tr>`).join('')}</table><div class="muted" style="padding:6px 8px">Le più lontane che mi ascoltano: nessuna ricezione ancora (PSK Reporter)</div></div></div>`;
  }
  function comandi() {
    return `<div class="p" style="grid-column:4;grid-row:2"><h3>Comandi</h3><div class="b" style="padding:8px"><div class="muted" style="display:flex;gap:30px">TX<span>RX</span><span>RAPPORTO</span></div><div class="mono" style="display:flex;gap:10px;margin:4px 0 8px"><span class="mini">1500</span><span class="mini">1500</span><span class="mini">-10</span></div><span class="mini">Hold Tx Freq</span> <span class="mini">Spot DX</span> <span class="mini on">Tx/Rx Split</span> <span class="mini">Canc DX</span></div></div>`;
  }
  function cq() {
    return `<div class="p" style="grid-column:4;grid-row:3"><h3>CQ e novità</h3><div class="b"><div class="empty" style="height:auto;padding:8px">Nessun CQ per ora</div>${DXPED.map((x) => `<div class="cq-dx"><b>${x[0]}</b><span>${x[1]}</span><span>${x[2]}</span><span class="fino">fino al ${x[3]}</span><span class="live">● IN ONDA ${x[4]} MHz ${x[5]} · ${x[6]} · da ${x[7]}</span></div>`).join('')}</div></div>`;
  }
  function log() {
    return `<div class="p" style="grid-column:2;grid-row:2"><h3>Log <span class="muted">0 QSO</span><span class="r"><span class="mini on">Log</span><span class="mini">Mappa</span><span class="mini on">RX</span></span></h3><div class="b"><div class="empty" style="font-size:13px">Il log è vuoto · clicca per importarlo o collegare un log esterno</div></div></div>`;
  }

  // la parte centrale, diversa per ogni modo
  function centro(id) {
    const m = modo(id);
    if (id === 'ft8' || id === 'vhf') {
      return `<div class="p" style="grid-column:1;grid-row:1/3"><h3>Attività di banda<span class="r"><span class="mini">Filtro</span><span class="mini">Nuovi ↑</span></span></h3><div class="b"><div class="empty">In ascolto: i messaggi compaiono qui</div></div></div>
      <div class="p" style="grid-column:2;grid-row:1"><h3>Finestra RX <span class="mono muted">1500 Hz ±50</span><span class="r"><span class="mini">Nuovi ↑</span></span></h3><div class="b"><div class="empty">Niente sulla tua frequenza di ricezione</div></div></div>
      <div class="p" style="grid-column:3;grid-row:2/4"><h3>Trasmissione<span class="r"><span class="rxbadge">RX</span></span></h3><div class="b">${['Risposta con locatore', 'Rapporto', 'Rapporto con R', 'Chiusura RR73', 'Saluto 73'].map((t, i) => `<div class="txrow"><i>${i + 1}</i>${t}</div>`).join('')}<div class="txrow sel"><i>6</i><span class="mono"><b>CQ IU3ABC JN55</b></span><span>Chiamata CQ</span></div></div></div>
      <div class="p" style="grid-column:1/3;grid-row:3"><h3>Cascata<span class="r muted">0–3000 Hz</span></h3><div class="b"><div class="wf"><span class="rx"></span></div></div></div>`;
    }
    const testi = {
      cw: ['Decodifica CW · 22 wpm', 'CQ CQ DE 5R8IC 5R8IC K<br>5R8IC DE IU3ABC IU3ABC KN<br><span class="me">IU3ABC 599 TU K</span>', 'Macro', 'F1 CQ · F2 599 · F3 TU 73 · velocità 22 wpm · keyer interno o CAT'],
      rtty: ['Ricezione RTTY · 45,45 baud, shift 170 Hz', 'CQ CQ CQ DE TJ1GD TJ1GD PSE K<br><span class="me">TJ1GD DE IU3ABC UR 599 599 NAME TIZIANO BK</span>', 'Testo da trasmettere', 'macro F1-F8 · CQ, scambio, 73'],
      psk: ['Ricezione PSK31 · 1250 Hz', 'CQ CQ DE CT1BOL CT1BOL PSE K<br><span class="me">CT1BOL DE IU3ABC GM OM UR 599</span>', 'Testo da trasmettere', 'più canali PSK insieme nella cascata'],
      sstv: ['Immagine in arrivo · Scottie 1 · 14.230', '<div style="height:150px;border-radius:6px;background:linear-gradient(180deg,#2a6fb8,#e0a050 60%,#3a2a18)"></div>', 'Galleria e trasmissione', '3 immagini oggi da 9A1AA, OH3JF, ZL1HX'],
      sat: ['Passaggi da JN55 · RS-44 tra 12 min', '<table><tr><th>Satellite</th><th>Sorge</th><th>Elev.</th><th>Durata</th></tr><tr><td><b>RS-44</b></td><td>14:02</td><td>52°</td><td>16 min</td></tr><tr><td>SO-50</td><td>14:37</td><td>23°</td><td>11 min</td></tr><tr><td>AO-91</td><td>15:10</td><td>71°</td><td>14 min</td></tr></table>', 'Doppler RS-44', 'Uplink 145.965,2 (+3,1 kHz) · Downlink 435.639,8 (−9,4 kHz) · Az 118° El 12° · rotore segue'],
    }[id];
    return `<div class="p" style="grid-column:1/3;grid-row:1"><h3><span style="color:${m.c}">●</span> ${testi[0]}<span class="r"><span class="mini">${m.nome}</span></span></h3><div class="b mtext">${testi[1]}</div>${m.pronto ? '' : '<div class="soonbox">Prototipo: questo modo non c\'è ancora (progetto P2/P1 in docs/tasks.md). Radio, cluster, log, bande e DXpedition sono gli stessi della pagina FT8.</div>'}</div>
      <div class="p" style="grid-column:3;grid-row:2/4"><h3>${testi[2]}</h3><div class="b" style="padding:10px">${testi[3]}<div style="margin-top:10px"><span class="mini on">Trasmetti</span> <span class="mini">STOP</span></div></div></div>
      <div class="p" style="grid-column:1/3;grid-row:3"><h3>Cascata · ${m.nome}<span class="r muted">${m.sotto}</span></h3><div class="b"><div class="wf"><span class="rx"></span></div></div></div>`;
  }
  function board(id) {
    return `<div class="board">${centro(id)}${cluster()}${bande()}${comandi()}${cq()}${id === 'ft8' || id === 'vhf' ? log() : ''}</div>`;
  }
  function row2(id, modeSelect = true) {
    const m = modo(id);
    return `<div class="row2">${modeSelect ? `<span class="modepill" style="border-color:${m.c};color:${m.c}">${m.nome === 'VHF' ? 'MSK144' : m.nome} ▾</span>` : ''}<span class="mhz mono">— MHz ▾</span>${id === 'ft8' || id === 'vhf' ? '<div class="seg"><span class="on">Monitora</span><span>Hound</span><span>SWL</span><span class="on">AGCc</span><span>Filtro</span><span class="on">a7/a8</span><span class="on">Multithread</span></div>' : '<div class="seg"><span class="on">Monitora</span><span>Filtro stretto</span><span>AFC</span></div>'}<div class="seg"><span>Ferma</span><span>Decodifica</span><span>Cancella</span></div><div class="seg"><span>Auto</span></div></div>`;
  }
  const brand = '<div class="brand">AURORA<small>by IU3ABC</small></div>';
  const vol = '<div class="vol"><span class="mini">🔊</span><span class="mono muted">13:56:30 UTC</span></div>';
  return { MODI, CONDIVISI, ALTRO, modo, board, row2, brand, vol };
})();
