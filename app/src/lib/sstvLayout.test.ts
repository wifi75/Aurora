import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';

// Pagina SSTV in ricezione (richiesta di Luc, #102): le informazioni (modo, righe, sincronismo, livello, sintonia) stanno a sinistra, l'immagine
// ricevuta grande al centro; con una colonna sola (finestra stretta) l'immagine sta sopra.
describe('SSTV in ricezione: disposizione delle colonne', () => {
  const css = readFileSync('src/styles/sstv.css', 'utf8');

  it('il pannello delle informazioni va prima dell\'immagine, e torna dopo con una colonna sola', () => {
    expect(css).toMatch(/\.sstv-rx > \.sstv-side \{ order: -1; \}/);
    const narrow = css.slice(css.indexOf('@media (max-width: 820px)'));
    expect(narrow).toMatch(/\.sstv-rx > \.sstv-side \{ order: 0; \}/);
  });

  it('la colonna fissa e\' la prima, quella dell\'immagine prende il resto', () => {
    expect(css).toMatch(/\.sstv-rx \{ display: grid; grid-template-columns: minmax\(300px, 420px\) minmax\(0, 1fr\);/);
    expect(css).toMatch(/\.sstv-rx--log \{ grid-template-columns: minmax\(290px, 380px\) minmax\(0, 46rem\) minmax\(200px, 240px\);/);
  });

  // la finestra dell'immagine non si allunga su tutta la colonna: resta a 4:3 con una larghezza massima (segnalazione del proprietario, 2026-11-24)
  it("lo schermo dell'immagine e' 4:3 e non piu' largo di 46rem", () => {
    expect(css).toMatch(/\.sstv-rx \.sstv-screen \{[^}]*max-width: 46rem;[^}]*aspect-ratio: 4 \/ 3 !important;/);
  });

  // le frecce di scorrimento spente non tengono occupato il lato sinistro: le miniature partono dal bordo (richiesta del proprietario)
  it('le frecce spente delle miniature non occupano posto', () => {
    expect(css).toMatch(/\.sstv-strip-wrap:not\(:has\(\.sstv-strip-arrow:not\(:disabled\)\)\) \.sstv-strip-arrow \{ display: none; \}/);
  });
});
