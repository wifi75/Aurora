import { useT } from '../i18n';
import { attach, openGroupWindow, readGroups, type ItemId } from '../lib/detached';
import '../styles/solo.css';

/** Al posto di una pagina che sta in una finestra a parte: lo dice, e permette di portarla davanti o di rimetterla qui. */
export function DetachedNote({ item }: { item: ItemId }) {
  const t = useT();
  const show = () => {
    const g = readGroups().find((x) => x.items.includes(item));
    if (g) openGroupWindow(g.id);
  };
  return (
    <div className="detached-note">
      <span>{t('win.elsewhere')}</span>
      <button type="button" className="chip glass" onClick={show}>
        {t('win.show')}
      </button>
      <button type="button" className="chip glass" onClick={() => attach(item)}>
        {t('win.attach')}
      </button>
    </div>
  );
}
