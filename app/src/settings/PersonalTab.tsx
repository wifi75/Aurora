import { useT } from '../i18n';
import { usePersonal } from '../lib/personal';
import { GeneralWindowTab } from './GeneralWindowTab';
import { Cells, Field, Group, Message, Page } from './layout';

/**
 * Stazione › Dati personali: nominativo, locatore, regione IARU e nominativi composti (voci del motore, con Applica), poi nome, cognome,
 * QTH ed email dell'operatore. Nome e QTH sono gli stessi di «Il mio nome» e «Il mio QTH» della pagina PSK·RTTY·CW (un posto solo).
 */
export function PersonalTab() {
  const t = useT();
  const [p, set] = usePersonal();
  return (
    <div className="sw-page">
      <GeneralWindowTab part="station" />
      <Page>
        <Group title={t('sw.p.operator')} span={3}>
          <Cells min={14}>
            <Field label={t('sw.p.name')} stack>
              <input type="text" value={p.myname} maxLength={30} autoComplete="given-name" onChange={(e) => set({ myname: e.target.value })} />
            </Field>
            <Field label={t('sw.p.surname')} stack>
              <input type="text" value={p.mysurname} maxLength={40} autoComplete="family-name" onChange={(e) => set({ mysurname: e.target.value })} />
            </Field>
            <Field label={t('sw.p.qth')} stack>
              <input type="text" value={p.myqth} maxLength={40} autoComplete="address-level2" onChange={(e) => set({ myqth: e.target.value })} />
            </Field>
            <Field label={t('sw.p.email')} stack>
              <input type="email" value={p.myemail} maxLength={80} autoComplete="email" onChange={(e) => set({ myemail: e.target.value.trim() })} />
            </Field>
          </Cells>
          <Message kind="muted">{t('sw.p.note')}</Message>
        </Group>
      </Page>
    </div>
  );
}
