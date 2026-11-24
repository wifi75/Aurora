import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// La finestra di un pannello staccato chiama renderPanel durante il render: ogni `const` che il pannello legge deve stare prima del primo
// `return` anticipato di Live.tsx, altrimenti il browser dice «Cannot access ... before initialization» (successo con tagFilterOn).
describe('Live.tsx: ordine delle dichiarazioni', () => {
  const src = readFileSync('src/pages/Live.tsx', 'utf8');
  it('tagFilterOn e\' dichiarato prima di renderPanel', () => {
    const decl = src.indexOf('const tagFilterOn');
    const render = src.indexOf('const renderPanel');
    expect(decl).toBeGreaterThan(-1);
    expect(render).toBeGreaterThan(-1);
    expect(decl).toBeLessThan(render);
  });
});
