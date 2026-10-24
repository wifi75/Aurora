import { AutomationStateSchema, type AutomationRules, type AutomationState } from './automation';
import { historyParams, type HistoryFilter } from '../lib/history';
import type { Wishlist } from '../lib/wishlist';
import { HealthSchema, type Health, type WantedAlert } from './schemas';
import { parseEngineTx, useEngineTx } from '../state/engineTx';

const BASE = '/api/v1';

export class ApiError extends Error {
  constructor(
    readonly status: number,
    readonly code: string,
    readonly retryAfterSecs?: number,
  ) {
    super(`${status} ${code}`);
    this.name = 'ApiError';
  }
}

async function request(path: string, init?: RequestInit): Promise<Response> {
  const response = await fetch(BASE + path, { credentials: 'include', ...init });
  if (!response.ok) {
    let code = 'error';
    try {
      const body: unknown = await response.json();
      if (body && typeof body === 'object' && 'error' in body && typeof body.error === 'string') code = body.error;
    } catch {
      // corpo assente o non JSON: resta il codice generico
    }
    const retry = Number(response.headers.get('retry-after'));
    throw new ApiError(response.status, code, Number.isFinite(retry) && retry > 0 ? retry : undefined);
  }
  return response;
}

function jsonInit(body: unknown): RequestInit {
  return { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) };
}

export async function getHealth(): Promise<Health> {
  return HealthSchema.parse(await (await request('/health')).json());
}

/** Il dispositivo e' associato? Solo un 401 vuol dire "no"; ogni altro errore e' un problema di rete. */
export async function checkSession(): Promise<boolean> {
  const response = await fetch(`${BASE}/devices`, { credentials: 'include' });
  if (response.status === 401) return false;
  if (!response.ok) throw new ApiError(response.status, 'error');
  return true;
}

export async function postHalt(): Promise<void> {
  await request('/commands/halt-tx', { method: 'POST' });
}

export async function postReply(decodeId: number): Promise<void> {
  await request('/commands/reply', jsonInit({ decode_id: decodeId }));
}

export async function finishPairing(code: string, deviceName: string): Promise<void> {
  await request('/pair/finish', jsonInit({ code, device_name: deviceName }));
}

export async function getAutomation(): Promise<AutomationState> {
  return AutomationStateSchema.parse(await (await request('/automation')).json());
}

export interface AutomationChange {
  enabled?: boolean;
  mode?: 'dry_run' | 'live';
  rules?: AutomationRules;
  accept_license_notice?: boolean;
}

export async function putAutomation(change: AutomationChange): Promise<AutomationState> {
  const init = { ...jsonInit(change), method: 'PUT' };
  return AutomationStateSchema.parse(await (await request('/automation', init)).json());
}

export interface StationInfo {
  engine?: string;
  tx_allowed: boolean;
  rig_control?: boolean;
  udp_listen: string;
  udp_forward: string[];
  export_udp: string[];
  export_file: string | null;
  audio_device: string | null;
  cty_loaded: number;
  worked_qsos: number;
  limits: { max_consecutive_cycles: number; max_per_day: number; dead_man_secs: number; run_unattended: boolean };
}

export async function getStationInfo(): Promise<StationInfo> {
  return (await (await request('/station-info')).json()) as StationInfo;
}

export async function postFreeText(text: string, send: boolean): Promise<void> {
  await request('/commands/free-text', jsonInit({ text, send }));
}

export async function postCq(modifier: string): Promise<void> {
  await request('/commands/cq', jsonInit({ modifier }));
}

/** Sceglie a mano il messaggio Tx (1-6) da cui il motore parte, come il pallino accanto ai messaggi. */
export async function postTxMessage(n: number): Promise<void> {
  await request('/commands/tx-message', jsonInit({ n }));
}

export async function postTxFrequency(hz: number): Promise<void> {
  await request('/commands/tx-frequency', jsonInit({ hz }));
}

/** Risponde con la frequenza impostata (`dial_hz`), cosi' la pagina la mostra subito. */
export async function postBand(band: string): Promise<{ dial_hz?: number }> {
  return (await (await request('/commands/band', jsonInit({ band }))).json()) as { dial_hz?: number };
}

