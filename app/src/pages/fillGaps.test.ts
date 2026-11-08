import { describe, expect, it } from 'vitest';
import { defaultBoard } from '../lib/boardLayout';
import { fillGaps } from './Live';

describe('fillGaps', () => {
  it('nascosti Log e DX Cluster, le finestre vicine riempiono il posto', () => {
    const all = defaultBoard() as Record<string, { x: number; y: number; w: number; h: number }>;
    const { log, cluster, ...rest } = all;
    const out = fillGaps(rest, [log!, cluster!]);
    // Attivita' di banda prende il posto del Log
    expect(out.all).toEqual({ x: 0, y: 0, w: 6, h: 19.25 });
    // Bande aperte e Comandi si allargano a sinistra al posto del DX Cluster
    expect(out.dxmap).toEqual({ x: 6, y: 0, w: 6, h: 9.5 });
    expect(out.cmds).toEqual({ x: 6, y: 9.5, w: 6, h: 4 });
    // le altre restano dove sono
    expect(out.tx).toEqual(all.tx);
    expect(out.cq).toEqual(all.cq);
  });

  it('un buco che nessuno copre esattamente resta vuoto', () => {
    const out = fillGaps({ a: { x: 0, y: 0, w: 2, h: 3 } }, [{ x: 2, y: 0, w: 2, h: 5 }]);
    expect(out.a).toEqual({ x: 0, y: 0, w: 2, h: 3 });
  });
});
