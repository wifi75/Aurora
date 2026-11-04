// Finestra di conferma del log («Richiedimi di loggare il QSO»): it, en, de, fr, es, pt.
export const logPromptStrings = {
  'lp.title': ['Registrare il QSO con {call}?', 'Log the QSO with {call}?', 'QSO mit {call} loggen?', 'Enregistrer le QSO avec {call} ?', '¿Registrar el QSO con {call}?', 'Registar o QSO com {call}?'],
  'lp.title.lead': ['Registrare il QSO con', 'Log the QSO with', 'QSO loggen mit', 'Enregistrer le QSO avec', '¿Registrar el QSO con', 'Registar o QSO com'],
  'lp.summary': ['Banda, modo, frequenza e ora del QSO', 'Band, mode, frequency and time of the QSO', 'Band, Betriebsart, Frequenz und Zeit des QSO', 'Bande, mode, fréquence et heure du QSO', 'Banda, modo, frecuencia y hora del QSO', 'Banda, modo, frequência e hora do QSO'],
  'lp.waiting': ['{n} QSO in attesa', '{n} QSOs waiting', '{n} QSOs warten', '{n} QSO en attente', '{n} QSO en espera', '{n} QSO à espera'],
  'lp.log': ['Registra', 'Log it', 'Loggen', 'Enregistrer', 'Registrar', 'Registar'],
  'lp.discard': ['Scarta', 'Discard', 'Verwerfen', 'Ignorer', 'Descartar', 'Descartar'],
  'lp.error': ['Non è stato possibile registrare il QSO: riprova.', 'The QSO could not be logged: try again.', 'Das QSO konnte nicht geloggt werden: erneut versuchen.', 'Le QSO n’a pas pu être enregistré : réessaie.', 'No se pudo registrar el QSO: inténtalo de nuevo.', 'Não foi possível registar o QSO: tenta de novo.'],
  'lp.badField': ['Controlla il campo «{field}».', 'Check the “{field}” field.', 'Feld „{field}“ prüfen.', 'Vérifie le champ « {field} ».', 'Revisa el campo «{field}».', 'Verifica o campo «{field}».'],
  'lp.f.call': ['Nominativo', 'Callsign', 'Rufzeichen', 'Indicatif', 'Indicativo', 'Indicativo'],
  'lp.f.grid': ['Locatore', 'Locator', 'Locator', 'Locator', 'Locator', 'Locator'],
  'lp.f.rst_sent': ['Rapporto inviato', 'Report sent', 'Rapport gesendet', 'Report envoyé', 'Reporte enviado', 'Relatório enviado'],
  'lp.f.rst_rcvd': ['Rapporto ricevuto', 'Report received', 'Rapport empfangen', 'Report reçu', 'Reporte recibido', 'Relatório recebido'],
  'lp.f.name': ['Nome', 'Name', 'Name', 'Nom', 'Nombre', 'Nome'],
  'lp.f.qth': ['Città (QTH)', 'Town (QTH)', 'Ort (QTH)', 'Ville (QTH)', 'Ciudad (QTH)', 'Cidade (QTH)'],
  'lp.f.comment': ['Commento', 'Comment', 'Kommentar', 'Commentaire', 'Comentario', 'Comentário'],
} as const;
