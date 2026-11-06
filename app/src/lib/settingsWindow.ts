/** Apre le Impostazioni nella loro finestra a parte (nel programma per Windows la crea il guscio, piccola e al centro). */
export function openSettingsWindow(tab?: string): void {
  const q = tab && /^[a-z]{1,12}$/.test(tab) ? `?tab=${tab}` : '';
  window.open(`${location.origin}/${q}#/finestra/impostazioni`, 'aurora-impostazioni', 'width=1040,height=760');
}
