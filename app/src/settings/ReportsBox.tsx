import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useState } from 'react';
import {
  ApiError,
  dismissCrashReport,
  getReports,
  newReportsId,
  previewCrashReport,
  putReports,
  sendCrashReport,
  sendSupportBundle,
  type ReportsPreview,
  type ReportsState,
} from '../api/client';
import { useT } from '../i18n';

const KEY = ['reports'];

/** Cosa mostra «Vedi cosa mando»: i file del pacchetto o un rapporto automatico, esattamente come partono. */
type Shown = { kind: 'bundle'; preview: ReportsPreview } | { kind: 'json'; value: unknown };

/** La finestra con il contenuto esatto di quello che parte. */
function PreviewDialog({ shown, onClose }: { shown: Shown; onClose: () => void }) {
  const t = useT();
  useEffect(() => {
    const esc = (e: KeyboardEvent) => e.key === 'Escape' && onClose();
    document.addEventListener('keydown', esc);
    return () => document.removeEventListener('keydown', esc);
  }, [onClose]);
  return (
    <div className="dxd-backdrop" onClick={onClose}>
      <div className="dxd rep-dialog" role="dialog" aria-label={t('rep.previewTitle')} onClick={(e) => e.stopPropagation()}>
        <h3>{t('rep.previewTitle')}</h3>
        {shown.kind === 'bundle' ? (
          <>
            <p className="about-support-note">{t('rep.previewBytes', { n: shown.preview.files.length, kb: Math.ceil(shown.preview.bytes / 1024) })}</p>
            {shown.preview.files.map((f) => (
              <details key={f.name}>
                <summary>
                  {f.name} ({Math.ceil(f.bytes / 1024)} KB)
                </summary>
                <pre className="rep-pre">{f.text}</pre>
              </details>
            ))}
          </>
        ) : (
          <pre className="rep-pre">{JSON.stringify(shown.value, null, 2)}</pre>
        )}
        <div className="about-support-actions">
          <button type="button" className="glass" onClick={onClose}>
            {t('rep.close')}
          </button>
        </div>
      </div>
    </div>
  );
}

function failureText(t: ReturnType<typeof useT>, error: unknown): string {
  return t('rep.failed', { message: error instanceof ApiError ? error.code : '—' });
}

