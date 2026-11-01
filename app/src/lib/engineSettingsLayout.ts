import type { Key } from '../i18n';

/** Come si disegna una voce delle impostazioni del motore proprio. */
export type SettingKind = 'bool' | 'text' | 'num' | 'secret' | 'reply' | 'choice';

export interface SettingRow {
  key: string;
  kind: SettingKind;
  /** Intervallo dei numeri (di base le porte, 1-65535). */
  min?: number;
  max?: number;
  /** Le scelte di una voce `choice`: il valore che si salva e l'etichetta. */
  options?: readonly { value: string; label: Key }[];
}

/** I servizi dell'invio diretto dei QSO (pulsante «Prova» e stato dell'ultimo invio). */
export type UploadService = 'wavelog' | 'clublog' | 'qrz' | 'lotw';

export interface SettingGroup {
  title: Key;
  rows: readonly SettingRow[];
  /** Invio diretto dei QSO: il servizio da provare e la spiegazione sotto le voci. */
  upload?: { service: UploadService; help: Key };
}

/** La scheda Segnalazioni come nella schermata di JTDX: stessi gruppi, stesse voci, stesso ordine. */
export const REPORT_GROUPS: readonly SettingGroup[] = [
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
    title: 'ens.g.adif',
    rows: [
      { key: 'TCPServer', kind: 'text' },
      { key: 'TCPServerPort', kind: 'num' },
      { key: 'EnableTCPConnection', kind: 'bool' },
      { key: 'UDP2Server', kind: 'text' },
      { key: 'UDP2ServerPort', kind: 'num' },
      { key: 'EnableUDP2adifBroadcast', kind: 'bool' },
    ],
  },
  {
    title: 'ens.g.alltxt',
    rows: [
      { key: 'WriteDecodedALLTXT', kind: 'bool' },
      { key: 'WriteDecodedDebugALLTXT', kind: 'bool' },
    ],
  },
  {
    title: 'ens.g.net',
    rows: [
      { key: 'EQSLSend', kind: 'bool' },
      { key: 'EQSLUser', kind: 'text' },
      { key: 'EQSLPasswd', kind: 'secret' },
      { key: 'EQSLNick', kind: 'text' },
      { key: 'PSKReporter', kind: 'bool' },
      { key: 'AllowSpotsDXSummit', kind: 'bool' },
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
    rows: [
      { key: 'QRZLogSend', kind: 'bool' },
      { key: 'QRZLogKey', kind: 'secret' },
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
  {
    title: 'ens.g.udp',
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
];
