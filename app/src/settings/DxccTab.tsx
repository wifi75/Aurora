import { useEffect, useState } from 'react';
import { DxccGrid, DxccLabelStrip } from '../components/DxccCard';
import { useT, type Key } from '../i18n';
import { COUNTRY_LABELS, LABEL_IDS, LABEL_TEXT, type LabelId } from '../lib/dxccCard';
import {
  CELL_KINDS,
  DXCC_BANDS,
  DXCC_ROWS,
  LABEL_FIELDS,
  hasCustomColors,
  useDxccPrefs,
  type CellKindKey,
  type DxccRow,
  type LabelField,
} from '../lib/dxccPrefs';
import { sampleLabels, sampleMatrix, sampleStationCells } from '../lib/dxccSample';
import { Check, Help } from './common';
import { Cells, Group, Notes, Page } from './layout';

/** Token del tema (chiaro o scuro) che danno il colore di fabbrica di ogni voce. */
const CELL_TOKEN: Record<CellKindKey, string> = { worked: '--dxcc-worked', unconfirmed: '--dxcc-unconfirmed', none: '--dxcc-none' };

function labelToken(id: LabelId, field: LabelField): string {
  const country = COUNTRY_LABELS.includes(id);
  switch (field) {
    case 'onBg':
      return country ? '--rare' : '--accent';
    case 'onInk':
      return country ? '--rare-ink' : '--accent-ink';
    case 'offBg':
      return '--panel-raised';
    case 'offInk':
      return '--text-muted';
  }
}

