import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';

// i fogli di stile si leggono dal disco (le prove girano dalla cartella `app`): nelle prove gli `import` dei CSS sono vuoti
const layout = readFileSync('src/styles/settings-layout.css', 'utf8');
const solo = readFileSync('src/styles/solo.css', 'utf8');

/** Le dichiarazioni di una regola, cercata per il suo selettore esatto (l'ultima che lo usa vince, come nel foglio). */
function rule(css: string, selector: string): string {
  const blocks = [...css.matchAll(/([^{}]+)\{([^{}]*)\}/g)].filter((m) => (m[1] ?? '').replace(/\/\*[\s\S]*?\*\//g, '').split(',').some((s) => s.trim() === selector));
  return blocks.map((m) => m[2] ?? '').join(';');
}

describe('Radio (CAT e PTT) su una finestra stretta', () => {
  it('the link grid wraps by the width of its box: three columns at most, then two, then one', () => {
    const grid = rule(layout, '.sw-panel .sx-cells.cat-link');
    // `auto-fill` con una larghezza minima: quante colonne ci stanno lo decide il riquadro, non lo schermo (a 1024 px tre colonne fisse non ci stavano)
    expect(grid).toMatch(/grid-template-columns:\s*repeat\(auto-fill,\s*minmax\(min\(100%,\s*max\(15rem,\s*calc\(\(100% - 46px\) \/ 3\)\)\),\s*1fr\)\)/);
  });

  it('every field stays inside its cell: no minimum width larger than the cell', () => {
    for (const selector of ['.sw-panel .sx-cells.cat-link > .sx-field', '.sw-panel .sx-cells.cat-link > .sx-field .sx-control']) {
      const decl = rule(layout, selector);
      expect(decl, selector).toMatch(/min-width:\s*0/);
      expect(decl, selector).toMatch(/max-width:\s*100%/);
    }
    // fuori dalla finestra le larghezze minime delle caselle si fermano alla larghezza della cella
    expect(rule(layout, '.cat-link .cat-w-l .sx-control')).toMatch(/min-width:\s*min\(14rem,\s*100%\)\s*;?\s*$/);
    expect(rule(layout, '.cat-link .cat-w-s .sx-control')).toMatch(/min-width:\s*min\(11\.5rem,\s*100%\)\s*;?\s*$/);
    // il nome della voce puo' andare a capo invece di allargare la cella
    expect(rule(layout, '.sw-panel .sx-cells.cat-link > .sx-field .sx-label')).toMatch(/white-space:\s*normal/);
  });
});

describe('Audio su una finestra stretta', () => {
  it('the volume slider wraps under its name instead of leaving the box', () => {
    expect(rule(solo, '.dev-vol')).toMatch(/flex-wrap:\s*wrap/);
    expect(rule(solo, '.dev-vol label')).toMatch(/flex:\s*0 1 auto/);
  });
});
