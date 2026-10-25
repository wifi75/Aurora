import type { Key } from '../i18n';

/** Come si disegna una voce delle impostazioni del motore proprio. */
export type SettingKind = 'bool' | 'text' | 'num' | 'secret' | 'reply';

export interface SettingRow {
  key: string;
  kind: SettingKind;
  /** Intervallo dei numeri (di base le porte, 1-65535). */
  min?: number;
  max?: number;
}

export interface SettingGroup {
  title: Key;
  rows: readonly SettingRow[];
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
