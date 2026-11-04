import { useEffect, useState } from 'react';
import type { EngineSettings } from '../api/schemas';
import { useT, type Key } from '../i18n';
import { Cells, Field, Group, Help, Message } from './layout';
import type { Draft } from './NativeSettings';

/** La voce delle impostazioni con l'elenco dei destinatari (JSON). */
export const DESTINATIONS_KEY = 'UdpDestinations';
/** Quanti destinatari al massimo (come il motore). */
export const MAX_DESTINATIONS = 16;

/** Un destinatario UDP con le sue opzioni: stessi nomi del motore (`engine_settings::UdpDestination`). */
export interface UdpDestination {
  name: string;
  host: string;
  port: number;
  enabled: boolean;
  wsjtx: boolean;
  loggedAdif: boolean;
  adif: boolean;
  n1mm: boolean;
  multicast: boolean;
  ttl: number;
  iface: string;
}

const BLANK: UdpDestination = { name: '', host: '127.0.0.1', port: 2237, enabled: true, wsjtx: false, loggedAdif: false, adif: false, n1mm: false, multicast: false, ttl: 1, iface: '' };

/** I tipi proposti quando si aggiunge un destinatario: valori di partenza comuni. */
const PRESETS: Record<string, { label: Key; value: Partial<UdpDestination> }> = {
  wsjtx: { label: 'int.p.wsjtx', value: { name: 'GridTracker', host: '127.0.0.1', port: 2237, wsjtx: true } },
  bblogger: { label: 'int.p.bblogger', value: { name: 'BBLogger', host: '127.0.0.1', port: 2237, wsjtx: true, loggedAdif: true } },
  multicast: { label: 'int.p.multicast', value: { name: 'Multicast', host: '239.255.0.1', port: 2237, wsjtx: true, multicast: true, ttl: 1, iface: '127.0.0.1' } },
  adif: { label: 'int.p.adif', value: { name: 'Log4OM', host: '127.0.0.1', port: 2333, adif: true } },
  n1mm: { label: 'int.p.n1mm', value: { name: 'N1MM XML', host: '127.0.0.1', port: 12060, n1mm: true } },
};

/** Legge l'elenco salvato; un testo non leggibile vale come elenco vuoto. */
export function parseDestinations(text: string | undefined): UdpDestination[] {
  try {
    const raw: unknown = JSON.parse(text && text.trim() !== '' ? text : '[]');
    if (!Array.isArray(raw)) return [];
    return raw.filter((d): d is Record<string, unknown> => typeof d === 'object' && d !== null).map((d) => ({ ...BLANK, ...d }) as UdpDestination);
  } catch {
    return [];
  }
}

const IPV4 = /^(25[0-5]|2[0-4]\d|1?\d?\d)(\.(25[0-5]|2[0-4]\d|1?\d?\d)){3}$/;
const isIpv4 = (h: string) => IPV4.test(h.trim());
const isIpv6 = (h: string) => /^[0-9a-f:.]+$/i.test(h.trim()) && h.includes(':');
const isHost = (h: string) => h.trim().toLowerCase() === 'localhost' || isIpv4(h) || isIpv6(h);
const isGroup = (h: string) => (isIpv4(h) ? Number(h.trim().split('.')[0]) >= 224 && Number(h.trim().split('.')[0]) <= 239 : isIpv6(h) && h.trim().toLowerCase().startsWith('ff'));

/** Il primo errore di un destinatario (la stessa regola del motore), o `null` se va bene. */
export function destinationError(d: UdpDestination): Key | null {
  if (d.name.length > 40) return 'int.err.name';
  if (!isHost(d.host)) return 'int.err.host';
  if (!Number.isInteger(d.port) || d.port < 1 || d.port > 65535) return 'int.err.port';
  if (d.multicast) {
    if (!isGroup(d.host)) return 'int.err.group';
    if (!Number.isInteger(d.ttl) || d.ttl < 1 || d.ttl > 255) return 'int.err.ttl';
    if (d.iface.trim() !== '' && !(isIpv4(d.iface) && isIpv4(d.host))) return 'int.err.iface';
  } else if (isGroup(d.host)) {
    return 'int.err.unicast';
  }
  return null;
}

