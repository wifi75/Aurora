import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import { ApiError, getQmap, postQmapDecode, postQmapTune, putQmap, type QmapConfig, type QmapDecode, type QmapSnapshot } from '../api/client';
import { useT } from '../i18n';
import { formatKhz } from '../lib/format';
import '../styles/qmap.css';

const PERIODS = [15, 30, 60, 120, 300] as const;
const RATES = [48_000, 96_000, 192_000, 384_000] as const;
const LETTERS = ['A', 'B', 'C', 'D', 'E'] as const;

/** Frequenza mostrata: in kHz se il centro e' noto, altrimenti lo scostamento dal centro. */
function freqLabel(d: QmapDecode): string {
  if (d.rf_hz !== null) return formatKhz(d.rf_hz / 1000);
  const khz = d.offset_hz / 1000;
  return `${khz >= 0 ? '+' : '−'}${formatKhz(Math.abs(khz))}`;
}

/** Lo spettro della fetta con un segno su ogni messaggio (clic = QSO). */
function Spectrum({ snap, onPick }: { snap: QmapSnapshot; onPick: (d: QmapDecode) => void }) {
  const data = snap.spectrum_db;
  if (data.length < 2) return <div className="qmap-spectrum qmap-spectrum--empty" />;
  const lo = Math.min(...data);
  const hi = Math.max(...data);
  const range = Math.max(hi - lo, 1);
  const w = 1000;
  const h = 120;
  const points = data.map((v, i) => `${((i / (data.length - 1)) * w).toFixed(1)},${(h - 6 - ((v - lo) / range) * (h - 16)).toFixed(1)}`).join(' ');
  const span = snap.spectrum_high_hz - snap.spectrum_low_hz || 1;
  const x = (hz: number) => ((hz - snap.spectrum_low_hz) / span) * w;
  return (
    <svg className="qmap-spectrum" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" role="img">
      <line className="qmap-center" x1={x(0)} x2={x(0)} y1={0} y2={h} />
      <polyline className="qmap-trace" points={points} />
      {snap.decodes.map((d) => (
        <g key={`${d.text}-${d.period_s}`} className="qmap-mark" onClick={() => onPick(d)}>
          <title>{`${freqLabel(d)} · ${d.mode} · ${d.text}`}</title>
          <rect x={x(d.offset_hz) - 4} y={0} width={8} height={h} />
          <line x1={x(d.offset_hz)} x2={x(d.offset_hz)} y1={4} y2={h - 4} />
        </g>
      ))}
    </svg>
  );
}