/** Porta la radio su una frequenza esatta (Hz), come per uno spot del cluster. */
export async function postFrequency(hz: number): Promise<{ dial_hz?: number }> {
  return (await (await request('/commands/frequency', jsonInit({ hz }))).json()) as { dial_hz?: number };
}

export async function postMode(mode: string): Promise<void> {
  await request('/commands/mode', jsonInit({ mode }));
}

export async function postRxFrequency(hz: number): Promise<void> {
  await request('/commands/rx-frequency', jsonInit({ hz }));
}

export interface FrequencyEntry {
  band: string;
  mode: string;
  hz: number;
}

export async function getFrequencies(): Promise<FrequencyEntry[]> {
  return (await (await request('/frequencies')).json()) as FrequencyEntry[];
}

export async function putFrequencies(entries: FrequencyEntry[]): Promise<void> {
  await request('/frequencies', { ...jsonInit(entries), method: 'PUT' });
}

export interface ScheduleEntry {
  hour: number;
  minute: number;
  band: string;
}

export interface ScheduleState {
  enabled: boolean;
  entries: ScheduleEntry[];
}

export async function getSchedule(): Promise<ScheduleState> {
  return (await (await request('/schedule')).json()) as ScheduleState;
}

export async function putSchedule(state: ScheduleState): Promise<void> {
  await request('/schedule', { ...jsonInit(state), method: 'PUT' });
}

export async function getAccess(): Promise<{ open: boolean }> {
  return (await (await request('/access')).json()) as { open: boolean };
}

export async function putAccess(open: boolean): Promise<void> {
  await request('/access', { ...jsonInit({ open }), method: 'PUT' });
}

export interface PairingSettings {
  code_ttl_secs: number;
  min_secs: number;
  max_secs: number;
}

export async function getPairingSettings(): Promise<PairingSettings> {
  return (await (await request('/pairing-settings')).json()) as PairingSettings;
}

export async function putPairingSettings(codeTtlSecs: number): Promise<void> {
  await request('/pairing-settings', { ...jsonInit({ code_ttl_secs: codeTtlSecs }), method: 'PUT' });
}

export interface PairedDevice {
  id: number;
  name: string;
  created_at: number;
}

export async function getDevices(): Promise<PairedDevice[]> {
  return (await (await request('/devices')).json()) as PairedDevice[];
}

export async function revokeDevice(id: number): Promise<void> {
  await request(`/devices/${id}`, { method: 'DELETE' });
}

export interface PairingOffer {
  code: string;
  expires_in: number;
  qr_svg: string;
}

/** Un codice nuovo con il QR; il link punta all'indirizzo da cui si sta guardando la pagina. */
export async function startPairing(baseUrl: string): Promise<PairingOffer> {
  return (await (await request('/pair/start', jsonInit({ base_url: baseUrl }))).json()) as PairingOffer;
}

export interface OutputLevelState {
  attenuation_db: number;
  min_db: number;
  max_db: number;
}

export async function getOutputLevel(): Promise<OutputLevelState> {
  return (await (await request('/output-level')).json()) as OutputLevelState;
}

export async function putOutputLevel(attenuationDb: number): Promise<void> {
  await request('/output-level', { ...jsonInit({ attenuation_db: attenuationDb }), method: 'PUT' });
}

/** Il volume di registrazione di Windows dell'ingresso audio e la regolazione automatica (AUTO). */
export interface InputLevelState {
  device: string;
  volume_pct: number;
  auto: boolean;
}

export async function getInputLevel(): Promise<InputLevelState> {
  return (await (await request('/input-level')).json()) as InputLevelState;
}

/** Con un volume scelto a mano AUTO si spegne, salvo che lo stesso invio lo riaccenda. */
export async function putInputLevel(change: { volume_pct?: number; auto?: boolean }): Promise<InputLevelState> {
  const body = (await (await request('/input-level', { ...jsonInit(change), method: 'PUT' })).json()) as { status: InputLevelState };
  return body.status;
}