/** Il riquadro «Integrazioni» della scheda Segnalazioni: un destinatario per riga con le sue scelte; si salva da solo con le altre voci. */
export function IntegrationsPanel({ doc, draft }: { doc: EngineSettings; draft: Draft }) {
  const t = useT();
  const current = doc.values[DESTINATIONS_KEY] ?? '[]';
  const pending = draft.changes[DESTINATIONS_KEY];
  const [rows, setRows] = useState<UdpDestination[]>(() => parseDestinations(pending ?? current));
  // dopo il salvataggio (o un cambio da un'altra pagina) l'elenco mostrato riparte da quello salvato
  useEffect(() => {
    if (pending === undefined) setRows(parseDestinations(current));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [current]);
  const live = doc.items[DESTINATIONS_KEY]?.state === 'live';

  const update = (next: UdpDestination[]) => {
    setRows(next);
    // solo un elenco tutto valido va al salvataggio automatico: un indirizzo a meta' non diventa un errore del motore
    if (next.every((d) => destinationError(d) === null)) draft.set(DESTINATIONS_KEY, JSON.stringify(next), current);
  };
  const change = (i: number, patch: Partial<UdpDestination>) => update(rows.map((d, j) => (j === i ? { ...d, ...patch } : d)));
  const invalid = rows.some((d) => destinationError(d) !== null);

  return (
    <Group
      title={
        <>
          {t('int.title')}
          {/* la spiegazione lunga e l'avviso sui doppioni stanno nel «?», non sotto il riquadro */}
          <Help tip={`${t('int.help')} ${t('int.doubles')}`} />
        </>
      }
      span={2}
    >
      {!live ? <Message kind="muted">{t(`ens.r.${doc.items[DESTINATIONS_KEY]?.reason ?? 'own_decoder'}` as Key)}</Message> : null}
      {live ? (
        <div className="int-add">
          {/* un pulsante per ogni tipo di programma, con i valori di partenza giusti: niente menu a tendina da capire */}
          <span className="muted">{rows.length === 0 ? t('int.empty') : t('int.addMore')}</span>
          {Object.entries(PRESETS).map(([id, p]) => (
            <button
              key={id}
              type="button"
              className="glass"
              disabled={rows.length >= MAX_DESTINATIONS}
              title={rows.length >= MAX_DESTINATIONS ? t('int.max', { n: MAX_DESTINATIONS }) : t(`int.tip.${id}` as Key)}
              onClick={() => update([...rows, { ...BLANK, ...(p.value ?? {}) }])}
            >
              + {t(p.label)}
            </button>
          ))}
        </div>
      ) : null}
      {live
        ? rows.map((d, i) => {
            const error = destinationError(d);
            return (
              <fieldset key={i} className="int-dest" aria-label={d.name || d.host}>
                <legend>
                  {d.name || `${d.host}:${d.port}`}{' '}
                  <button title={t('tip.ui.remove')} type="button" className="glass int-remove" onClick={() => update(rows.filter((_, j) => j !== i))}>
                    {t('int.remove')}
                  </button>
                </legend>
                <Cells min={13}>
                  <Field label={t('int.name')} hint={t('int.name.help')}>
                    <input type="text" className="jx-text" maxLength={40} value={d.name} onChange={(e) => change(i, { name: e.target.value })} />
                  </Field>
                  <Field label={t('int.host')} hint={t('int.host.help')}>
                    <input type="text" className="jx-text" maxLength={60} value={d.host} aria-invalid={error === 'int.err.host' || error === 'int.err.group' || error === 'int.err.unicast'} onChange={(e) => change(i, { host: e.target.value.trim() })} />
                  </Field>
                  <Field label={t('int.port')} hint={t('int.port.help')}>
                    <input type="number" className="set-number" min={1} max={65535} value={Number.isFinite(d.port) ? d.port : ''} aria-invalid={error === 'int.err.port'} onChange={(e) => change(i, { port: e.target.valueAsNumber })} />
                  </Field>
                  <Field label={t('int.enabled')} hint={t('int.enabled.help')}>
                    <input type="checkbox" checked={d.enabled} onChange={(e) => change(i, { enabled: e.target.checked })} />
                  </Field>
                  <Field label={t('int.wsjtx')} hint={t('int.wsjtx.help')}>
                    <input type="checkbox" checked={d.wsjtx} onChange={(e) => change(i, { wsjtx: e.target.checked })} />
                  </Field>
                  <Field label={t('int.loggedAdif')} hint={t('int.loggedAdif.help')}>
                    <input type="checkbox" checked={d.loggedAdif} onChange={(e) => change(i, { loggedAdif: e.target.checked })} />
                  </Field>
                  <Field label={t('int.adif')} hint={t('int.adif.help')}>
                    <input type="checkbox" checked={d.adif} onChange={(e) => change(i, { adif: e.target.checked })} />
                  </Field>
                  <Field label={t('int.n1mm')} hint={t('int.n1mm.help')}>
                    <input type="checkbox" checked={d.n1mm} onChange={(e) => change(i, { n1mm: e.target.checked })} />
                  </Field>
                  <Field label={t('int.multicast')} hint={t('int.multicast.help')}>
                    <input type="checkbox" checked={d.multicast} onChange={(e) => change(i, { multicast: e.target.checked })} />
                  </Field>
                  {d.multicast ? (
                    <>
                      <Field label={t('int.ttl')} hint={t('int.ttl.help')}>
                        <input type="number" className="set-number" min={1} max={255} value={Number.isFinite(d.ttl) ? d.ttl : ''} aria-invalid={error === 'int.err.ttl'} onChange={(e) => change(i, { ttl: e.target.valueAsNumber })} />
                      </Field>
                      <Field label={t('int.iface')} hint={t('int.iface.help')}>
                        <input type="text" className="jx-text" maxLength={45} value={d.iface} placeholder="127.0.0.1" aria-invalid={error === 'int.err.iface'} onChange={(e) => change(i, { iface: e.target.value.trim() })} />
                      </Field>
                    </>
                  ) : null}
                </Cells>
                {error ? <Message kind="alert">{t(error)}</Message> : null}
              </fieldset>
            );
          })
        : null}
      {invalid ? <Message kind="muted">{t('int.err.wait')}</Message> : null}
    </Group>
  );
}
