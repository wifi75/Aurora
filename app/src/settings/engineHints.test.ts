import { describe, expect, it } from 'vitest';
import { STRINGS } from '../i18n';
import { ENGINE_GROUPS } from '../lib/engineGroups';
import { REPORT_GROUPS } from '../lib/engineSettingsLayout';

describe('engine hints', () => {
  it('every engine setting has its explanation for the «?»', () => {
    const keys = new Set<string>();
    for (const items of Object.values(ENGINE_GROUPS)) for (const i of items) keys.add(i.key);
    for (const g of REPORT_GROUPS) for (const r of g.rows) keys.add(r.key);
    const missing = [...keys].filter((k) => !(`jxh.${k}` in STRINGS));
    expect(missing).toEqual([]);
  });
});