/** Un messaggio dello storico (la riga com'e' nel core). */
export interface HistoryRow {
  id: number;
  ts_ms: number;
  band: string | null;
  mode: string;
  dial_hz: number;
  freq_hz: number;
  snr: number;
  dt: number;
  text: string;
  call: string | null;
  country: string | null;
  continent: string | null;
  tag: string | null;
}

export interface HistoryPage {
  total: number;
  rows: HistoryRow[];
}

export interface HistoryStats {
  total: number;
  oldest_ms: number | null;
  calls: number;
}

export async function getHistory(filter: HistoryFilter, offset: number, limit: number): Promise<HistoryPage> {
  const params = historyParams(filter, offset, limit, Date.now());
  return (await (await request(`/history?${params.toString()}`)).json()) as HistoryPage;
}

export async function getHistoryStats(): Promise<HistoryStats> {
  return (await (await request('/history/stats')).json()) as HistoryStats;
}

export async function clearHistory(): Promise<void> {
  await request('/history', { method: 'DELETE' });
}

/** Quali servizi online sono accesi (PSK Reporter, propagazione). */
export interface OnlineSettings {
  psk: boolean;
  solar: boolean;
}

export interface PskReport {
  receiver: string;
  locator: string;
  snr: number | null;
  freq_hz: number;
  mode: string;
  /** Secondi Unix. */
  time_s: number;
  country: string | null;
}

export interface PskState {
  enabled: boolean;
  psk: { fetched_ms: number; callsign: string; hours: number; reports: PskReport[]; error: string | null };
}

export interface SolarData {
  updated: string;
  sfi: number | null;
  a_index: number | null;
  k_index: number | null;
  xray: string;
  sunspots: number | null;
  aurora: number | null;
  solar_wind: number | null;
  magnetic_field: number | null;
  geomag: string;
  signal_noise: string;
  muf: string;
  bands: Array<{ name: string; time: string; condition: string }>;
  vhf: Array<{ name: string; location: string; status: string }>;
}

export interface SolarState {
  enabled: boolean;
  solar: { fetched_ms: number; data: SolarData | null; error: string | null };
}

export async function getOnline(): Promise<OnlineSettings> {
  return (await (await request('/online')).json()) as OnlineSettings;
}

export async function putOnline(settings: OnlineSettings): Promise<OnlineSettings> {
  return (await (await request('/online', { ...jsonInit(settings), method: 'PUT' })).json()) as OnlineSettings;
}

export async function getPsk(): Promise<PskState> {
  return (await (await request('/online/psk')).json()) as PskState;
}

export async function getSolar(): Promise<SolarState> {
  return (await (await request('/online/solar')).json()) as SolarState;
}

/** Chiede subito i dati ai servizi accesi (al massimo una volta al minuto per servizio). */
export async function refreshOnline(): Promise<{ asked: boolean }> {
  return (await (await request('/online/refresh', { method: 'POST' })).json()) as { asked: boolean };
}

export interface AwardStats {
  qsos: number;
  dxcc_worked: number;
  dxcc_total: number;
  cq_zones_worked: number;
  cq_zones_total: number;
  itu_zones_worked: number;
  itu_zones_total: number;
  grids: number;
  prefixes: number;
  calls: number;
  bands: Array<{ band: string; dxcc: number; qsos: number }>;
}

export interface StatsResponse {
  stats: AwardStats;
  missing_dxcc: string[];
}

export interface ClusterServer {
  name: string;
  host: string;
  port: number;
  enabled: boolean;
}

export interface ClusterConfig {
  enabled: boolean;
  callsign: string;
  servers: ClusterServer[];
  login_commands: string[];
  max_spots: number;
  max_age_min: number;
}

export interface ClusterSpot {
  id: number;
  received_ms: number;
  server: string;
  spotter: string;
  freq_khz: number;
  dx_call: string;
  comment: string;
  time_utc: string;
  grid: string | null;
  band: string | null;
  mode: string | null;
  country: string | null;
  continent: string | null;
}

export interface ClusterServerStatus {
  key: string;
  name: string;
  state: 'connecting' | 'connected' | 'error';
  detail: string | null;
  spots: number;
  since_ms: number;
}

export interface ClusterCatalogEntry {
  name: string;
  host: string;
  port: number;
  software: string;
  place: string;
  note: string;
}