/** Il valore (#rrggbb) di un token del tema attuale, per mostrarlo nel selettore. */
function tokenHex(token: string): string {
  const v = getComputedStyle(document.documentElement).getPropertyValue(token).trim();
  if (/^#[0-9a-f]{6}$/i.test(v)) return v.toLowerCase();
  const short = /^#([0-9a-f])([0-9a-f])([0-9a-f])$/i.exec(v);
  if (short) return `#${short[1]}${short[1]}${short[2]}${short[2]}${short[3]}${short[3]}`.toLowerCase();
  const rgb = /^rgba?\((\d+)[ ,]+(\d+)[ ,]+(\d+)/.exec(v);
  if (rgb) return `#${[rgb[1], rgb[2], rgb[3]].map((n) => Number(n).toString(16).padStart(2, '0')).join('')}`;
  return '#808080';
}

/** Selettore colore con il tasto per tornare al valore di fabbrica. */
function ColorField({ label, tip, value, token, onChange }: { label: string; tip: string; value: string | undefined; token: string; onChange(value: string | undefined): void }) {
  const t = useT();
  return (
    <label className="dxcc-set-color" title={tip}>
      <span>{label}</span>
      <input type="color" aria-label={tip} value={value ?? tokenHex(token)} onChange={(e) => onChange(e.target.value)} />
      <button type="button" className="glass" disabled={value === undefined} aria-label={`${label}: ${t('dxs.factory')}`} title={t('dxs.factory')} onClick={() => onChange(undefined)}>
        ↺
      </button>
    </label>
  );
}

/** Scheda Impostazioni «DXCC»: bande e modi, colori (per modo e delle sei etichette) e aspetto della scheda; si salvano in questo programma. */
export function DxccTab() {
  const t = useT();
  const prefs = useDxccPrefs((s) => s.prefs);
  const set = useDxccPrefs((s) => s.set);
  const resetColors = useDxccPrefs((s) => s.resetColors);
  const [, setTick] = useState(0);

  // al cambio di tema (chiaro/scuro) i colori di fabbrica mostrati nei selettori si aggiornano
  useEffect(() => {
    const observer = new MutationObserver(() => setTick((n) => n + 1));
    observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    return () => observer.disconnect();
  }, []);

  const toggleBand = (band: string, on: boolean) => {
    const next = DXCC_BANDS.filter((b) => (b === band ? on : prefs.bands.includes(b)));
    if (next.length > 0) set({ bands: next });
  };
  const toggleRow = (row: DxccRow, on: boolean) => {
    const next = { ...prefs.rows, [row]: on };
    if (DXCC_ROWS.some((r) => next[r])) set({ rows: next });
  };
  const setCell = (row: DxccRow, kind: CellKindKey, value: string | undefined) => {
    const cell = { ...prefs.cells[row] };
    if (value) cell[kind] = value;
    else delete cell[kind];
    set({ cells: { ...prefs.cells, [row]: cell } });
  };
  const setLabel = (id: LabelId, field: LabelField, value: string | undefined) => {
    const colors = { ...prefs.labels[id] };
    if (value) colors[field] = value;
    else delete colors[field];
    set({ labels: { ...prefs.labels, [id]: colors } });
  };

  const partName = (field: LabelField) => t(field.endsWith('Bg') ? 'dxs.bg' : 'dxs.ink');
  const stateName = (field: LabelField) => t(field.startsWith('on') ? 'dxs.on' : 'dxs.off');

  return (
    <Page>
      <Group title={t('dxs.guide.title')} span={3}>
        <ul className="dxs-guide">
          {(['what', 'labels', 'grid', 'bands', 'colors'] as const).map((k) => (
            <li key={k}>{t(`dxs.guide.${k}` as Key)}</li>
          ))}
        </ul>
      </Group>

      <Group title={t('dxs.preview')} span={2} rows={2}>
        <div className="dxcc-set-preview">
          <div className={`dxcc-card dxcc-card--panel${prefs.compact ? ' dxcc-card--compact' : ''}`}>
            <DxccLabelStrip labels={sampleLabels()} captions={[`${t('dxcc.cap.country')} - IU3ABC`, `20m - IU3ABC`, `FT8 - IU3ABC`]} />
            <DxccGrid matrix={sampleMatrix()} station={sampleStationCells()} />
          </div>
        </div>
      </Group>

      <Group title={t('dxs.look')}>
        <Cells min={13}>
          <Check label={t('dxs.captions')} hint={t('dxs.captionsTip')} checked={prefs.captions} onChange={(captions) => set({ captions })} />
          <Check label={t('dxs.total')} hint={t('dxs.totalTip')} checked={prefs.total} onChange={(total) => set({ total })} />
          <Check label={t('dxs.tryField')} hint={t('dxs.tryFieldTip')} checked={prefs.tryField} onChange={(tryField) => set({ tryField })} />
          <Check label={t('dxs.compact')} hint={t('dxs.compactTip')} checked={prefs.compact} onChange={(compact) => set({ compact })} />
        </Cells>
      </Group>

      <Group title={t('dxs.rows')}>
        <div title={t('dxs.rowsTip')}>
          <Cells min={7}>
            {DXCC_ROWS.map((r) => (
              <Check key={r} label={r} hint={t('dxs.rowsTip')} checked={prefs.rows[r]} onChange={(on) => toggleRow(r, on)} />
            ))}
          </Cells>
        </div>
      </Group>

      <Group title={t('dxs.bands')} span={3}>
        <div title={t('dxs.bandsTip')}>
          <Cells min={7}>
            {DXCC_BANDS.map((b) => (
              <Check key={b} label={b === 'U' ? 'U' : b} hint={t('dxs.bandsTip')} checked={prefs.bands.includes(b)} onChange={(on) => toggleBand(b, on)} />
            ))}
          </Cells>
        </div>
      </Group>

      <Group
        title={t('dxs.colors')}
        span={3}
        aside={
          <button type="button" className="glass" disabled={!hasCustomColors(prefs)} title={t('dxs.resetColorsTip')} onClick={resetColors}>
            {t('dxs.resetColors')}
          </button>
        }
      >
        <div className="dxcc-set-colors">
          {DXCC_ROWS.map((row) => (
            <fieldset key={row} className="dxcc-set-group">
              <legend>{t('dxs.cellsOf', { mode: row })}</legend>
              {CELL_KINDS.map((kind) => (
                <ColorField
                  key={kind}
                  label={t(`dxcc.cell.${kind}` as Key)}
                  tip={t('dxs.cellTip', { kind: t(`dxcc.cell.${kind}` as Key), mode: row })}
                  value={prefs.cells[row][kind]}
                  token={CELL_TOKEN[kind]}
                  onChange={(v) => setCell(row, kind, v)}
                />
              ))}
            </fieldset>
          ))}
          {LABEL_IDS.map((id) => (
            <fieldset key={id} className="dxcc-set-group">
              <legend>{t('dxs.labelsOf', { label: LABEL_TEXT[id] })}</legend>
              {LABEL_FIELDS.map((field) => (
                <ColorField
                  key={field}
                  label={`${partName(field)}, ${stateName(field)}`}
                  tip={t('dxs.labelTip', { part: partName(field), label: LABEL_TEXT[id], state: stateName(field) })}
                  value={prefs.labels[id][field]}
                  token={labelToken(id, field)}
                  onChange={(v) => setLabel(id, field, v)}
                />
              ))}
            </fieldset>
          ))}
        </div>
      </Group>

      <Notes span={3}>
        <Help>{t('dxs.intro')}</Help>
        <Help>{t('dxs.colorsHelp')}</Help>
      </Notes>
    </Page>
  );
}
