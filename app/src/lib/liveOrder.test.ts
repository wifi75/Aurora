import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// La finestra di un pannello staccato chiama renderPanel durante il render: ogni `const` che il pannello legge deve stare prima del primo
// `return` anticipato di Live.tsx, altrimenti il browser dice «Cannot access ... before initialization» (successo con tagFilterOn).
describe('Live.tsx: ordine delle dichiarazioni', () => {
  const src = readFileSync('src/pages/Live.tsx', 'utf8');

  it("tagFilterOn e' dichiarato prima di renderPanel", () => {
    const decl = src.indexOf('const tagFilterOn');
    const render = src.indexOf('const renderPanel');
    expect(decl).toBeGreaterThan(-1);
    expect(render).toBeGreaterThan(-1);
    expect(decl).toBeLessThan(render);
  });

  // Se la vista cambia sulla stessa istanza (la finestra scende sotto i 768 px, un tablet ruota) il numero degli hook non deve cambiare:
  // React lancerebbe «Rendered fewer hooks than expected».
  it('nessun hook dopo il primo return anticipato del componente (soloPanel / vista compatta)', () => {
    const early = src.indexOf('if (soloPanel)');
    expect(early).toBeGreaterThan(-1);
    const next = src.indexOf('\nfunction ', early);
    const body = src.slice(early, next === -1 ? undefined : next);
    const hooks = body.match(/(^|[^.\w])use[A-Z]\w*\(/gm) ?? [];
    expect(hooks, `hook dopo il return anticipato: ${hooks.join(' ')}`).toEqual([]);
  });
});
