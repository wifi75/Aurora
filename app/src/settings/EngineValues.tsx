import { type EngineGroup } from '../lib/engineGroups';
import { NativeValues, useIsNative } from './NativeSettings';

/** Le impostazioni di un gruppo: sono di Aurora (salvate da lei e applicate subito). */
export function EngineValues({ group, title }: { group: EngineGroup; title?: string }) {
  const native = useIsNative();
  return native ? <NativeValues group={group} title={title} /> : null;
}
