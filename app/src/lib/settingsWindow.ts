/**
 * Apre le Impostazioni nella loro finestra a parte (nel programma per Windows la crea il guscio, piccola e al centro), sul nodo `tab`
 * dell'albero (pagina, sottofamiglia o id di una vecchia scheda); senza, su Informazioni.
 */
export function openSettingsWindow(tab?: string): void {
  const q = tab && /^[a-z0-9]{1,24}$/.test(tab) ? `?tab=${tab}` : '';
  window.open(`${location.origin}/${q}#/finestra/impostazioni`, 'aurora-impostazioni', 'width=1120,height=760');
}

/** Evento con cui un collegamento dentro una scheda chiede alla finestra Impostazioni di passare a un'altra scheda. */
export const SETTINGS_TAB_EVENT = 'aurora-settings-tab';

/**
 * Porta la finestra Impostazioni gia' aperta (riquadro o finestra a parte) sulla scheda `tab`.
 * Un semplice `#/impostazioni/<scheda>` non basta: nel riquadro la scheda scelta resta quella dell'apertura e nella finestra a parte
 * l'indirizzo cambierebbe pagina. Se nessuna finestra Impostazioni risponde si apre la scheda dall'indirizzo, come prima.
 */
export function selectSettingsTab(tab: string): void {
  const handled = !window.dispatchEvent(new CustomEvent<string>(SETTINGS_TAB_EVENT, { detail: tab, cancelable: true }));
  if (!handled) location.hash = `#/impostazioni/${tab}`;
}
