import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { allowFirewall, ApiError, getNetworkCheck } from '../api/client';
import { useT } from '../i18n';
import { coreText } from '../lib/coreText';

/**
 * Durante l'associazione del telefono: cosa sa il PC della sua rete (Wi-Fi, rete «Pubblica», firewall, indirizzo scelto per il QR)
 * e i consigli per il telefono, cosi' chi non riesce a collegarsi sa subito perche'.
 */
export function NetworkChecklist({ waiting }: { waiting: boolean }) {
  const t = useT();
  const qc = useQueryClient();
  const check = useQuery({ queryKey: ['lan-check'], queryFn: getNetworkCheck, retry: false, refetchInterval: waiting ? 15_000 : false });
  const allow = useMutation({ mutationFn: allowFirewall, onSettled: () => void qc.invalidateQueries({ queryKey: ['lan-check'] }) });
  const c = check.data;
  if (!c) return null;
  // con la regola di Aurora valida su ogni rete, anche una rete «Pubblica» lascia entrare il telefono
  const pub = c.category === 'Public' && c.firewall_allowed !== true;
  const blocked = c.firewall_allowed === false;
  const first = c.addresses[0];

  return (
    <div className="net-check" aria-live="polite">
      {waiting ? <p className="net-check__wait">{t('net.waiting')}</p> : null}
      <ul>
        <li className={c.ssid ? 'is-ok' : 'is-info'}>
          <span aria-hidden="true">{c.ssid ? '✓' : 'i'}</span>
          <span>{c.ssid ? t('net.ssid', { ssid: c.ssid }) : t('net.cable')}</span>
        </li>
        {pub || blocked ? (
          <li className="is-ko">
            <span aria-hidden="true">✕</span>
            <span>{pub ? t('net.public') : t('net.blocked')}</span>
            <button type="button" className="glass glass--on" disabled={allow.isPending} onClick={() => allow.mutate()}>
              {allow.isPending ? t('net.allowing') : t('net.allow')}
            </button>
          </li>
        ) : c.firewall_allowed ? (
          <li className="is-ok">
            <span aria-hidden="true">✓</span>
            <span>{t('net.allowed')}</span>
          </li>
        ) : null}
        {first ? (
          <li className="is-ok">
            <span aria-hidden="true">✓</span>
            <span>{t('net.address', { ip: first[0], name: first[1] })}</span>
          </li>
        ) : (
          <li className="is-ko">
            <span aria-hidden="true">✕</span>
            <span>{t('net.noAddress')}</span>
          </li>
        )}
      </ul>
      {allow.isError ? <p className="net-check__err">{allow.error instanceof ApiError && allow.error.detail ? coreText(allow.error.detail, t) : t('net.allowFailed')}</p> : null}
      <p className="net-check__tip">{t('net.tips')}</p>
    </div>
  );
}