export function QmapPage() {
  const t = useT();
  const client = useQueryClient();
  const query = useQuery({ queryKey: ['qmap'], queryFn: getQmap, retry: false, refetchInterval: 2000 });
  const snap = query.data;
  const [draft, setDraft] = useState<QmapConfig | null>(null);
  const [note, setNote] = useState('');
  useEffect(() => {
    if (snap && draft === null) setDraft(snap.config);
  }, [snap, draft]);

  const errorText = (e: unknown) => (e instanceof ApiError && e.code === 'local_only' ? t('qmap.localOnly') : e instanceof ApiError ? (e.detail ?? e.code) : String(e));
  const save = useMutation({
    mutationFn: (c: QmapConfig) => putQmap(c),
    onSuccess: (s) => {
      setNote('');
      client.setQueryData(['qmap'], s);
      setDraft(s.config);
    },
    onError: (e) => setNote(errorText(e)),
  });
  const decodeFile = useMutation({ mutationFn: postQmapDecode, onError: (e) => setNote(errorText(e)) });
  const tune = useMutation({
    mutationFn: (d: QmapDecode) => postQmapTune(d.rf_hz ?? 0, d.period_s, d.submode),
    onSuccess: (r) => setNote(t('qmap.tuned', { freq: formatKhz(r.dial_hz / 1000), mode: r.mode })),
    onError: (e) => setNote(t('qmap.tuneFailed', { code: e instanceof ApiError ? e.code : String(e) })),
  });
  const pick = (d: QmapDecode) => {
    if (d.rf_hz === null) setNote(t('qmap.noCenter'));
    else tune.mutate(d);
  };

  if (!snap || !draft) return <div className="qmap"><p className="muted">{query.isError ? errorText(query.error) : '…'}</p></div>;
  const edit = (patch: Partial<QmapConfig>) => setDraft({ ...draft, ...patch });
  const toggleSub = (s: number) => edit({ submodes: draft.submodes.includes(s) ? draft.submodes.filter((x) => x !== s) : [...draft.submodes, s].sort() });

  return (
    <div className="qmap">
      <header className="qmap-head">
        <div>
          <h1>{t('qmap.title')}</h1>
          <p className="muted">{t('qmap.needSdr')}</p>
        </div>
        <span className={`qmap-pill qmap-pill--${snap.state}`} title={snap.message || undefined}>{t(`qmap.state.${snap.state}`)}</span>
      </header>

      <section className="qmap-config">
        <label>
          <span>{t('qmap.source')}</span>
          <select value={draft.source} onChange={(e) => edit({ source: e.target.value as QmapConfig['source'] })}>
            <option value="off">{t('qmap.source.off')}</option>
            <option value="tci">{t('qmap.source.tci')}</option>
            <option value="file">{t('qmap.source.file')}</option>
          </select>
        </label>
        {draft.source === 'tci' ? (
          <>
            <label>
              <span>{t('qmap.tci')}</span>
              <input value={draft.tci_address} placeholder="127.0.0.1:40001" onChange={(e) => edit({ tci_address: e.target.value })} />
            </label>
            <label>
              <span>{t('qmap.rate')}</span>
              <select value={draft.sample_rate} onChange={(e) => edit({ sample_rate: Number(e.target.value) })}>
                {RATES.map((r) => (
                  <option key={r} value={r}>{`${r / 1000} kHz`}</option>
                ))}
              </select>
            </label>
          </>
        ) : null}
        {draft.source === 'file' ? (
          <label className="qmap-wide">
            <span>{t('qmap.file')}</span>
            <input value={draft.file_path} onChange={(e) => edit({ file_path: e.target.value })} />
          </label>
        ) : null}
        <label title={t('tip.qmap.center')}>
          <span>{t('qmap.center')}</span>
          <input inputMode="decimal" value={draft.center_hz ? String(draft.center_hz / 1000) : ''} onChange={(e) => edit({ center_hz: Math.max(0, Math.round(Number(e.target.value.replace(',', '.')) * 1000) || 0) })} />
        </label>
        <label>
          <span>{t('qmap.period')}</span>
          <select value={draft.period_s} onChange={(e) => edit({ period_s: Number(e.target.value) })}>
            {PERIODS.map((p) => (
              <option key={p} value={p}>{`${p} s`}</option>
            ))}
          </select>
        </label>
        <fieldset className="qmap-subs">
          <legend>{t('qmap.submodes')}</legend>
          {LETTERS.map((l, i) => (
            <label key={l}>
              <input type="checkbox" checked={draft.submodes.includes(i)} onChange={() => toggleSub(i)} />
              {l}
            </label>
          ))}
        </fieldset>
        <label className="qmap-check">
          <input type="checkbox" checked={draft.fading} onChange={(e) => edit({ fading: e.target.checked })} />
          {t('qmap.fading')}
        </label>
        <div className="qmap-actions">
          <button type="button" disabled={save.isPending} onClick={() => save.mutate(draft)}>{t('qmap.save')}</button>
          {snap.config.source === 'file' ? (
            <button type="button" disabled={decodeFile.isPending || snap.state === 'decoding'} onClick={() => decodeFile.mutate()}>{t('qmap.decodeFile')}</button>
          ) : null}
        </div>
      </section>

      {note || snap.message ? <p className="qmap-note">{note || snap.message}</p> : null}

      <section className="qmap-panel">
        <div className="qmap-panel-head">
          <h2>{t('qmap.spectrum')}</h2>
          <span className="muted">{t('qmap.candidates', { n: snap.candidates, d: snap.decodes.length })}</span>
        </div>
        <Spectrum snap={snap} onPick={pick} />
      </section>

      <section className="qmap-panel">
        {snap.decodes.length === 0 ? (
          <p className="muted">{t('qmap.empty')}</p>
        ) : (
          <table className="qmap-table">
            <thead>
              <tr>
                <th className="num">{t('qmap.col.freq')}</th>
                <th>{t('qmap.col.mode')}</th>
                <th className="num">dB</th>
                <th className="num">DT</th>
                <th>{t('qmap.col.msg')}</th>
                <th />
              </tr>
            </thead>
            <tbody>
              {snap.decodes.map((d) => (
                <tr key={`${d.text}-${d.period_s}`} onDoubleClick={() => pick(d)}>
                  <td className="num">{freqLabel(d)}</td>
                  <td>{d.mode}</td>
                  <td className="num">{d.snr_db}</td>
                  <td className="num">{d.dt_s.toFixed(1)}</td>
                  <td className="qmap-msg">{d.text}</td>
                  <td>
                    <button type="button" className="qmap-qso" disabled={tune.isPending} title={t('tip.qmap.qso', { mode: d.mode })} onClick={() => pick(d)}>
                      {t('qmap.qso')}
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </section>
    </div>
  );
}
