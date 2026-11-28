import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query';
import { getAutomation, getOnline, putAutomation, putOnline, type OnlineSettings } from '../api/client';
import type { AutomationRules } from '../api/automation';
import { useT, type Key } from '../i18n';
import { selectSettingsTab } from '../lib/settingsWindow';
import { SPECIAL_TAGS, type SpecialTag } from '../lib/specialCq';
import { anySpecialSound, useSpecialCq, type SpecialCqPrefs } from '../lib/specialCqPrefs';
import { useSoundAssign } from '../lib/soundAssign';
import { AUTOMATION_DRAFT } from './AutomationTab';
import { Check } from './common';
import { Actions, Cells, Group, Message, Page } from './layout';
import { useSharedDraft } from './session';

/**
 * Pagina «CQ speciali» (Modi › FT8 · FT4): riconosce «CQ POTA», «CQ SOTA», «CQ IOTA», «CQ WWFF» e «CQ QRP» (richiesta #115).
 * In cima l'interruttore generale, poi la tabella per tipo (etichetta, avviso sonoro, risposta automatica) e un riquadro breve per ogni funzione.
 * Tutto spento di fabbrica. Etichette e avvisi valgono per questo dispositivo e si applicano subito; la risposta automatica e' una regola
 * dell'Automazione (`reply_to_special`) e si salva con Applica, come le altre regole.
 */
