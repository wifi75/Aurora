import { describe, expect, it } from 'vitest';
import { STRINGS } from '../i18n';
import { parseRoute } from './route';
import { allPages, INFO_ID, isKnownNode, LEGACY_IDS, modeNode, OLD_TAB_IDS, resolveNode, settingsNodeFor, SETTINGS_TREE } from './settingsTree';

describe("l'albero delle Impostazioni", () => {
  it('ha cinque famiglie con i colori della bozza', () => {
    expect(SETTINGS_TREE.map((f) => [f.id, f.color])).toEqual([
      ['station', '#2dd4bf'],
      ['modes', '#8e7bff'],
      ['log', '#f5a524'],
      ['look', '#4cc3ff'],
      ['access', '#f472b6'],
    ]);
  });

  it('ha id unici, validi negli indirizzi e con un nome tradotto', () => {
    const ids = [...SETTINGS_TREE.map((f) => f.id), ...SETTINGS_TREE.flatMap((f) => f.items.flatMap((i) => (i.kind === 'sub' ? [i.id, ...i.pages.map((p) => p.id)] : [i.id]))), INFO_ID];
    expect(new Set(ids).size).toBe(ids.length);
    for (const id of ids) {
      expect(id).toMatch(/^[a-z0-9]{1,24}$/);
      expect(parseRoute(`#/impostazioni/${id}`)).toEqual({ name: 'settings', tab: id });
    }
    const labels = [...SETTINGS_TREE.map((f) => f.label), ...SETTINGS_TREE.flatMap((f) => f.items.flatMap((i) => (i.kind === 'sub' ? [i.label, ...i.pages.map((p) => p.label)] : [i.label])))];
    for (const label of labels) expect(STRINGS[label]).toBeDefined();
  });
});

describe('i collegamenti vecchi', () => {
  it('ogni scheda di prima porta a una pagina nuova', () => {
    const pages = new Set(allPages().map((p) => p.page.id));
    for (const old of OLD_TAB_IDS) {
      const path = resolveNode(old);
      if (old === 'info') {
        expect(path.page).toBe(INFO_ID);
        continue;
      }
      expect(pages.has(path.page), old).toBe(true);
      expect(path.family, old).not.toBeNull();
    }
  });

  it('le schede che hanno cambiato nome vanno alla pagina giusta', () => {
    expect(resolveNode('general')).toMatchObject({ page: 'personal', family: { id: 'station' }, sub: null });
    expect(resolveNode('filters')).toMatchObject({ page: 'messages', family: { id: 'modes' }, sub: { id: 'modeft8' } });
    expect(resolveNode('cqnews').page).toBe('messages');
    expect(resolveNode('sstvtx')).toMatchObject({ page: 'sstvtx', family: { id: 'modes' }, sub: { id: 'modesstv' } });
    expect(resolveNode('automation')).toMatchObject({ page: 'automation', sub: { id: 'modeft8' } });
    expect(resolveNode('support')).toMatchObject({ page: 'support', family: { id: 'access' } });
    for (const target of Object.values(LEGACY_IDS)) expect(allPages().some((p) => p.page.id === target)).toBe(true);
  });

  it('senza id, o con un id sconosciuto, si apre Informazioni con tutto chiuso', () => {
    for (const id of [undefined, null, '', 'nonesiste']) {
      expect(resolveNode(id)).toEqual({ family: null, sub: null, page: INFO_ID });
    }
    expect(isKnownNode('nonesiste')).toBe(false);
    expect(isKnownNode('general')).toBe(true);
    expect(isKnownNode(INFO_ID)).toBe(true);
  });

  it('una sottofamiglia o una famiglia aprono la loro prima pagina', () => {
    expect(resolveNode('modedigi')).toMatchObject({ page: 'digi', sub: { id: 'modedigi' } });
    expect(resolveNode('modesat')).toMatchObject({ page: 'satorbits', sub: { id: 'modesat' } });
    expect(resolveNode('station')).toMatchObject({ page: 'personal', family: { id: 'station' } });
  });
});

describe("l'ingranaggio delle pagine dei modi", () => {
  it('porta al nodo della pagina in cui si e', () => {
    expect(settingsNodeFor('digi', 'FT8')).toBe('modedigi');
    expect(settingsNodeFor('sstv', 'FT8')).toBe('modesstv');
    expect(settingsNodeFor('sat', 'FT8')).toBe('modesat');
    expect(settingsNodeFor('contest', 'FT8')).toBe('contest');
    expect(settingsNodeFor('fox', 'FT8')).toBe('contest');
    expect(settingsNodeFor('dxmap', 'FT8')).toBeUndefined();
  });

  it('nella Stazione segue il modo in uso', () => {
    expect(settingsNodeFor('live', 'FT8')).toBe('modeft8');
    expect(modeNode('FT4')).toBe('modeft8');
    expect(modeNode('JT65')).toBe('jt');
    expect(modeNode('JT9')).toBe('jt');
    expect(modeNode('FST4-60')).toBe('jt');
    expect(modeNode('FST4W-300')).toBe('wspr');
    expect(modeNode('WSPR')).toBe('wspr');
    expect(modeNode('Q65-60A')).toBe('vhf');
    expect(modeNode('MSK144')).toBe('vhf');
    expect(modeNode('JT4')).toBe('vhf');
    for (const node of ['modeft8', 'jt', 'wspr', 'vhf', 'modedigi', 'modesstv', 'modesat', 'contest']) expect(isKnownNode(node)).toBe(true);
  });
});
