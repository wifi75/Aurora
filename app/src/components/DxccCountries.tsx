import { useState } from 'react';
import { useT, type Key } from '../i18n';
import { bandHeader, cellKind, filterCountries, type DxccMatrix } from '../lib/dxccCard';

/** I paesi lavorati con una casella per banda: per modo (FT8, FT4, DIG) o per tutti i modi. */
export function DxccCountries({ matrix }: { matrix: DxccMatrix }) {
  const t = useT();
  const [row, setRow] = useState('ALL');
  const [text, setText] = useState('');
  const shown = filterCountries(matrix.countries ?? [], text, row);
  return (
    <div className="dxcc-countries">
      <div className="dxcc-countries-bar">
        <input type="search" value={text} placeholder={t('dxcc.countries.filter')} aria-label={t('dxcc.countries.filter')} onChange={(e) => setText(e.target.value)} />
        <label className="dxcc-countries-mode">
          {t('dxcc.countries.mode')}
          <select value={row} onChange={(e) => setRow(e.target.value)}>
            {['ALL', ...matrix.rows].map((r) => (
              <option key={r} value={r}>
                {t(`dxcc.row.${r}` as Key)}
              </option>
            ))}
          </select>
        </label>
        <span className="muted">{t('dxcc.countries.shown', { n: shown.length })}</span>
      </div>
      {shown.length === 0 ? (
        <p className="muted">{t('dxcc.countries.empty')}</p>
      ) : (
        <div className="dxcc-countries-wrap">
          <table className="dxcc-grid dxcc-grid--countries">
            <thead>
              <tr>
                <th scope="col" className="dxcc-name">
                  {t('dxcc.countries.country')}
                </th>
                {matrix.bands.map((b) => (
                  <th key={b} scope="col" className="dxcc-band" title={b === 'U' ? t('dxcc.bandU') : b}>
                    {bandHeader(b)}
                  </th>
                ))}
              </tr>
            </thead>
            <tbody>
              {shown.map((c) => (
                <tr key={c.id}>
                  <th scope="row" className="dxcc-name">
                    {c.name} <span className="muted data">{c.prefix}</span>
                  </th>
                  {matrix.bands.map((b) => {
                    const kind = cellKind(c.cells[row]?.[b]);
                    return (
                      <td key={b} className="dxcc-cell" data-kind={kind} title={`${c.name} ${b}: ${t(`dxcc.cell.${kind}` as Key)}`}>
                        <span className="visually-hidden">{t(`dxcc.cell.${kind}` as Key)}</span>
                      </td>
                    );
                  })}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
