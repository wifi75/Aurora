import { useQuery, useQueryClient } from '@tanstack/react-query';
import { checkSession } from './api/client';
import { Footer } from './components/Footer';
import { useRoute } from './lib/route';
import { Live } from './pages/Live';
import { Pair } from './pages/Pair';

export function App() {
  const route = useRoute();
  const queryClient = useQueryClient();
  const session = useQuery({ queryKey: ['session'], queryFn: checkSession, retry: false });

  const onPaired = () => {
    location.hash = '#/';
    void queryClient.invalidateQueries({ queryKey: ['session'] });
  };

  let content;
  if (route.name === 'pair') {
    content = <Pair initialCode={route.code} onPaired={onPaired} />;
  } else if (session.isPending) {
    content = <p className="muted">Verifica del dispositivo…</p>;
  } else if (session.isError) {
    content = (
      <div className="notice">
        <p>Non riesco a contattare Aurora. Controlla che sia acceso sul PC e che la rete sia raggiungibile.</p>
        <button type="button" onClick={() => session.refetch()}>
          Riprova
        </button>
      </div>
    );
  } else if (!session.data) {
    content = <Pair onPaired={onPaired} />;
  } else {
    content = <Live />;
  }

  return (
    <div className="app-shell">
      <header className="app-header">
        <span className="app-brand">AURORA</span>
      </header>
      <main className="app-main">{content}</main>
      <Footer />
    </div>
  );
}