export function SpecialCqTab() {
  const t = useT();
  const prefs = useSpecialCq((s) => s.prefs);
  const save = useSpecialCq((s) => s.set);
  const shared = useSharedDraft();
  // la ricerca del parco la fa il core: l'interruttore sta con gli altri servizi online (spento di fabbrica)
  const client = useQueryClient();
  const online = useQuery({ queryKey: ['online'], queryFn: getOnline, retry: false });
  const automation = useQuery({ queryKey: ['automation'], queryFn: getAutomation, retry: false });
  const putPota = useMutation({
    mutationFn: (settings: OnlineSettings) => putOnline(settings),
    onSuccess: () => {
      void client.invalidateQueries({ queryKey: ['online'] });
      void client.invalidateQueries({ queryKey: ['online-pota'] });
    },
  });
  const setPota = (pota: boolean) => {
    if (online.data) putPota.mutate({ ...online.data, pota });
  };

  // le regole dell'Automazione: la bozza comune della finestra (Applica/Salva), o quelle in vigore
  const savedRules = automation.data?.rules;
  const rules = (shared?.drafts[AUTOMATION_DRAFT] as AutomationRules | undefined) ?? savedRules;
  const replyTo: readonly string[] = rules?.reply_to_special ?? [];
  const setReply = (tags: string[]) => {
    if (!rules || !shared) return;
    const next = { ...rules, reply_to_special: SPECIAL_TAGS.filter((x) => tags.includes(x)) };
    shared.setDraftOf(AUTOMATION_DRAFT, JSON.stringify(next) === JSON.stringify(savedRules) ? undefined : next);
  };

  const stopReplies = useMutation({
    mutationFn: (next: AutomationRules) => putAutomation({ rules: next }),
    onSuccess: (state) => client.setQueryData(['automation'], state),
  });

  const change = (next: SpecialCqPrefs) => {
    save(next);
    if (!next.enabled) {
      // spegnendo l'interruttore generale si ferma anche la lettura degli spot POTA e si tolgono le risposte automatiche
      if (online.data?.pota === true) setPota(false);
      if (replyTo.length > 0) setReply([]);
      // le regole in vigore si tolgono subito, senza aspettare Applica: spento l'interruttore, l'Automazione non deve piu' rispondere
      if (savedRules && savedRules.reply_to_special.length > 0) stopReplies.mutate({ ...savedRules, reply_to_special: [] });
    }
    // un avviso acceso deve suonare: se l'evento non ha ancora un suono, gli si da' quello di fabbrica (il telegrafo «CQ»)
    if (anySpecialSound(next)) useSoundAssign.getState().ensure(['special_cq']);
  };
  const flag = (kind: 'show' | 'sound', tag: SpecialTag, value: boolean) => change({ ...prefs, [kind]: { ...prefs[kind], [tag]: value } });
  const toggleReply = (tag: SpecialTag, on: boolean) => setReply(on ? [...replyTo, tag] : replyTo.filter((x) => x !== tag));

  const off = !prefs.enabled;
  const autoReady = !!rules && !!shared;
  const columns: ReadonlyArray<{ key: 'show' | 'sound' | 'auto'; label: Key; tip: Key }> = [
    { key: 'show', label: 'scq.colShow', tip: 'tip.scq.show' },
    { key: 'sound', label: 'scq.colSound', tip: 'tip.scq.sound' },
    { key: 'auto', label: 'scq.colAuto', tip: 'tip.scq.auto' },
  ];

  return (
    <Page className="scq-page">
      <Group title={t('scq.title')} span={2}>
        <Message kind="muted">{t('scq.lead')}</Message>
        <Cells min={16}>
          <Check label={t('scq.enabled')} hint={t('tip.scq.main')} checked={prefs.enabled} onChange={(enabled) => change({ ...prefs, enabled })} />
        </Cells>
      </Group>

      <Group title={t('scq.grpTable')} span={2}>
        <Message kind="muted">{off ? t('scq.tableOff') : t('scq.tableLead')}</Message>
        <div className="sx-tablewrap">
          <table className="sx-table sx-table--compact scq-table">
            <thead>
              <tr>
                <th scope="col">{t('scq.colTag')}</th>
                {columns.map((c) => (
                  <th key={c.key} scope="col" title={t(c.tip)}>
                    {t(c.label)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {SPECIAL_TAGS.map((tag) => (
                <tr key={tag}>
                  <th scope="row" title={t(`scq.desc.${tag}` as Key)}>
                    <strong>{tag}</strong> <span className="muted">{t(`scq.desc.${tag}` as Key)}</span>
                  </th>
                  {columns.map((c) => {
                    const checked = c.key === 'auto' ? replyTo.includes(tag) : prefs[c.key][tag];
                    const disabled = off || (c.key === 'auto' && !autoReady);
                    return (
                      <td key={c.key} title={t(c.tip)}>
                        <span className="sx-cellbox">
                          <input
                            type="checkbox"
                            aria-label={`${tag} · ${t(c.label)}`}
                            checked={!off && checked}
                            disabled={disabled}
                            onChange={(e) => (c.key === 'auto' ? toggleReply(tag, e.target.checked) : flag(c.key, tag, e.target.checked))}
                          />
                        </span>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </Group>

      <Group title={t('scq.grpShow2')}>
        <Message kind="muted">{t('scq.showLead')}</Message>
      </Group>

      <Group title={t('scq.grpSounds')}>
        <Message kind="muted">{t('scq.soundLead')}</Message>
        <Actions>
          <button type="button" className="sw-linkbtn" title={t('tip.scq.openAlerts')} onClick={() => selectSettingsTab('alerts')}>
            {t('scq.openAlerts')}
          </button>
        </Actions>
      </Group>

      <Group title={t('scq.grpPota')}>
        <Message kind="muted">{t('scq.potaLead')}</Message>
        <Cells min={16}>
          <Check label={t('scq.pota')} hint={t('tip.scq.pota')} checked={prefs.enabled && online.data?.pota === true} onChange={(v) => setPota(v && prefs.enabled)} />
        </Cells>
      </Group>

      <Group title={t('scq.grpAuto')}>
        <Message kind="muted">{t('scq.autoLead')}</Message>
        {automation.isError ? <Message kind="alert">{t('auto.unavailable')}</Message> : null}
        <Actions>
          <button type="button" className="sw-linkbtn" title={t('tip.scq.openAuto')} onClick={() => selectSettingsTab('automation')}>
            {t('scq.openAuto')}
          </button>
        </Actions>
      </Group>

      <Group title={t('scq.grpSpot')} aside={<span className="pill">{t('scq.soon')}</span>}>
        <Message kind="muted">{t('scq.spotLead')}</Message>
      </Group>
    </Page>
  );
}
