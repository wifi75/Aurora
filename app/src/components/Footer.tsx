import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';
import { useT } from '../i18n';

/** Nome, versione e autore dal core: una sola riga, mai scritti a mano qui. */
export function Footer() {
  const t = useT();
  const { data, isError } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  return (
    <footer className="app-footer">
      <small>
        {data
          ? t('footer.credit', { name: data.name, version: data.version, author: data.author })
          : isError
            ? t('footer.coreDown')
            : t('footer.connecting')}
      </small>
    </footer>
  );
}
