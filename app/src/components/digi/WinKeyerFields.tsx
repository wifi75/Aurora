import { useEffect, useState, type ReactNode } from 'react';
import { ApiError, digiWinKeyerTest, getCatPorts, WINKEYER_DEFAULTS, type CatPort, type DigiSettings, type KeyerStatus, type WinKeyerSettings } from '../../api/client';
import { useT } from '../../i18n';

/** La versione del firmware come la stampa il WinKeyer (23 = WK2.3, 31 = WK3.1). */
export const keyerVersion = (v: number) => `${v >= 30 ? 'WK3' : 'WK2'} ${Math.floor(v / 10)}.${v % 10}`;

/** Collegato / non collegato, con porta e firmware. */
export function KeyerState({ keyer }: { keyer?: KeyerStatus | null }) {
  const t = useT();
  return (
    <p className={`sstv-hint digi-wk-state${keyer?.connected ? ' digi-wk-state--on' : ''}`}>
      {keyer?.connected ? t('digi.wk.connected', { port: keyer.port, v: keyerVersion(keyer.version) }) : t('digi.wk.disconnected')}
      {keyer?.connected && keyer.pot_wpm != null ? ` · ${t('digi.wk.potNow', { wpm: keyer.pot_wpm })}` : null}
    </p>
  );
}

/**
 * Il WinKeyer. Nel dialogo del modo (`full` spento): l'uscita della CW (audio o WinKeyer), lo stato e «Prova». Nelle Impostazioni
 * (Stazione › WinKeyer, `full`): anche porta, velocita' e potenziometro, peso, rapporto, PTT, paddle, sidetone, spaziature e compensazioni.
 */
