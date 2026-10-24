import { useEffect, useRef, useState, type FormEvent } from 'react';
import { ApiError, finishPairing } from '../api/client';
import { useT, type T } from '../i18n';
import { defaultDeviceName } from '../lib/deviceName';

interface Props {
  initialCode?: string;
  /** Aperta da un link con il codice (QR o pulsante): si associa da sola, senza un clic. */
  autoSubmit?: boolean;
  onPaired(): void;
}

const digitsOnly = (v: string) => v.replace(/\D/g, '').slice(0, 8);

function messageFor(error: unknown, t: T): string {
  if (error instanceof ApiError) {
    if (error.code === 'bad_code') return t('pair.badCode');
    if (error.code === 'expired') return t('pair.expired');
    if (error.code === 'bad_name') return t('pair.badName');
  }
  return t('app.noCore');
}

export function Pair({ initialCode = '', autoSubmit = false, onPaired }: Props) {
  const t = useT();
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
      setError(t('pair.short'));
      return;
    }
    if (name.trim() === '') {
      setError(t('pair.nameRequired'));
      return;
    }
    setError('');
    setBusy(true);
    try {
      await finishPairing(code, name.trim());
      onPaired();
    } catch (e) {
      if (e instanceof ApiError && e.status === 429) setLockLeft(e.retryAfterSecs ?? 60);
      else setError(messageFor(e, t));
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
      <h1>{t('pair.title')}</h1>
      <p className="muted">{t('pair.hint')}</p>
      <label>
        {t('pair.code')}
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
        {t('pair.name')}
        <input value={name} onChange={(e) => setName(e.target.value)} placeholder={t('pair.name')} maxLength={64} />
      </label>
      <button type="submit" className="primary">
        {t('pair.button')}
      </button>
      <p className="form-note" role="alert">
        {lockLeft > 0 ? t('pair.locked', { n: lockLeft }) : error}
      </p>
    </form>
  );
}
