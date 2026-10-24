import { beforeEach, describe, expect, it } from 'vitest';
import { useJtdxDraft } from './jtdxDraft';

beforeEach(() => useJtdxDraft.getState().clear());

describe('useJtdxDraft', () => {
  it('keeps a change and drops it again when the value goes back to the current one', () => {
    const { set } = useJtdxDraft.getState();
    set('NDepth', '2', '3');
    expect(useJtdxDraft.getState().changes).toEqual({ NDepth: '2' });
    set('NDepth', '3', '3');
    expect(useJtdxDraft.getState().changes).toEqual({});
  });

  it('treats a missing current value as empty and can be cleared', () => {
    const { set, clear } = useJtdxDraft.getState();
    set('MyCall', '', undefined);
    expect(useJtdxDraft.getState().changes).toEqual({});
    set('MyCall', 'IU3ABC', undefined);
    set('MyGrid', 'IO91wm', 'JN45');
    expect(Object.keys(useJtdxDraft.getState().changes).sort()).toEqual(['MyCall', 'MyGrid']);
    clear();
    expect(useJtdxDraft.getState().changes).toEqual({});
  });
});
