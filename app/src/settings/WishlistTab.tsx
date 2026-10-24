import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { getWishlist, putWishlist, testWishlist } from '../api/client';
import { useT, type Key } from '../i18n';
import { formatKhz } from '../lib/format';
import { parseLines, type Wishlist } from '../lib/wishlist';
import { Check, Help } from './common';
import { useAutosave } from './useAutosave';

interface Draft {
  enabled: boolean;
  calls: string;
  prefixes: string;
  countries: string;
  grids: string;
  bands: string;
  cooldown: string;
  fromDecodes: boolean;
  fromCluster: boolean;
  notifyUrl: string;
}

function toDraft(w: Wishlist): Draft {
  return {
    enabled: w.enabled,
    calls: w.calls.join('\n'),
    prefixes: w.prefixes.join('\n'),
    countries: w.countries.join('\n'),
    grids: w.grids.join('\n'),
    bands: w.bands.join(', '),
    cooldown: String(w.cooldown_min),
    fromDecodes: w.from_decodes,
    fromCluster: w.from_cluster,
    notifyUrl: w.notify_url,
  };
}

const CALL_LIKE = /^[A-Za-z0-9/]{1,15}$/;
const GRID4 = /^[A-Ra-r]{2}\d{2}$/;
const BAND = /^[A-Za-z0-9]{2,5}$/;

/** Il testo della scheda in una lista valida per il core, o `null` se qualche voce non va. */
export function draftToWishlist(d: Draft): Wishlist | null {
  const calls = parseLines(d.calls).map((s) => s.toUpperCase());
  const prefixes = parseLines(d.prefixes).map((s) => s.toUpperCase());
  const grids = parseLines(d.grids).map((s) => s.toUpperCase());
  const bands = parseLines(d.bands).map((s) => s.toLowerCase());
  const countries = d.countries.split(/\r?\n/).map((s) => s.trim()).filter((s) => s !== '');
  const cooldown = Number(d.cooldown);
  const url = d.notifyUrl.trim();
  if ([calls, prefixes, grids, bands, countries].some((l) => l.length > 200)) return null;
  if (!calls.every((c) => CALL_LIKE.test(c)) || !prefixes.every((p) => CALL_LIKE.test(p))) return null;
  if (!grids.every((g) => GRID4.test(g)) || !bands.every((b) => BAND.test(b))) return null;
  if (countries.some((c) => c.length > 60)) return null;
  if (!Number.isInteger(cooldown) || cooldown < 1 || cooldown > 240) return null;
  if (url !== '' && (url.length > 300 || !/^https?:\/\/\S+$/.test(url))) return null;
  return {
    enabled: d.enabled,
    calls,
    prefixes,
    countries,
    grids,
    bands,
    cooldown_min: cooldown,
    from_decodes: d.fromDecodes,
    from_cluster: d.fromCluster,
    notify_url: url,
  };
}

