import { useEffect, useRef, useState } from 'react';
import { useT } from '../../i18n';
import { formatKhz, type WefaxPreset } from '../../lib/wefaxStations';

interface Props {
  groups: { band: number; presets: WefaxPreset[] }[];
  /** La stazione su cui si e' (o l'ultima scelta): evidenziata e portata in vista all'apertura. */
  value: string;
  onSelect(p: WefaxPreset): void;
}

const label = (p: WefaxPreset) => `${formatKhz(p.khz)} kHz · ${p.call} ${p.place}`;
/** Nella casella chiusa: frequenza, il primo nominativo e il luogo (l'elenco aperto mostra tutti i nominativi). */
const shortLabel = label;

/**
 * L'elenco delle stazioni WEFAX per banda. Al posto del menu del sistema (che non si puo' far scorrere) all'apertura l'elenco si porta da solo sulla
 * stazione impostata per ultima, la evidenzia e la lascia al centro; frecce, Invio, Esc e un clic fuori funzionano come in un menu.
 */
export function FreqPicker({ groups, value, onSelect }: Props) {
  const t = useT();
  const [open, setOpen] = useState(false);
  const [active, setActive] = useState(value);
  const box = useRef<HTMLDivElement | null>(null);
  const list = useRef<HTMLDivElement | null>(null);
  const flat = groups.flatMap((g) => g.presets);
  const current = flat.find((p) => p.id === value);

  // all'apertura: la voce attiva e' quella impostata e si porta al centro dell'elenco
  useEffect(() => {
    if (!open) return;
    setActive(value);
    const el = list.current?.querySelector<HTMLElement>('[data-active="true"]');
    el?.scrollIntoView({ block: 'center' });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);
  // la voce attiva resta in vista mentre ci si muove con le frecce
  useEffect(() => {
    if (!open) return;
    list.current?.querySelector<HTMLElement>('[data-active="true"]')?.scrollIntoView({ block: 'nearest' });
  }, [active, open]);
  // un clic fuori chiude
  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (box.current && !box.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener('mousedown', onDown);
    return () => window.removeEventListener('mousedown', onDown);
  }, [open]);

  const choose = (p: WefaxPreset) => {
    setOpen(false);
    onSelect(p);
  };
  const move = (d: number) => {
    const i = Math.max(0, flat.findIndex((p) => p.id === active));
    const next = flat[Math.min(flat.length - 1, Math.max(0, i + d))];
    if (next) setActive(next.id);
  };
  const onKey = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      setOpen(false);
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (!open) setOpen(true);
      else move(e.key === 'ArrowDown' ? 1 : -1);
    } else if (e.key === 'Enter' && open) {
      e.preventDefault();
      const p = flat.find((x) => x.id === active);
      if (p) choose(p);
    }
  };

  return (
    <div className="wefax-picker" ref={box} onKeyDown={onKey}>
      <button
        type="button"
        className="sstv-modesel wefax-freq wefax-picker-btn"
        aria-haspopup="listbox"
        aria-expanded={open}
        aria-label={t('wefax.freq.label')}
        title={t('wefax.freq.tip')}
        onClick={() => setOpen((v) => !v)}
      >
        <span className="wefax-picker-text">{current ? shortLabel(current) : t('wefax.freq.choose')}</span>
        <span aria-hidden="true" className="wefax-picker-caret">▾</span>
      </button>
      {open ? (
        <div className="wefax-picker-list" role="listbox" aria-label={t('wefax.freq.label')} ref={list}>
          {groups.map((g) => (
            <div key={g.band} role="group" aria-label={t('wefax.freq.band', { mhz: String(g.band) })}>
              <div className="wefax-picker-band">{t('wefax.freq.band', { mhz: String(g.band) })}</div>
              {g.presets.map((p) => (
                <div
                  key={p.id}
                  role="option"
                  aria-selected={p.id === value}
                  data-active={p.id === active}
                  className={`wefax-picker-opt${p.id === value ? ' wefax-picker-opt--now' : ''}${p.id === active ? ' wefax-picker-opt--active' : ''}`}
                  onMouseEnter={() => setActive(p.id)}
                  onClick={() => choose(p)}
                >
                  {label(p)}
                </div>
              ))}
            </div>
          ))}
        </div>
      ) : null}
    </div>
  );
}
