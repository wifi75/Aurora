import type { Entry } from '../core';

/** Livello audio di uscita (cursore "Pwr" di JTDX). [it, en, de, fr, es, pt] */
export const outputStrings = {
  'out.label': ['Livello audio di uscita (Pwr)', 'Audio output level (Pwr)', 'Audio-Ausgangspegel (Pwr)', 'Niveau audio de sortie (Pwr)', 'Nivel de audio de salida (Pwr)', 'Nível de áudio de saída (Pwr)'],
  'out.value': ['{pct} % · attenuazione {db} dB', '{pct} % · attenuation {db} dB', '{pct} % · Dämpfung {db} dB', '{pct} % · atténuation {db} dB', '{pct} % · atenuación {db} dB', '{pct} % · atenuação {db} dB'],
  'out.unavailable': [
    'Livello di uscita non disponibile (serve il comando radio e JTDX acceso).',
    'Output level not available (needs radio control and JTDX running).',
    'Ausgangspegel nicht verfügbar (Funkgerätesteuerung und laufendes JTDX nötig).',
    "Niveau de sortie indisponible (il faut la commande radio et JTDX lancé).",
    'Nivel de salida no disponible (necesita el control de radio y JTDX en marcha).',
    'Nível de saída indisponível (precisa do controlo do rádio e do JTDX ligado).',
  ],
  'out.saved': ['Livello di uscita impostato.', 'Output level set.', 'Ausgangspegel gesetzt.', 'Niveau de sortie réglé.', 'Nivel de salida fijado.', 'Nível de saída definido.'],
  'tip.out': [
    "Regola il livello audio di uscita. Se l'RTX è fuori ALC abbassa questo livello. Per un'emissione ottima il livello ALC deve rimanere a 0. Si cambia solo con la trasmissione abilitata.",
    'Adjusts the audio output level. If the transceiver is out of ALC, lower this level. For an optimal emission the ALC level must stay at 0. It can only be changed with transmission enabled.',
    'Stellt den Audio-Ausgangspegel ein. Wenn das Funkgerät außerhalb des ALC arbeitet, senke diesen Pegel. Für eine optimale Aussendung muss der ALC-Pegel bei 0 bleiben. Nur bei freigegebenem Senden änderbar.',
    "Règle le niveau audio de sortie. Si l'émetteur-récepteur dépasse l'ALC, baisse ce niveau. Pour une émission optimale, le niveau ALC doit rester à 0. Modifiable seulement avec l'émission activée.",
    'Ajusta el nivel de audio de salida. Si el equipo sale del ALC, baja este nivel. Para una emisión óptima el nivel ALC debe quedar en 0. Solo se cambia con la transmisión habilitada.',
    'Ajusta o nível de áudio de saída. Se o rádio sai do ALC, baixa este nível. Para uma emissão ótima o nível ALC deve ficar em 0. Só se muda com a transmissão ativa.',
  ],
} as const satisfies Record<string, Entry>;
