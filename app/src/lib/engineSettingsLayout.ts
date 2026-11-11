import type { Key } from '../i18n';

/** Come si disegna una voce delle impostazioni del motore proprio. */
export type SettingKind = 'bool' | 'text' | 'num' | 'secret' | 'reply' | 'choice';

export interface SettingRow {
  key: string;
  kind: SettingKind;
  /** Intervallo dei numeri (di base le porte, 1-65535). */
  min?: number;
  max?: number;
  /** Passo dei numeri (di base 1; per esempio 0.1 per la soglia del ROS). */
  step?: number;
  /** Le scelte di una voce `choice`: il valore che si salva e l'etichetta (da tradurre, o `text` gia' pronto come «50 Hz»). */
  options?: readonly ({ value: string; label: Key } | { value: string; text: string })[];
}

/** I servizi con il pulsante «Prova»: invio diretto dei QSO, ricerche, spot, dati online, orologio e programmi collegati. */
export type UploadService =
  | 'wavelog'
  | 'clublog'
  | 'qrz'
  | 'lotw'
  | 'hrdlog'
  | 'qrzlookup'
  | 'hamqth'
  | 'eqsl'
  | 'pskreporter'
  | 'dxsummit'
  | 'ng3k'
  | 'solar'
  | 'ntp'
  | 'udp'
  | 'udp2'
  | 'tcp';


export interface SettingGroup {
  title: Key;
  rows: readonly SettingRow[];
  /** Invio diretto dei QSO: il servizio da provare e la spiegazione (nel «?» del titolo). */
  /** `tip`: il suggerimento del pulsante; `auto: false`: niente prova da sola all'apertura; `always`: servizio sempre attivo (spia senza interruttore). */
  upload?: { service: UploadService; help: Key; tip?: Key; auto?: boolean; always?: boolean };
  /** La spiegazione del riquadro, nel «?» del titolo. */
  help?: Key;
  /** Una riga breve sotto le voci (per esempio il rimando a un'altra pagina). */
  note?: Key;
}

/** Scheda «Servizi di log»: un riquadro per servizio esterno, in ordine. */
export const LOG_SERVICE_GROUPS: readonly SettingGroup[] = [
  {
    title: 'ens.g.eqsl',
    upload: { service: 'eqsl', help: 'ens.eqsl.help', tip: 'tip.test.eqsl' },
    rows: [
      { key: 'EQSLSend', kind: 'bool' },
      { key: 'EQSLUser', kind: 'text' },
      { key: 'EQSLPasswd', kind: 'secret' },
      { key: 'EQSLNick', kind: 'text' },
    ],
  },
  {
    title: 'ens.g.wavelog',
    upload: { service: 'wavelog', help: 'ens.up.wavelog.help' },
    rows: [
      { key: 'WavelogSend', kind: 'bool' },
      { key: 'WavelogUrl', kind: 'text' },
      { key: 'WavelogKey', kind: 'secret' },
      { key: 'WavelogStation', kind: 'text' },
    ],
  },
  {
    title: 'ens.g.clublog',
    upload: { service: 'clublog', help: 'ens.up.clublog.help' },
    rows: [
      { key: 'ClublogSend', kind: 'bool' },
      { key: 'ClublogEmail', kind: 'text' },
      { key: 'ClublogPasswd', kind: 'secret' },
      { key: 'ClublogCall', kind: 'text' },
      { key: 'ClublogApiKey', kind: 'secret' },
    ],
  },
  {
    title: 'ens.g.qrz',
    upload: { service: 'qrz', help: 'ens.up.qrz.help' },
    note: 'ens.qrz.seeOnline',
    rows: [
      { key: 'QRZLogSend', kind: 'bool' },
      { key: 'QRZLogKey', kind: 'secret' },
      { key: 'QRZLogCall', kind: 'text' },
    ],
  },
  {
    title: 'ens.g.hrdlog',
    upload: { service: 'hrdlog', help: 'ens.up.hrdlog.help' },
    rows: [
      { key: 'HrdlogSend', kind: 'bool' },
      { key: 'HrdlogCall', kind: 'text' },
      { key: 'HrdlogCode', kind: 'secret' },
    ],
  },
  {
    title: 'ens.g.lotw',
    upload: { service: 'lotw', help: 'ens.up.lotw.help' },
    rows: [
      { key: 'LoTWSend', kind: 'bool' },
      { key: 'LoTWTqsl', kind: 'text' },
      { key: 'LoTWLocation', kind: 'text' },
    ],
  },
];

