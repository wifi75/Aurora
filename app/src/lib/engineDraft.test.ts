import { beforeEach, describe, expect, it } from 'vitest';
import { useEngineDraft } from './engineDraft';

beforeEach(() => useEngineDraft.getState().clear());

describe('useEngineDraft', () => {
  it('keeps a change and drops it again when the value goes back to the current one', () => {
    const { set } = useEngineDraft.getState();
    set('NDepth', '2', '3');
    expect(useEngineDraft.getState().changes).toEqual({ NDepth: '2' });
    set('NDepth', '3', '3');
    expect(useEngineDraft.getState().changes).toEqual({});
  });

  it('treats a missing current value as empty and can be cleared', () => {
    const { set, clear } = useEngineDraft.getState();
    set('MyCall', '', undefined);
    expect(useEngineDraft.getState().changes).toEqual({});
    set('MyCall', 'IU3ABC', undefined);
    set('MyGrid', 'IO91wm', 'JN45');
    expect(Object.keys(useEngineDraft.getState().changes).sort()).toEqual(['MyCall', 'MyGrid']);
    clear();
    expect(useEngineDraft.getState().changes).toEqual({});
  });
});
