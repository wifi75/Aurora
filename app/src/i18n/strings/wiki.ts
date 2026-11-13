import type { Entry } from '../core';

/** Guida: titolo, introduzione e ricerca. I capitoli stanno in `i18n/guide.ts`. [it, en, de, fr, es, pt] */
export const wikiStrings = {
  'wiki.title': ['Guida', 'Guide', 'Anleitung', 'Guide', 'Guía', 'Guia'],
  'wiki.intro': [
    'Aurora è lo schermo e il cervello della stazione: decodifica, trasmette e comanda la radio.',
    'Aurora is the screen and brain of the station: it decodes, transmits and drives the radio.',
    'Aurora ist Bildschirm und Gehirn der Station: dekodiert, sendet und steuert das Funkgerät.',
    'Aurora est l\'écran et le cerveau de la station : elle décode, émet et pilote la radio.',
    'Aurora es la pantalla y el cerebro de la estación: decodifica, transmite y maneja la radio.',
    'A Aurora é o ecrã e o cérebro da estação: descodifica, transmite e comanda o rádio.',
  ],
  'guide.hits': ['{n} di {total}', '{n} of {total}', '{n} von {total}', '{n} sur {total}', '{n} de {total}', '{n} de {total}'],
  'guide.noHits': ['Nessun risultato', 'No results', 'Keine Treffer', 'Aucun résultat', 'Sin resultados', 'Sem resultados'],
  'guide.prevHit': ['Risultato precedente (Maiusc+Invio)', 'Previous result (Shift+Enter)', 'Vorheriger Treffer (Umschalt+Eingabe)', 'Résultat précédent (Maj+Entrée)', 'Resultado anterior (Mayús+Intro)', 'Resultado anterior (Shift+Enter)'],
  'guide.nextHit': ['Risultato successivo (Invio)', 'Next result (Enter)', 'Nächster Treffer (Eingabe)', 'Résultat suivant (Entrée)', 'Resultado siguiente (Intro)', 'Resultado seguinte (Enter)'],
  'guide.search': ['Cerca nella guida…', 'Search the guide…', 'In der Anleitung suchen…', 'Chercher dans le guide…', 'Buscar en la guía…', 'Pesquisar no guia…'],
  'guide.fix': ['Se non va', 'If it does not work', 'Wenn es nicht geht', 'Si ça ne marche pas', 'Si no funciona', 'Se não funcionar'],
  'guide.none': ['Niente trovato per questa ricerca.', 'Nothing found for this search.', 'Nichts gefunden.', 'Rien trouvé pour cette recherche.', 'Nada encontrado para esta búsqueda.', 'Nada encontrado para esta pesquisa.'],
} as const satisfies Record<string, Entry>;
