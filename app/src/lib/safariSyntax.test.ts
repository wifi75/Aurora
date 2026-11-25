import { describe, expect, it } from 'vitest';

// Tutti i sorgenti dell'interfaccia (come testo): sono quelli che il Safari del Mac legge.
const sources = import.meta.glob('../**/*.{ts,tsx}', { query: '?raw', import: 'default', eager: true }) as Record<string, string>;

describe('sintassi che i Safari vecchi non capiscono', () => {
  it("nessuna espressione regolare con lookbehind: Safari prima del 16.4 (macOS Monterey) ferma l'interfaccia (#108)", () => {
    const bad: string[] = [];
    for (const [file, text] of Object.entries(sources)) {
      if (file.endsWith('.test.ts') || file.endsWith('.test.tsx')) continue;
      text.split('\n').forEach((line, i) => {
        // le righe di commento possono nominarlo
        if (/^\s*(\/\/|\*|\/\*)/.test(line)) return;
        if (/\(\?<[=!]/.test(line)) bad.push(`${file}:${i + 1}`);
      });
    }
    expect(bad).toEqual([]);
  });
});
