import react from '@vitejs/plugin-react';
import { defineConfig } from 'vitest/config';
import { readFileSync } from 'node:fs';

/** La versione dell'interfaccia, per accorgersi quando il core e' stato aggiornato (telefono: ricarica senza cache). */
const version = (JSON.parse(readFileSync('package.json', 'utf8')) as { version: string }).version;

// In sviluppo l'interfaccia fa da proxy verso il core (porta 8787), WebSocket compreso.
export default defineConfig({
  plugins: [react()],
  define: { __AURORA_UI_VERSION__: JSON.stringify(version) },
  // macOS Monterey ha un Safari vecchio (15/16): senza un traguardo esplicito l'interfaccia non partiva sul Mac di un utente (#108)
  build: { target: ['es2021', 'safari14'] },
  server: {
    port: 5173,
    proxy: {
      // xfwd: il core vede che la richiesta e' passata da un proxy e rifiuta di dare codici di associazione
      '/api': { target: 'http://127.0.0.1:8787', ws: true, changeOrigin: false, xfwd: true },
    },
  },
  test: {
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: false,
  },
});
