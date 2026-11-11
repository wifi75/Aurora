import { useState } from 'react';
import { addLogQso } from '../../api/client';
import { useT } from '../../i18n';
import { bandOf } from '../../lib/format';
import { replyErrorMessage } from '../../lib/replyError';
import { toast } from '../../lib/toast';

/** La scheda per mettere a log il QSO fatto in SSTV (come il riquadro Log dell'MMSSTV): nominativo, rapporti, nome, QTH, note. */
export function SstvLogCard({ dialHz }: { dialHz: number }) {
  const t = useT();
  const empty = { call: '', sent: '59', rcvd: '59', name: '', qth: '', note: '' };
  const [f, setF] = useState(empty);
  const [busy, setBusy] = useState(false);
  const set = (k: keyof typeof empty, v: string) => setF((p) => ({ ...p, [k]: v }));

  const save = async () => {
    const call = f.call.trim().toUpperCase();
    if (!call) {
      toast(t('sstv.log.needCall'), 'error');
      return;
    }
    const now = new Date();
    setBusy(true);
    try {
      await addLogQso({
        call,
        date: now.toISOString().slice(0, 10),
        time: now.toISOString().slice(11, 16),
        band: bandOf(dialHz) ?? '',
        freq_hz: dialHz,
        mode: 'SSTV',
        rst_sent: f.sent.trim(),
        rst_rcvd: f.rcvd.trim(),
        grid: '',
        name: f.name.trim(),
        qth: f.qth.trim(),
        comment: f.note.trim(),
      });
      toast(t('sstv.log.saved', { call }), 'ok');
      setF(empty);
    } catch (error) {
      toast(replyErrorMessage(error, t), 'error');
    } finally {
      setBusy(false);
    }
  };

  return (
    <section className="sstv-log" aria-label={t('sstv.log.title')}>
      <div className="sstv-log-title">{t('sstv.log.title')}</div>
      <label className="sstv-log-f sstv-log-call">
        <span>{t('sstv.log.call')}</span>
        <input value={f.call} onChange={(e) => set('call', e.target.value.toUpperCase())} autoCapitalize="characters" spellCheck={false} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.his')}</span>
        <input value={f.sent} onChange={(e) => set('sent', e.target.value)} inputMode="numeric" maxLength={3} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.my')}</span>
        <input value={f.rcvd} onChange={(e) => set('rcvd', e.target.value)} inputMode="numeric" maxLength={3} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.name')}</span>
        <input value={f.name} onChange={(e) => set('name', e.target.value)} />
      </label>
      <label className="sstv-log-f">
        <span>{t('sstv.log.qth')}</span>
        <input value={f.qth} onChange={(e) => set('qth', e.target.value)} />
      </label>
      <label className="sstv-log-f sstv-log-note">
        <span>{t('sstv.log.note')}</span>
        <input value={f.note} onChange={(e) => set('note', e.target.value)} />
      </label>
      <div className="sstv-log-btns">
        <button type="button" className="sstv-btn sstv-btn--primary" disabled={busy} onClick={() => void save()}>
          {t('sstv.log.save')}
        </button>
        <button type="button" className="sstv-btn" onClick={() => setF(empty)}>
          {t('sstv.log.clear')}
        </button>
      </div>
    </section>
  );
}
