import { useEffect, useRef, useState, type FormEvent } from 'react';
import { ApiError, finishPairing } from '../api/client';
import { defaultDeviceName } from '../lib/deviceName';

interface Props {
  initialCode?: string;
  /** Aperta da un link con il codice (QR o pulsante): si associa da sola, senza un clic. */
  autoSubmit?: boolean;
  onPaired(): void;
}

const digitsOnly = (v: string) => v.replace(/\D/g, '').slice(0, 8);

function messageFor(error: unknown): string {
  if (error instanceof ApiError) {
    if (error.code === 'bad_code') return 'Codice non valido. Controlla le cifre e riprova.';
    if (error.code === 'expired') return 'Il codice è scaduto. Generane uno nuovo sul PC.';
    if (error.code === 'bad_name') return 'Dai al dispositivo un nome tra 1 e 64 caratteri.';
  }
  return 'Non riesco a contattare Aurora. Controlla che sia acceso sul PC e che la rete sia raggiungibile.';
}

export function Pair({ initialCode = '', autoSubmit = false, onPaired }: Props) {
  const [code, setCode] = useState(digitsOnly(initialCode));
  const [name, setName] = useState(() => defaultDeviceName(navigator.userAgent));
  const [error, setError] = useState('');
  const [lockLeft, setLockLeft] = useState(0);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (lockLeft <= 0) return;
    const timer = setInterval(() => setLockLeft((n) => Math.max(0, n - 1)), 1000);
    return () => clearInterval(timer);
  }, [lockLeft > 0]); // eslint-disable-line react-hooks/exhaustive-deps

  const attempt = async () => {
    if (lockLeft > 0 || busy) return;
    if (code.length !== 8) {
      setError('Il codice ha 8 cifre: controlla quello mostrato sul PC.');
      return;
    }
    if (name.trim() === '') {
      setError('Dai un nome al dispositivo, per riconoscerlo nell\'elenco.');
      return;
    }
    setError('');
    setBusy(true);
    try {
      await finishPairing(code, name.trim());
      onPaired();
    } catch (e) {
      if (e instanceof ApiError && e.status === 429) setLockLeft(e.retryAfterSecs ?? 60);
      else setError(messageFor(e));
    } finally {
      setBusy(false);
    }
  };

  const submit = (event: FormEvent) => {
    event.preventDefault();
    void attempt();
  };

  // il codice e' monouso: un solo tentativo automatico, anche con il doppio montaggio di sviluppo
  const autoTried = useRef(false);
  useEffect(() => {
    if (autoSubmit && !autoTried.current && code.length === 8) {
      autoTried.current = true;
      void attempt();
    }
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  return (
    <form className="pair" onSubmit={submit} noValidate>
      <h1>Associa questo dispositivo</h1>
      <p className="muted">Sul PC apri Aurora e genera un codice: vale 2 minuti.</p>
      <label>
        Codice di associazione
        <input
          className="data"
          inputMode="numeric"
          autoComplete="one-time-code"
          value={code}
          onChange={(e) => setCode(digitsOnly(e.target.value))}
          placeholder="12345678"
        />
      </label>
      <label>
        Nome del dispositivo
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder="Nome del dispositivo" maxLength={64} />
      </label>
      <button type="submit" className="primary">
        Associa questo dispositivo
      </button>
      <p className="form-note" role="alert">
        {lockLeft > 0 ? `Troppi tentativi. Riprova tra ${lockLeft} secondi.` : error}
      </p>
    </form>
  );
}
