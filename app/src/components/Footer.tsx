import { useQuery } from '@tanstack/react-query';
import { getHealth } from '../api/client';

/** Nome, versione e autore dal core: una sola riga, mai scritti a mano qui. */
export function Footer() {
  const { data, isError } = useQuery({ queryKey: ['health'], queryFn: getHealth, staleTime: 60_000 });
  return (
    <footer className="app-footer">
      <small>
        {data
          ? `${data.name} v${data.version} — Progettazione e sviluppo: ${data.author}`
          : isError
            ? 'Core non raggiungibile. Controlla che Aurora sia acceso sul PC.'
            : 'Connessione al core…'}
      </small>
    </footer>
  );
}