/** Scheda «Programmi collegati»: server UDP primario, ADIF via UDP, TCP (N1MM); sotto, il riquadro Integrazioni. */
export const LINK_GROUPS: readonly SettingGroup[] = [
  {
    title: 'ens.g.udp',
    // il server primario riceve sempre: la spia non segue il primo interruttore (le richieste del programma); nessuna prova da sola all'apertura
    upload: { service: 'udp', help: 'ens.udp.help', tip: 'tip.test.udp', auto: false, always: true },
    rows: [
      { key: 'UDPServer', kind: 'text' },
      { key: 'UDPServerPort', kind: 'num' },
      { key: 'AcceptUDPRequests', kind: 'bool' },
      { key: 'AcceptUDPReplyMessages', kind: 'reply' },
      { key: 'udpWindowToFront', kind: 'bool' },
      { key: 'udpWindowRestore', kind: 'bool' },
      { key: 'EnableUDP1adifSending', kind: 'bool' },
      { key: 'preventFalseUDPspots', kind: 'bool' },
      { key: 'ApplyFiltersToUDPmessages', kind: 'bool' },
    ],
  },
  {
    title: 'ens.g.adif',
    upload: { service: 'udp2', help: 'ens.rep.help', tip: 'tip.test.udp2', auto: false },
    rows: [
      { key: 'EnableUDP2adifBroadcast', kind: 'bool' },
      { key: 'UDP2Server', kind: 'text' },
      { key: 'UDP2ServerPort', kind: 'num' },
    ],
  },
  {
    title: 'ens.g.tcp',
    upload: { service: 'tcp', help: 'ens.tcp.help', tip: 'tip.test.tcp', auto: false },
    rows: [
      { key: 'EnableTCPConnection', kind: 'bool' },
      { key: 'TCPServer', kind: 'text' },
      { key: 'TCPServerPort', kind: 'num' },
    ],
  },
];

/** Gli spot (PSK Reporter e DXSummit), un riquadro ciascuno con la sua spia: stanno nella scheda «Servizi online». */
export const SPOT_GROUPS: readonly SettingGroup[] = [
  {
    title: 'ens.g.pskspot',
    upload: { service: 'pskreporter', help: 'ens.dxsummit.help', tip: 'tip.test.pskreporter' },
    rows: [{ key: 'PSKReporter', kind: 'bool' }],
  },
  {
    title: 'ens.g.dxsummit',
    upload: { service: 'dxsummit', help: 'ens.dxsummit.help', tip: 'tip.test.dxsummit' },
    rows: [{ key: 'AllowSpotsDXSummit', kind: 'bool' }],
  },
];

/** Le ricerche dei nominativi (riquadro del nominativo nel DX Cluster): stanno nella scheda «Servizi online». */
export const LOOKUP_GROUPS: readonly SettingGroup[] = [
  {
    title: 'ens.g.qrzlookup',
    upload: { service: 'qrzlookup', help: 'ens.up.qrzlookup.help' },
    note: 'ens.qrz.seeLog',
    rows: [
      { key: 'QRZLookup', kind: 'bool' },
      { key: 'QRZUser', kind: 'text' },
      { key: 'QRZPasswd', kind: 'secret' },
      { key: 'LookupToQso', kind: 'bool' },
    ],
  },
  {
    title: 'ens.g.hamqth',
    upload: { service: 'hamqth', help: 'ens.up.hamqth.help' },
    rows: [
      { key: 'HamQTHLookup', kind: 'bool' },
      { key: 'HamQTHUser', kind: 'text' },
      { key: 'HamQTHPasswd', kind: 'secret' },
    ],
  },
];

/** Come si registra il QSO e il file ALL.TXT: stanno nella scheda «Sequenza». */
export const QSO_LOG_GROUPS: readonly SettingGroup[] = [
  {
    title: 'ens.g.log',
    rows: [
      { key: 'PromptToLog', kind: 'bool' },
      { key: 'AutoQSOLogging', kind: 'bool' },
      { key: 'toRTTY', kind: 'bool' },
      { key: 'dBtoComments', kind: 'bool' },
      { key: 'distanceToComments', kind: 'bool' },
      { key: 'ClearCallGrid', kind: 'bool' },
      { key: 'ClearCallGridExit', kind: 'bool' },
    ],
  },
  {
    title: 'ens.g.alltxt',
    rows: [
      { key: 'WriteDecodedALLTXT', kind: 'bool' },
      { key: 'WriteDecodedDebugALLTXT', kind: 'bool' },
    ],
  },
];

/** Tutte le voci che una volta stavano nella scheda Segnalazioni (per i controlli e la ricerca). */
export const REPORT_GROUPS: readonly SettingGroup[] = [...LINK_GROUPS, ...LOG_SERVICE_GROUPS, ...SPOT_GROUPS, ...QSO_LOG_GROUPS];