/** Impostazioni, stato dei collegamenti e spot recenti del DX Cluster (`limit` spot al massimo). */
export async function getClusterSpots(limit = 200): Promise<{ config: ClusterConfig; servers: ClusterServerStatus[]; spots: ClusterSpot[] }> {
  return (await (await request(`/dx-cluster?limit=${limit}`)).json()) as { config: ClusterConfig; servers: ClusterServerStatus[]; spots: ClusterSpot[] };
}

export async function putCluster(config: ClusterConfig): Promise<void> {
  await request('/dx-cluster', { ...jsonInit(config), method: 'PUT' });
}

/** Paese e zone di un nominativo, dal file dei prefissi (cty.dat); 404 se non lo conosce. */
export async function getDxcc(call: string): Promise<{ country: string; continent: string; prefix: string; cq_zone: number; itu_zone: number }> {
  return (await (await request(`/dxcc/${encodeURIComponent(call)}`)).json()) as { country: string; continent: string; prefix: string; cq_zone: number; itu_zone: number };
}

/** Prova un server: si collega e legge il benvenuto, senza dire chi siamo. */
export async function testClusterServer(host: string, port: number): Promise<{ ok: boolean; ms: number | null; banner: string | null; error: string | null }> {
  return (await (await request('/dx-cluster/test', jsonInit({ host, port }))).json()) as { ok: boolean; ms: number | null; banner: string | null; error: string | null };
}

/** I server DX Cluster pubblici noti. */
export async function getClusterCatalog(): Promise<ClusterCatalogEntry[]> {
  return (await (await request('/dx-cluster/catalog')).json()) as ClusterCatalogEntry[];
}

/** Manda un comando di consultazione a un cluster collegato (mai spot o messaggi). */
export async function sendClusterCommand(server: string, text: string): Promise<void> {
  await request('/dx-cluster/command', jsonInit({ server, text }));
}

export interface ClockConfig {
  enabled: boolean;
  servers: string[];
  interval_secs: number;
  warn_ms: number;
  auto_correct: boolean;
}

export interface ClockMeasurement {
  at_ms: number;
  server: string | null;
  offset_ms: number | null;
  delay_ms: number | null;
  servers_ok: number;
  servers_tried: number;
  error: string | null;
  corrected: boolean;
}

/** Client NTP: impostazioni e ultima misura dell'orologio del PC. */
export async function getClock(): Promise<{ config: ClockConfig; last: ClockMeasurement | null }> {
  return (await (await request('/clock')).json()) as { config: ClockConfig; last: ClockMeasurement | null };
}

export async function putClock(config: ClockConfig): Promise<void> {
  await request('/clock', { ...jsonInit(config), method: 'PUT' });
}

/** Misura subito lo scarto dell'orologio. */
export async function checkClock(): Promise<ClockMeasurement> {
  return (await (await request('/clock/check', { method: 'POST' })).json()) as ClockMeasurement;
}

/** Sposta l'ora di Windows dello scarto misurato (serve l'amministratore, solo dal PC di casa). */
export async function correctClock(): Promise<ClockMeasurement> {
  return (await (await request('/clock/correct', { method: 'POST' })).json()) as ClockMeasurement;
}

/** Le impostazioni del motore, per nome, come testo (dal suo file di configurazione). */
export async function getEngineConfig(): Promise<Record<string, string>> {
  return (await (await request('/engine-config')).json()) as Record<string, string>;
}

/** Applica modifiche alle impostazioni del motore: lo chiude, scrive il suo file (con copia di sicurezza) e lo riavvia. */
export async function applyEngineConfig(changes: Record<string, string>): Promise<void> {
  await request('/engine-config', jsonInit({ changes }));
}

export interface Log4omQso {
  date: string;
  call: string;
  band: string;
  mode: string;
  rst_sent: string;
  rst_rcvd: string;
  country: string;
  grid: string;
}

export interface Log4omView {
  total: number;
  qsos: Log4omQso[];
}

export async function getLog4om(limit: number): Promise<Log4omView> {
  return (await (await request(`/log4om?limit=${limit}`)).json()) as Log4omView;
}

