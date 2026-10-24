// Solo per i test: la lettura di un file di node_modules.
declare module 'node:fs' {
  export function readFileSync(path: string, encoding: string): string;
}
