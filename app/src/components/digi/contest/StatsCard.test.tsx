import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { TextContestScore } from '../../../api/contestTesto';
import { StatsCard } from './StatsCard';

const base: TextContestScore = {
  qso: 3,
  doppi: 0,
  punti: 10,
  molt: 3,
  punteggio: 30,
  qtc: 0,
  tipi: ['dom', 'dxcc'],
  bande: [{ band: '10m', qso: 3, doppi: 0, punti: 10, molt: 3, cw: 2, phone: 1, digital: 0 }],
  esclusi: 0,
  griglia: [],
  mancanti: [],
  recenti: [],
  rate10: 0,
  rate60: 0,
  media_ora: 0,
};

describe('StatsCard: moltiplicatori per modo', () => {
  it('senza il dato del core la riga manca', () => {
    render(<StatsCard score={base} />);
    expect(screen.queryByTestId('ctw-mult-modes')).toBeNull();
  });
  it('con ARRL 10 mostra i moltiplicatori di ogni modo', () => {
    render(<StatsCard score={{ ...base, molt_per_modo: { SSB: 1, CW: 2 } }} />);
    expect(screen.getByTestId('ctw-mult-modes').textContent).toContain('CW 2 · SSB 1');
  });
});