export function WinKeyerFields({
  settings: s,
  keyer,
  onSave,
  full = false,
  section,
}: {
  settings: DigiSettings;
  keyer?: KeyerStatus | null;
  onSave(patch: Partial<DigiSettings>): void;
  full?: boolean;
  /** Nelle Impostazioni ogni parte e' un riquadro. */
  section?: (title: string, children: ReactNode) => ReactNode;
}) {
  const t = useT();
  const wk: WinKeyerSettings = { ...WINKEYER_DEFAULTS, ...(s.winkeyer ?? {}) };
  const on = s.cw_output === 'winkeyer';
  const viaTci = s.cw_output === 'tci';
  const [ports, setPorts] = useState<CatPort[]>([]);
  const [result, setResult] = useState<string | null>(null);
  const [testing, setTesting] = useState(false);
  useEffect(() => {
    if (!full) return;
    let alive = true;
    getCatPorts()
      .then((p) => alive && setPorts(p))
      .catch(() => undefined);
    return () => {
      alive = false;
    };
  }, [full]);
  const save = (patch: Partial<WinKeyerSettings>) => onSave({ winkeyer: { ...wk, ...patch } });
  const num = (v: string, min: number, max: number) => Math.min(max, Math.max(min, Math.round(Number(v) || 0)));
  const test = () => {
    setTesting(true);
    setResult(null);
    digiWinKeyerTest()
      .then((v) => setResult(t('digi.wk.testOk', { v: keyerVersion(v) })))
      .catch((e: unknown) => setResult(e instanceof ApiError ? (e.detail ?? e.code) : String(e)))
      .finally(() => setTesting(false));
  };
  const box = section ?? ((_title: string, children: ReactNode) => children);
  const portList = wk.port && !ports.some((p) => p.port === wk.port) ? [{ port: wk.port, name: '', virtual: false }, ...ports] : ports;
  const numberField = (key: keyof WinKeyerSettings, label: string, tip: string, min: number, max: number, step = 1) => (
    <label title={tip}>
      <span>{label}</span>
      <input type="number" min={min} max={max} step={step} value={wk[key] as number} onChange={(e) => save({ [key]: num(e.target.value, min, max) } as Partial<WinKeyerSettings>)} />
    </label>
  );
  const check = (key: keyof WinKeyerSettings, label: string, tip: string) => (
    <label className="sstv-check" title={tip}>
      <input type="checkbox" checked={wk[key] as boolean} onChange={(e) => save({ [key]: e.target.checked } as Partial<WinKeyerSettings>)} />
      {label}
    </label>
  );
  const output = (
    <>
      <label title={t('digi.wk.outputTip')}>
        <span>{t('digi.wk.output')}</span>
        <select value={on ? 'winkeyer' : viaTci ? 'tci' : 'audio'} onChange={(e) => onSave({ cw_output: e.target.value as 'audio' | 'winkeyer' | 'tci' })}>
          <option value="audio">{t('digi.wk.outAudio')}</option>
          <option value="winkeyer">WinKeyer</option>
          <option value="tci">{t('digi.wk.outTci')}</option>
        </select>
      </label>
      {viaTci ? <p className="sstv-hint">{t('digi.wk.tciHint')}</p> : null}
    </>
  );
  const testRow = (
    <div className="digi-wk-test">
      <button type="button" className="glass sstv-btn" disabled={testing || !wk.port} title={t('digi.wk.testTip')} onClick={test}>
        {t('digi.wk.test')}
      </button>
      {result ? <span className="sstv-hint">{result}</span> : null}
    </div>
  );
  if (!full) {
    return (
      <>
        {output}
        {on ? (
          <>
            <KeyerState keyer={keyer} />
            {testRow}
            <p className="sstv-hint">{t('digi.wk.whereHint')}</p>
          </>
        ) : null}
      </>
    );
  }
  return (
    <>
      {box(
        t('digi.wk.secLink'),
        <>
          {output}
          <label title={t('digi.wk.portTip')}>
            <span>{t('digi.wk.port')}</span>
            <select value={wk.port} onChange={(e) => save({ port: e.target.value })}>
              <option value="">—</option>
              {portList.map((p) => (
                <option key={p.port} value={p.port}>
                  {p.name ? `${p.port} · ${p.name}` : p.port}
                </option>
              ))}
            </select>
          </label>
          <KeyerState keyer={keyer} />
          {testRow}
        </>,
      )}
      {box(
        t('digi.wk.secSpeed'),
        <>
          <label>
            <span>{t('digi.cw.wpm', { wpm: Math.round(s.cw_wpm) })}</span>
            <input type="range" min={5} max={60} step={1} value={s.cw_wpm} disabled={wk.use_pot} onChange={(e) => onSave({ cw_wpm: Number(e.target.value) })} />
          </label>
          {check('use_pot', t('digi.wk.pot'), t('digi.wk.potTip'))}
          {numberField('pot_min_wpm', t('digi.wk.potMin'), t('digi.wk.potRangeTip'), 5, 60)}
          {numberField('pot_range_wpm', t('digi.wk.potSpan'), t('digi.wk.potRangeTip'), 5, 40)}
          {numberField('farnsworth_wpm', t('digi.wk.farnsworth'), t('digi.wk.farnsworthTip'), 0, 99)}
        </>,
      )}
      {box(
        t('digi.wk.secKeying'),
        <>
          <label title={t('digi.wk.weightTip')}>
            <span>{t('digi.wk.weight', { w: wk.weight })}</span>
            <input type="range" min={10} max={90} step={1} value={wk.weight} onChange={(e) => save({ weight: Number(e.target.value) })} />
          </label>
          <label title={t('digi.wk.ratioTip')}>
            <span>{t('digi.wk.ratio', { r: (wk.ratio * 3) / 50 })}</span>
            <input type="range" min={33} max={66} step={1} value={wk.ratio} onChange={(e) => save({ ratio: Number(e.target.value) })} />
          </label>
          {numberField('key_comp_ms', t('digi.wk.keyComp'), t('digi.wk.keyCompTip'), 0, 250)}
          {numberField('first_extension_ms', t('digi.wk.firstExt'), t('digi.wk.firstExtTip'), 0, 250)}
          {check('autospace', t('digi.wk.autospace'), t('digi.wk.autospaceTip'))}
          {check('contest_spacing', t('digi.wk.contestSpacing'), t('digi.wk.contestSpacingTip'))}
        </>,
      )}
      {box(
        t('digi.wk.secPtt'),
        <>
          {check('ptt', t('digi.wk.ptt'), t('digi.wk.pttTip'))}
          {numberField('lead_ms', t('digi.wk.lead'), t('digi.wk.leadTailTip'), 0, 2500, 10)}
          {numberField('tail_ms', t('digi.wk.tail'), t('digi.wk.leadTailTip'), 0, 2500, 10)}
          <label title={t('digi.wk.hangTip')}>
            <span>{t('digi.wk.hang')}</span>
            <select value={wk.hang_time} onChange={(e) => save({ hang_time: Number(e.target.value) })}>
              <option value={0}>1</option>
              <option value={1}>1,33</option>
              <option value={2}>1,67</option>
              <option value={3}>2</option>
            </select>
          </label>
        </>,
      )}
      {box(
        t('digi.wk.secPaddle'),
        <>
          <label title={t('digi.wk.paddleTip')}>
            <span>{t('digi.wk.paddle')}</span>
            <select value={wk.paddle} onChange={(e) => save({ paddle: e.target.value as WinKeyerSettings['paddle'] })}>
              <option value="iambic_b">Iambic B</option>
              <option value="iambic_a">Iambic A</option>
              <option value="ultimatic">Ultimatic</option>
              <option value="bug">{t('digi.wk.bug')}</option>
            </select>
          </label>
          {check('paddle_swap', t('digi.wk.swap'), t('digi.wk.swapTip'))}
          {check('paddle_breakin', t('digi.wk.breakin'), t('digi.wk.breakinTip'))}
          {check('sidetone', t('digi.wk.sidetone'), t('digi.wk.sidetoneTip'))}
          <label title={t('digi.wk.sidetoneTip')}>
            <span>{t('digi.wk.sidetoneHz')}</span>
            <select value={wk.sidetone_hz} disabled={!wk.sidetone} onChange={(e) => save({ sidetone_hz: Number(e.target.value) })}>
              {[4000, 2000, 1333, 1000, 800, 667, 571, 500, 444, 400].map((hz) => (
                <option key={hz} value={hz}>
                  {hz} Hz
                </option>
              ))}
            </select>
          </label>
        </>,
      )}
    </>
  );
}
