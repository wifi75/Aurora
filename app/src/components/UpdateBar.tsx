import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { ApiError, downloadUpdate, getUpdate, installUpdate, type UpdateStage } from '../api/client';
import { useT } from '../i18n';
import { browserStorage } from '../lib/theme';
import { useStation } from '../state/station';
import { InstallProgress } from './InstallProgress';

const KEY = 'aurora-update-dismissed';

/** Le fasi in cui il core sta lavorando: la barretta si rilegge spesso. */
const BUSY = new Set(['downloading', 'verifying', 'installing']);

function dismissedVersion(): string {
  try {
    return browserStorage()?.getItem(KEY) ?? '';
  } catch {
    return '';
  }
}

/**
 * Quando sul repository e' uscita una versione di Aurora piu' nuova di quella in uso, in testata compare una piccola icona (prima della lingua);
 * un clic apre le azioni: Installa ora, Note e download, Piu' tardi (nasconde l'avviso fino alla versione dopo).
 * Sul PC il core scarica la versione nuova in anticipo: al posto dell'icona c'e' allora una barretta arancio con l'avanzamento, che a fine
 * scaricamento diventa «Pronto · Installa e riavvia» (o, se non e' riuscito, «Download non riuscito · Riprova»).
 */
export function UpdateBar() {
  const t = useT();
  const update = useQuery({
    queryKey: ['update'],
    queryFn: getUpdate,
    retry: false,
    refetchInterval: (q) => {
      const d = q.state.data;
      const phase = d?.stage?.phase ?? 'idle';
      // scaricamento in corso: la barretta segue l'avanzamento; versione nuova appena trovata: lo scaricamento sta per partire
      if (BUSY.has(phase)) return 1_000;
      if (d?.available && d.can_install && phase === 'idle') return 5_000;
      return (d?.checked_at_ms ?? 0) === 0 ? 5_000 : 10 * 60_000;
    },
  });
  const [hidden, setHidden] = useState(dismissedVersion);
  const notice = useStation((s) => s.updateNotice);
  const [installing, setInstalling] = useState(false);
  const [failure, setFailure] = useState('');
  const [open, setOpen] = useState(false);
  const box = useRef<HTMLDivElement | null>(null);
  const install = async () => {
    setInstalling(true);
    setFailure('');
    void update.refetch();
    try {
      await installUpdate();
    } catch (error) {
      setInstalling(false);
      setFailure(t('upd.installFailed', { message: error instanceof ApiError ? (error.detail ?? error.code) : '—' }));
      void update.refetch();
    }
  };
  const retry = async () => {
    try {
      await downloadUpdate();
    } catch {
      // lo stato riletto dice comunque com'e' andata
    }
    void update.refetch();
  };
  // quando il core avvisa di una versione nuova i dati della pagina sono vecchi (senza «Installa ora»): si rileggono
  const refetch = update.refetch;
  useEffect(() => {
    if (notice) void refetch();
  }, [notice, refetch]);
  // l'evento del core arriva subito; il controllo della pagina dopo, con tutti i dettagli
  const release = update.data?.available ?? (notice ? { version: notice.version, tag: notice.tag, name: '', url: notice.url } : null);
  // il pannello si chiude con Esc o con un clic fuori
  useEffect(() => {
    if (!open) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    const onDown = (e: MouseEvent) => {
      if (box.current && !box.current.contains(e.target as Node)) setOpen(false);
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('mousedown', onDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('mousedown', onDown);
    };
  }, [open]);
  const stage = update.data?.stage;
  const phase = stage?.phase ?? 'idle';
  // mentre il core lavora la barretta si vede anche se l'avviso era stato rimandato; pronta o in errore segue «Piu' tardi»
  const busy = BUSY.has(phase) || installing;
  if (!release || (hidden === release.version && !busy)) return null;
  const banner = t('upd.banner', { version: release.version, current: update.data?.current ?? '…' });
  const strip = stage && (phase !== 'idle' || installing) ? stage : null;

  return (
    <div className="update-badge" ref={box}>
      {strip ? (
        <UpdateStrip stage={strip} installing={installing} open={open} onToggle={() => setOpen((v) => !v)} onInstall={() => void install()} onRetry={() => void retry()} />
      ) : (
        <button type="button" className="update-icon" aria-label={banner} aria-expanded={open} aria-haspopup="true" title={banner} onClick={() => setOpen((v) => !v)}>
          <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
            <path d="M12 3v12" />
            <path d="m7 11 5 5 5-5" />
            <path d="M5 20h14" />
          </svg>
          <span className="update-dot" aria-hidden="true" />
        </button>
      )}
      {open ? (
    <div className="update-pop" role="status">
      <strong>{banner}</strong>
      {release.name && release.name !== release.tag ? <span className="muted">{release.name}</span> : null}
      {update.data?.can_install ? (
        <button type="button" className="glass glass--on" disabled={installing} title={t('tip.ui.install')} onClick={() => void install()}>
          {t('upd.install')}
        </button>
      ) : null}
      <InstallProgress active={installing} />
      {failure ? <span role="alert">{failure}</span> : null}
      {release.url ? (
        <a className="glass glass--link" href={release.url} target="_blank" rel="noreferrer noopener">
          {t('upd.open')}
        </a>
      ) : null}
      <button
        type="button"
        className="glass"
        title={t('tip.ui.later')}
        onClick={() => {
          try {
            browserStorage()?.setItem(KEY, release.version);
          } catch {
            // senza archivio la barra torna alla prossima apertura
          }
          setHidden(release.version);
        }}
      >
        {t('upd.later')}
      </button>
    </div>
      ) : null}
    </div>
  );
}

interface StripProps {
  stage: UpdateStage;
  installing: boolean;
  open: boolean;
  onToggle: () => void;
  onInstall: () => void;
  onRetry: () => void;
}

/**
 * La barretta della testata: «⬇ 0.11.x», la barra e «45% · 6 s» durante lo scaricamento; «Pronto · Installa e riavvia» a fine scaricamento;
 * «Download non riuscito · Riprova» se non e' andato. Un clic sulla versione apre il pannello con le note e «Piu' tardi».
 */
function UpdateStrip({ stage, installing, open, onToggle, onInstall, onRetry }: StripProps) {
  const t = useT();
  const phase = installing && (stage.phase === 'ready' || stage.phase === 'idle') ? 'installing' : stage.phase;
  const pct = stage.percent ?? (stage.total > 0 ? Math.min(100, Math.floor((stage.done / stage.total) * 100)) : null);
  const tip = phase === 'error' && stage.error ? stage.error : t('upd.strip.tip', { version: stage.version });
  let info: string;
  if (phase === 'downloading') info = pct === null ? '…' : stage.eta_s !== null && stage.eta_s > 0 ? `${pct}% · ${stage.eta_s} s` : `${pct}%`;
  else if (phase === 'verifying') info = t('upd.strip.verifying');
  else if (phase === 'installing') info = t('upd.strip.installing');
  else if (phase === 'ready') info = t('upd.strip.ready');
  else info = t('upd.strip.failed');
  const showBar = phase === 'downloading' || phase === 'verifying';
  const moving = pct === null || phase === 'verifying';
  return (
    <span className={`update-strip update-strip--${phase}`} title={tip}>
      <button type="button" className="update-strip-ver" aria-expanded={open} aria-haspopup="true" onClick={onToggle}>
        {phase === 'ready' ? <span className="update-strip-dot" aria-hidden="true" /> : <span aria-hidden="true">⬇</span>} {phase === 'ready' ? `${t('upd.strip.ready')} ${stage.version}` : stage.version}
      </button>
      {showBar ? (
        <span className="update-strip-track" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={pct ?? undefined} aria-label={tip}>
          <span className={`update-strip-fill${moving ? ' update-strip-fill--wait' : ''}`} style={moving ? undefined : { width: `${pct}%` }} />
        </span>
      ) : null}
      {phase === 'ready' ? null : (
        <span className="update-strip-info" role={phase === 'error' ? 'alert' : 'status'}>
          {info}
        </span>
      )}
      {phase === 'ready' ? (
        <button type="button" className="update-strip-btn" onClick={onInstall}>
          {t('upd.strip.install')}
        </button>
      ) : null}
      {phase === 'error' ? (
        <button type="button" className="update-strip-btn" onClick={onRetry}>
          {t('upd.strip.retry')}
        </button>
      ) : null}
    </span>
  );
}
