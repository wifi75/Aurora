import type { Entry } from '../core';

/** Pagina Statistiche. [it, en, de, fr, es, pt] */
export const statsStrings = {
  'stats.title': ['Statistiche', 'Statistics', 'Statistik', 'Statistiques', 'Estadísticas', 'Estatísticas'],
  'stats.unavailable': [
    'Le statistiche non sono disponibili: Aurora non ha trovato il log dei QSO.',
    'Statistics are not available: Aurora did not find the QSO log.',
    'Statistik nicht verfügbar: Aurora hat das QSO-Log nicht gefunden.',
    "Statistiques indisponibles : Aurora n'a pas trouvé le log des QSO.",
    'Estadísticas no disponibles: Aurora no encontró el log de QSO.',
    'Estatísticas indisponíveis: a Aurora não encontrou o log de QSO.',
  ],
  'stats.awards': ['Premi', 'Awards', 'Diplome', 'Diplômes', 'Diplomas', 'Diplomas'],
  'stats.qsos': ['QSO nel log', 'QSOs in the log', 'QSOs im Log', 'QSO dans le log', 'QSO en el log', 'QSO no log'],
  'stats.calls': ['Nominativi diversi', 'Different callsigns', 'Verschiedene Rufzeichen', 'Indicatifs différents', 'Indicativos distintos', 'Indicativos diferentes'],
  'stats.dxcc': ['Paesi DXCC', 'DXCC countries', 'DXCC-Länder', 'Pays DXCC', 'Países DXCC', 'Países DXCC'],
  'stats.cq': ['Zone CQ', 'CQ zones', 'CQ-Zonen', 'Zones CQ', 'Zonas CQ', 'Zonas CQ'],
  'stats.itu': ['Zone ITU', 'ITU zones', 'ITU-Zonen', 'Zones ITU', 'Zonas ITU', 'Zonas ITU'],
  'stats.grids': ['Quadranti (4 caratteri)', 'Grids (4 characters)', 'Locator (4 Zeichen)', 'Locators (4 caractères)', 'Locators (4 caracteres)', 'Locators (4 caracteres)'],
  'stats.prefixes': ['Prefissi', 'Prefixes', 'Präfixe', 'Préfixes', 'Prefijos', 'Prefixos'],
  'stats.byBand': ['Per banda', 'By band', 'Pro Band', 'Par bande', 'Por banda', 'Por banda'],
  'stats.band': ['Banda', 'Band', 'Band', 'Bande', 'Banda', 'Banda'],
  'stats.bandDxcc': ['Paesi', 'Countries', 'Länder', 'Pays', 'Países', 'Países'],
  'stats.bandQsos': ['QSO', 'QSOs', 'QSOs', 'QSO', 'QSO', 'QSO'],
  'stats.missing': ['Paesi che ti mancano', 'Countries you still need', 'Fehlende Länder', 'Pays qui te manquent', 'Países que te faltan', 'Países que te faltam'],
  'stats.missingNone': ['Hai lavorato tutti i paesi.', 'You have worked every country.', 'Du hast alle Länder gearbeitet.', 'Tu as contacté tous les pays.', 'Has trabajado todos los países.', 'Trabalhaste todos os países.'],
  'stats.of': ['{n} di {total}', '{n} of {total}', '{n} von {total}', '{n} sur {total}', '{n} de {total}', '{n} de {total}'],
  'tip.stats.award': [
    'Avanzamento calcolato dal log dei QSO già fatti.',
    'Progress computed from the log of QSOs already made.',
    'Fortschritt, berechnet aus dem Log der bisherigen QSOs.',
    'Progression calculée depuis le log des QSO déjà faits.',
    'Avance calculado desde el log de QSO ya hechos.',
    'Progresso calculado a partir do log de QSO já feitos.',
  ],
} as const satisfies Record<string, Entry>;
