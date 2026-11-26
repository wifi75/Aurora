import { useState } from 'react';
import { useT } from '../i18n';
import {
  bandName,
  effectivePresets,
  isFactoryId,
  normalizeEntry,
  removeEntry,
  resetWefaxFreq,
  saveEntry,
  useWefaxFreq,
  validateEntry,
  writeWefaxFreq,
  BAND_MAX,
  CALL_MAX,
  PLACE_MAX,
} from '../lib/wefaxFreqs';
import { formatKhz, wefaxGroups, type WefaxPreset } from '../lib/wefaxStations';
import { Actions, Cells, Field, Group, Message, Page } from './layout';

interface Draft {
  call: string;
  place: string;
  khz: string;
  band: string;
}
const EMPTY: Draft = { call: '', place: '', khz: '', band: '' };

/**
 * Impostazioni › Modi › WEFAX › Frequenze: la tabella delle frequenze del menu della pagina WEFAX. Si cambia, si cancella e si aggiunge;
 * le voci di fabbrica cancellate si nascondono e tornano con «Ripristina quelle di fabbrica». Si salva subito (chiave `aurora-wefax-freq`).
 */
export function WefaxFreqTab() {
  const t = useT();
  const state = useWefaxFreq();
  const presets = effectivePresets(state);
  const groups = wefaxGroups(presets);
  const [draft, setDraft] = useState<Draft>(EMPTY);
  const [editing, setEditing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const bands = [...new Set(groups.map((g) => g.label ?? bandName(g.presets[0] ?? { band: g.band })))];

  const set = (patch: Partial<Draft>) => {
    setDraft((d) => ({ ...d, ...patch }));
    setError(null);
  };
  const cancel = () => {
    setDraft(EMPTY);
    setEditing(null);
    setError(null);
  };
  const submit = () => {
    const err = validateEntry(draft, presets, editing);
    if (err) {
      setError(t(`wefax.fl.err.${err}`));
      return;
    }
    // una banda scritta uguale a quella automatica (p. es. «7 MHz» per 7880 kHz) non serve come etichetta
    const entry = normalizeEntry(draft);
    if (entry.band && entry.band === bandName({ band: Math.floor(entry.khz / 1000) })) delete entry.band;
    writeWefaxFreq(saveEntry(state, editing, entry));
    cancel();
  };
  const edit = (p: WefaxPreset) => {
    setEditing(p.id);
    setDraft({ call: p.call, place: p.place, khz: String(p.khz), band: p.bandLabel ?? '' });
    setError(null);
  };
  const remove = (p: WefaxPreset) => {
    writeWefaxFreq(removeEntry(state, p.id));
    if (editing === p.id) cancel();
  };
  const reset = () => {
    resetWefaxFreq();
    cancel();
  };

  return (
    <Page>
      <Group title={t('wefax.fl.title')} span={3}>
        <Message kind="muted">{t('wefax.fl.intro')}</Message>
        <form
          className="wefax-fl-form"
          onSubmit={(e) => {
            e.preventDefault();
            submit();
          }}
        >
          <Cells min={14}>
            <Field label={t('wefax.fl.call')}>
              <input type="text" maxLength={CALL_MAX} value={draft.call} onChange={(e) => set({ call: e.target.value })} />
            </Field>
            <Field label={t('wefax.fl.place')}>
              <input type="text" maxLength={PLACE_MAX} value={draft.place} onChange={(e) => set({ place: e.target.value })} />
            </Field>
            <Field label={t('wefax.fl.khz')}>
              <input type="text" inputMode="decimal" value={draft.khz} onChange={(e) => set({ khz: e.target.value })} />
            </Field>
            <Field label={t('wefax.fl.band')}>
              <input type="text" list="wefax-fl-bands" maxLength={BAND_MAX} placeholder={t('wefax.fl.bandAuto')} value={draft.band} onChange={(e) => set({ band: e.target.value })} />
            </Field>
          </Cells>
          <datalist id="wefax-fl-bands">
            {bands.map((b) => (
              <option key={b} value={b} />
            ))}
          </datalist>
          {error ? <Message kind="alert">{error}</Message> : null}
          <Actions>
            <button type="submit" className="glass glass--on">
              {editing ? t('wefax.fl.save') : t('wefax.fl.add')}
            </button>
            {editing ? (
              <button type="button" className="glass" onClick={cancel}>
                {t('wefax.fl.cancel')}
              </button>
            ) : null}
            <button type="button" className="glass" title={t('wefax.fl.resetTip')} onClick={reset}>
              {t('wefax.fl.reset')}
            </button>
          </Actions>
        </form>
        {state.hidden.length > 0 ? <Message kind="muted">{t('wefax.fl.hidden', { n: String(state.hidden.length) })}</Message> : null}
        {presets.length === 0 ? (
          <Message kind="muted">{t('wefax.fl.empty')}</Message>
        ) : (
          <div className="sx-tablewrap sx-tablewrap--scroll">
            <table className="sx-table sx-table--compact">
              <thead>
                <tr>
                  <th scope="col">{t('wefax.fl.call')}</th>
                  <th scope="col">{t('wefax.fl.place')}</th>
                  <th scope="col">{t('wefax.fl.khz')}</th>
                  <th scope="col">{t('wefax.fl.band')}</th>
                  <th scope="col">{t('wefax.fl.actions')}</th>
                </tr>
              </thead>
              <tbody>
                {groups.flatMap((g) =>
                  g.presets.map((p) => {
                    const mark = !isFactoryId(p.id) ? t('wefax.fl.mine') : state.edited[p.id] ? t('wefax.fl.changed') : '';
                    const what = `${p.call} ${formatKhz(p.khz)} kHz`;
                    return (
                      <tr key={p.id} aria-current={editing === p.id ? 'true' : undefined}>
                        <td>
                          {p.call}
                          {mark ? <span className="muted"> · {mark}</span> : null}
                        </td>
                        <td>{p.place}</td>
                        <td>{formatKhz(p.khz)}</td>
                        <td>{bandName(p)}</td>
                        <td>
                          <button type="button" className="glass" aria-label={`${t('wefax.fl.edit')} ${what}`} onClick={() => edit(p)}>
                            {t('wefax.fl.edit')}
                          </button>{' '}
                          <button type="button" className="glass" aria-label={`${t('wefax.fl.delete')} ${what}`} onClick={() => remove(p)}>
                            {t('wefax.fl.delete')}
                          </button>
                        </td>
                      </tr>
                    );
                  }),
                )}
              </tbody>
            </table>
          </div>
        )}
      </Group>
    </Page>
  );
}
