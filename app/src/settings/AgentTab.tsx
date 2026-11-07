import { useState } from 'react';
import { useT } from '../i18n';
import { showAgentPopup } from '../components/AgentPopup';
import { browserStorage } from '../lib/theme';
import { Check, Help } from './common';
import { Cells, Group, Notes, Page } from './layout';

/** Come compare ogni avviso dell'agente nel tray. Gli aggiornamenti non sono qui: compaiono sempre (scelta fissa). */
type Show = 'popup' | 'icon' | 'off';
type Kind = 'news' | 'prop' | 'dxped' | 'daily';

interface AgentPrefs {
  enabled: boolean;
  show: Record<Kind, Show>;
  seconds: number;
  quietFrom: string;
  quietTo: string;
  quietOn: boolean;
}

const KEY = 'aurora-agente';
const KINDS: readonly Kind[] = ['news', 'prop', 'dxped', 'daily'];
const SHOWS: readonly Show[] = ['popup', 'icon', 'off'];
const DEFAULTS: AgentPrefs = {
  enabled: true,
  show: { news: 'popup', prop: 'icon', dxped: 'popup', daily: 'off' },
  seconds: 10,
  quietFrom: '22:00',
  quietTo: '07:00',
  quietOn: false,
};

function load(): AgentPrefs {
  try {
    const raw: unknown = JSON.parse(browserStorage()?.getItem(KEY) ?? 'null');
    if (typeof raw !== 'object' || raw === null) return DEFAULTS;
    const r = raw as Partial<AgentPrefs>;
    return { ...DEFAULTS, ...r, show: { ...DEFAULTS.show, ...(r.show ?? {}) } };
  } catch {
    return DEFAULTS;
  }
}

/**
 * «Agente nel tray» (prototipo): per ogni tipo di avviso si sceglie se compare come popup, solo sull'icona del tray o mai.
 * Le scelte si salvano con le altre impostazioni dell'interfaccia (`ui-settings.json`): l'agente le rilegge ogni minuto.
 */
export function AgentTab() {
  const t = useT();
  const [prefs, setPrefs] = useState<AgentPrefs>(load);
  const set = (change: Partial<AgentPrefs>) => {
    const next = { ...prefs, ...change };
    setPrefs(next);
    try {
      browserStorage()?.setItem(KEY, JSON.stringify(next));
    } catch {
      // senza archivio la scelta vale finche' la finestra resta aperta
    }
  };
  const setShow = (kind: Kind, show: Show) => set({ show: { ...prefs.show, [kind]: show } });

  return (
    <Page>
      <Group title={t('agent.title')} span={2}>
        <Cells min={17}>
          <Check label={t('agent.enabled')} hint={t('agent.enabledTip')} checked={prefs.enabled} onChange={(v) => set({ enabled: v })} />
        </Cells>
        <div className={`agent-table${prefs.enabled ? '' : ' agent-table--off'}`} role="table" aria-label={t('agent.title')}>
          <div className="agent-row agent-row--head" role="row">
            <span role="columnheader">{t('agent.kind')}</span>
            {SHOWS.map((s) => (
              <span key={s} role="columnheader">
                {t(`agent.show.${s}`)}
              </span>
            ))}
          </div>
          {KINDS.map((k) => (
            <div key={k} className="agent-row" role="row">
              <span role="rowheader">
                <b>{t(`agent.kind.${k}`)}</b>
                <small className="muted">{t(`agent.kind.${k}.tip`)}</small>
              </span>
              {SHOWS.map((s) => (
                <span key={s} role="cell">
                  <input
                    type="radio"
                    name={`agent-${k}`}
                    aria-label={`${t(`agent.kind.${k}`)}: ${t(`agent.show.${s}`)}`}
                    checked={prefs.show[k] === s}
                    disabled={!prefs.enabled}
                    onChange={() => setShow(k, s)}
                  />
                </span>
              ))}
            </div>
          ))}
        </div>
      </Group>

      <Group title={t('agent.popup')} span={2}>
        {/* stesse righe della tabella sopra: a sinistra cosa, a destra il comando */}
        <div className={`agent-opts${prefs.enabled ? '' : ' agent-table--off'}`}>
          <div className="agent-opt">
            <span>
              <b>{t('agent.seconds')}</b>
              <small className="muted">{t('agent.secondsTip')}</small>
            </span>
            <span className="agent-opt-ctl">
              <input
                type="number"
                className="agent-input agent-num"
                min={3}
                max={60}
                value={prefs.seconds}
                disabled={!prefs.enabled}
                aria-label={t('agent.seconds')}
                onChange={(e) => set({ seconds: Math.max(3, Math.min(60, Number(e.target.value) || 10)) })}
              />
              <span className="muted">s</span>
            </span>
          </div>
          <div className="agent-opt">
            <label className="agent-opt-check">
              <input type="checkbox" checked={prefs.quietOn} disabled={!prefs.enabled} onChange={(e) => set({ quietOn: e.target.checked })} />
              <span>
                <b>{t('agent.quiet')}</b>
                <small className="muted">{t('agent.quietTip')}</small>
              </span>
            </label>
            <span className={`agent-opt-ctl${prefs.quietOn ? '' : ' agent-opt-ctl--off'}`}>
              <span className="muted">{t('agent.quietFrom')}</span>
              <input type="time" className="agent-input agent-time" value={prefs.quietFrom} aria-label={t('agent.quietFrom')} disabled={!prefs.enabled || !prefs.quietOn} onChange={(e) => set({ quietFrom: e.target.value })} />
              <span className="muted">{t('agent.quietTo')}</span>
              <input type="time" className="agent-input agent-time" value={prefs.quietTo} aria-label={t('agent.quietTo')} disabled={!prefs.enabled || !prefs.quietOn} onChange={(e) => set({ quietTo: e.target.value })} />
            </span>
          </div>
          <div className="agent-opt">
            <span>
              <b>{t('agent.preview')}</b>
              <small className="muted">{t('agent.previewTip')}</small>
            </span>
            <span className="agent-opt-ctl">
              <button
                type="button"
                className="glass"
                onClick={() =>
                  showAgentPopup(
                    { kind: t('agent.try.updKind'), title: t('agent.try.updTitle'), text: t('agent.try.updText'), primary: t('agent.try.updMain'), secondary: t('agent.try.later'), update: true },
                    prefs.seconds,
                  )
                }
              >
                {t('agent.try.update')}
              </button>
              <button
                type="button"
                className="glass"
                onClick={() =>
                  showAgentPopup(
                    { kind: t('agent.kind.dxped'), title: t('agent.try.dxTitle'), text: t('agent.try.dxText'), primary: t('agent.try.dxMain'), secondary: t('agent.try.ignore') },
                    prefs.seconds,
                  )
                }
              >
                {t('agent.try.dx')}
              </button>
            </span>
          </div>
        </div>
      </Group>

      <Notes>
        <Help>{t('agent.intro')}</Help>
      </Notes>
    </Page>
  );
}