/** Scheda Desiderati: le stazioni volute e gli avvisi (pagina e telefono). */
export function WishlistTab() {
  const t = useT();
  const client = useQueryClient();
  const wishlist = useQuery({ queryKey: ['wishlist'], queryFn: getWishlist, retry: false });
  const [draft, setDraft] = useState<Draft | null>(null);
  const [note, setNote] = useState('');

  const shown: Draft | null = draft ?? (wishlist.data ? toDraft(wishlist.data.config) : null);
  const parsed = shown ? draftToWishlist(shown) : null;

  const save = useMutation({
    mutationFn: (list: Wishlist) => putWishlist(list),
    onSuccess: () => {
      setDraft(null);
      setNote(t('wish.saved'));
      void client.invalidateQueries({ queryKey: ['wishlist'] });
    },
    onError: () => setNote(t('wish.invalid')),
  });
  const test = useMutation({
    mutationFn: () => testWishlist(),
    onSuccess: (phone) => setNote(phone === 'sent' ? t('wish.testSent') : phone === 'failed' ? t('wish.testFailed') : t('wish.testNoUrl')),
    onError: () => setNote(t('wish.testFailed')),
  });

  useAutosave(
    parsed,
    draft !== null,
    (list) => save.mutate(list),
    (list) => void putWishlist(list).catch(() => undefined),
  );

  if (wishlist.isError) return <p className="notice">{t('set.loadError')}</p>;
  if (!wishlist.data || !shown) return <p className="muted">{t('set.loading')}</p>;
  const set = (change: Partial<Draft>) => setDraft({ ...shown, ...change });
  const area = (key: 'calls' | 'prefixes' | 'countries' | 'grids', label: Key, hint: Key) => (
    <label className="auto-field" title={t(hint)}>
      {t(label)}
      <textarea rows={4} value={shown[key]} spellCheck={false} onChange={(e) => set({ [key]: e.target.value } as Partial<Draft>)} />
    </label>
  );

  return (
    <>
      <Help>{t('wish.intro')}</Help>
      <Check label={t('wish.enabled')} hint={t('tip.wish.enabled')} checked={shown.enabled} onChange={(v) => set({ enabled: v })} />
      <div className="wish-grid">
        {area('calls', 'wish.calls', 'tip.wish.calls')}
        {area('prefixes', 'wish.prefixes', 'tip.wish.prefixes')}
        {area('countries', 'wish.countries', 'tip.wish.countries')}
        {area('grids', 'wish.grids', 'tip.wish.grids')}
      </div>
      <label className="set-check" title={t('tip.wish.bands')}>
        <span>{t('wish.bands')}</span>
        <input type="text" className="jx-text" value={shown.bands} placeholder="20m, 15m" onChange={(e) => set({ bands: e.target.value })} />
      </label>
      <label className="set-check" title={t('tip.wish.cooldown')}>
        <span>{t('wish.cooldown')}</span>
        <input type="number" className="set-number" min={1} max={240} value={shown.cooldown} onChange={(e) => set({ cooldown: e.target.value })} />
      </label>
      <Check label={t('wish.fromDecodes')} checked={shown.fromDecodes} onChange={(v) => set({ fromDecodes: v })} />
      <Check label={t('wish.fromCluster')} checked={shown.fromCluster} onChange={(v) => set({ fromCluster: v })} />

      <h3 className="set-sub">{t('wish.notifyUrl')}</h3>
      <label className="auto-field" title={t('tip.wish.notifyUrl')}>
        <input type="url" className="jx-text" value={shown.notifyUrl} placeholder="https://ntfy.sh/…" spellCheck={false} onChange={(e) => set({ notifyUrl: e.target.value })} />
      </label>
      <Help>{t('tip.wish.notifyUrl')}</Help>
      <div className="cmd-chips">
        <button type="button" className="glass" disabled={test.isPending || draft !== null} onClick={() => test.mutate()}>
          {t('wish.test')}
        </button>
      </div>
      {!parsed ? <p className="notice">{t('wish.invalid')}</p> : <Help>{t('wish.autosave')}</Help>}
      {note ? <p className="live-note" role="status">{note}</p> : null}

      <h3 className="set-sub">{t('wish.recent')}</h3>
      {wishlist.data.recent.length === 0 ? (
        <p className="muted">{t('wish.none')}</p>
      ) : (
        <ul className="wish-recent">
          {wishlist.data.recent.slice(0, 12).map((a) => (
            <li key={`${a.time_ms}-${a.call}`}>
              <strong className="data">{a.call}</strong> <span className="muted">{t(`wish.reason.${a.reason}` as Key)} · {t(`wish.source.${a.source}` as Key)}</span>
              {a.band ? ` · ${a.band}` : ''}
              {a.freq_hz ? ` · ${formatKhz(a.freq_hz / 1000)} kHz` : ''}
              <span className="muted"> · {new Date(a.time_ms).toLocaleTimeString()}</span>
            </li>
          ))}
        </ul>
      )}
    </>
  );
}