/** Una versione pubblicata di Aurora sul repository. */
export interface UpdateRelease {
  tag: string;
  version: string;
  name: string;
  notes: string;
  url: string;
  published_at: string;
  prerelease: boolean;
}

/** Stato degli aggiornamenti: versione in uso, impostazioni (senza il token) e ultimo esito. */
export interface UpdateStatus {
  current: string;
  enabled: boolean;
  repo: string;
  interval_hours: number;
  prereleases: boolean;
  has_token: boolean;
  checked_at_ms: number;
  available: UpdateRelease | null;
  latest: UpdateRelease | null;
  error: string | null;
}

/** Le impostazioni che si mandano: `token` assente = lo si lascia, vuoto = lo si toglie. */
export interface UpdateConfigInput {
  enabled: boolean;
  repo: string;
  interval_hours: number;
  prereleases: boolean;
  token?: string;
}

export async function getUpdate(): Promise<UpdateStatus> {
  return (await (await request('/update')).json()) as UpdateStatus;
}

export async function putUpdate(input: UpdateConfigInput): Promise<UpdateStatus> {
  return (await (await request('/update', { ...jsonInit(input), method: 'PUT' })).json()) as UpdateStatus;
}

/** Controlla subito se c'e' una versione nuova (serve internet). */
export async function checkUpdate(): Promise<UpdateStatus> {
  return (await (await request('/update/check', { method: 'POST' })).json()) as UpdateStatus;
}

/** La lista dei desiderati e gli ultimi avvisi. */
export async function getWishlist(): Promise<{ config: Wishlist; recent: WantedAlert[] }> {
  return (await (await request('/wishlist')).json()) as { config: Wishlist; recent: WantedAlert[] };
}

export async function putWishlist(config: Wishlist): Promise<Wishlist> {
  const body = (await (await request('/wishlist', { ...jsonInit(config), method: 'PUT' })).json()) as { config: Wishlist };
  return body.config;
}

/** Manda un avviso di prova; `phone` dice l'esito verso il telefono: `no_url`, `sent` o `failed`. */
export async function testWishlist(): Promise<'no_url' | 'sent' | 'failed'> {
  const body = (await (await request('/wishlist/test', { method: 'POST' })).json()) as { phone: 'no_url' | 'sent' | 'failed' };
  return body.phone;
}

/** Un quadrato locatore (4 caratteri) con QSO nel log: quanti e su quali bande. */
export interface WorkedSquare {
  grid: string;
  count: number;
  bands: string[];
}

/** I quadrati locatore gia' lavorati, dal log di Log4OM2 (per la mappa). */
export async function getWorkedMap(): Promise<{ squares: WorkedSquare[] }> {
  return (await (await request('/map/worked')).json()) as { squares: WorkedSquare[] };
}

export async function getStats(): Promise<StatsResponse> {
  return (await (await request('/stats')).json()) as StatsResponse;
}

export type ControlId =
  | 'autoseq' | 'tx_period' | 'single_qso' | 'answer_b4' | 'bypass' | 'skip_tx1' | 'rrr' | 'split' | 'show_wanted'
  | 'auto_tx' | 'enable_tx' | 'hound' | 'jt' | 'monitor' | 'swl' | 'sync' | 'agc' | 'filter' | 'decode' | 'erase' | 'stop_rx' | 'clear_dx' | 'gen_msgs' | 'rx_to_tx' | 'tx_to_rx';

/** Stato degli interruttori del motore (AutoSeq, 1 QSO, Abilita Tx...). */
export async function getControls(): Promise<Partial<Record<ControlId, boolean>>> {
  const body = (await (await request('/controls')).json()) as { controls?: Partial<Record<ControlId, boolean>>; tx?: unknown };
  // la stessa lettura porta anche i sei messaggi Tx del motore e quello selezionato
  useEngineTx.getState().set(parseEngineTx(body.tx));
  return body.controls ?? {};
}

/** Porta un interruttore allo stato voluto, o preme un pulsante semplice (senza `on`). */
export async function postControl(id: ControlId, on?: boolean): Promise<void> {
  await request('/commands/control', jsonInit(on === undefined ? { id } : { id, on }));
}