/** Le opzioni dei rapporti degli errori nella scheda Assistenza, con l'invio del pacchetto senza dati personali. */
export function ReportsBox() {
  const t = useT();
  const client = useQueryClient();
  const reports = useQuery({ queryKey: KEY, queryFn: getReports, retry: false });
  const [shown, setShown] = useState<Shown | null>(null);
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState('');
  const [failure, setFailure] = useState('');
  const data = reports.data;

  const apply = (next: ReportsState) => client.setQueryData(KEY, next);
  const change = (input: { ask_after_crash?: boolean; auto_errors?: boolean; count_active?: boolean }) => {
    setFailure('');
    putReports(input).then(apply, (e: unknown) => setFailure(failureText(t, e)));
  };
  const showBundle = () => {
    setFailure('');
    previewCrashReport().then((preview) => setShown({ kind: 'bundle', preview }), (e: unknown) => setFailure(failureText(t, e)));
  };
  const sendBundle = async () => {
    setBusy(true);
    setFailure('');
    setSent('');
    try {
      setSent((await sendSupportBundle()).code);
    } catch (e) {
      setFailure(failureText(t, e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="about-fact about-support">
      <h3>{t('rep.title')}</h3>
      <p>{t('rep.intro')}</p>
      {!data ? (
        <p className="about-support-note">{reports.isError ? t('rep.onlyPc') : '…'}</p>
      ) : (
        <>
          <label className="rep-check">
            <input type="checkbox" checked={data.ask_after_crash} onChange={(e) => change({ ask_after_crash: e.target.checked })} />
            <span>{t('rep.ask')}</span>
          </label>
          <label className="rep-check">
            <input type="checkbox" checked={data.auto_errors} onChange={(e) => change({ auto_errors: e.target.checked })} />
            <span>{t('rep.auto')}</span>
          </label>
          <label className="rep-check" title={t('rep.activeTip')}>
            <input type="checkbox" checked={data.count_active ?? true} onChange={(e) => change({ count_active: e.target.checked })} />
            <span>{t('rep.active')}</span>
          </label>
          <p className="about-support-note">{t('rep.activeTip')}</p>
          <p className="about-support-note">{t('rep.autoHint', { max: data.max_per_day })}</p>
          {data.auto_errors ? <p className="about-support-note">{t('rep.queue', { n: data.queue.length, sent: data.sent_today, max: data.max_per_day })}</p> : null}
          <div className="about-support-actions">
            <button type="button" className="glass" onClick={showBundle}>
              {t('rep.show')}
            </button>
            <button type="button" className="glass" onClick={() => setShown({ kind: 'json', value: data.queue.length ? data.queue : data.sample })}>
              {t('rep.showAuto')}
            </button>
          </div>
          <p className="about-support-note">
            {t('rep.id')}: <code>{data.install_id}</code>{' '}
            <button type="button" className="glass" title={t('tip.rep.newId')} onClick={() => newReportsId().then(apply, (e: unknown) => setFailure(failureText(t, e)))}>
              {t('rep.newId')}
            </button>
          </p>
          {data.configured ? (
            <div className="about-support-actions">
              <button type="button" className="glass about-support-main" disabled={busy} title={t('tip.rep.sendBundle')} onClick={() => void sendBundle()}>
                {busy ? t('rep.sending') : t('rep.sendBundle')}
              </button>
            </div>
          ) : (
            <p className="about-support-note">{t('rep.notConfigured')}</p>
          )}
          {sent ? <p className="about-support-ready">{t('rep.sent', { code: sent })}</p> : null}
        </>
      )}
      {failure ? (
        <p className="about-support-alert" role="alert">
          {failure}
        </p>
      ) : null}
      {shown ? <PreviewDialog shown={shown} onClose={() => setShown(null)} /> : null}
    </section>
  );
}

/** Al riavvio dopo un crash, se l'operatore lo ha chiesto: «Manda il rapporto», «Vedi cosa mando», «No». */
export function CrashPrompt() {
  const t = useT();
  const reports = useQuery({ queryKey: KEY, queryFn: getReports, retry: false, staleTime: Infinity });
  const [closed, setClosed] = useState(false);
  const [shown, setShown] = useState<Shown | null>(null);
  const [busy, setBusy] = useState(false);
  const [sent, setSent] = useState('');
  const [failure, setFailure] = useState('');
  if (closed || !reports.data?.crash_pending) return null;
  const configured = reports.data.configured;

  const send = async () => {
    setBusy(true);
    setFailure('');
    try {
      setSent((await sendCrashReport()).code);
    } catch (e) {
      setFailure(failureText(t, e));
    } finally {
      setBusy(false);
    }
  };
  const no = () => {
    void dismissCrashReport().catch(() => undefined);
    setClosed(true);
  };

  return (
    <div className="dxd-backdrop">
      <div className="dxd rep-dialog" role="alertdialog" aria-label={t('rep.crashTitle')}>
        <h3>{t('rep.crashTitle')}</h3>
        <p>{t('rep.crashText')}</p>
        {configured ? null : <p className="about-support-note">{t('rep.notConfigured')}</p>}
        {sent ? <p className="about-support-ready">{t('rep.sent', { code: sent })}</p> : null}
        {failure ? (
          <p className="about-support-alert" role="alert">
            {failure}
          </p>
        ) : null}
        <div className="about-support-actions">
          {sent ? (
            <button type="button" className="glass" onClick={() => setClosed(true)}>
              {t('rep.close')}
            </button>
          ) : (
            <>
              <button type="button" className="glass about-support-main" disabled={busy || !configured} onClick={() => void send()}>
                {busy ? t('rep.sending') : t('rep.crashSend')}
              </button>
              <button type="button" className="glass" onClick={() => previewCrashReport().then((preview) => setShown({ kind: 'bundle', preview }), (e: unknown) => setFailure(failureText(t, e)))}>
                {t('rep.show')}
              </button>
              <button type="button" className="glass" onClick={no}>
                {t('rep.no')}
              </button>
            </>
          )}
        </div>
      </div>
      {shown ? <PreviewDialog shown={shown} onClose={() => setShown(null)} /> : null}
    </div>
  );
}
