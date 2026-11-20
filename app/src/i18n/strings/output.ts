import type { Entry } from '../core';

/** Livello audio di uscita (cursore «Lvl TX» (prima «Pwr») del motore). [it, en, de, fr, es, pt] */
export const outputStrings = {
  'out.label': ['Livello audio di uscita (Lvl TX)', 'Audio output level (TX Lvl)', 'Audio-Ausgangspegel (TX-Pegel)', 'Niveau audio de sortie (Niv TX)', 'Nivel de audio de salida (Niv TX)', 'Nível de áudio de saída (Niv TX)'],
  'out.value': ['{pct} % · attenuazione {db} dB', '{pct} % · attenuation {db} dB', '{pct} % · Dämpfung {db} dB', '{pct} % · atténuation {db} dB', '{pct} % · atenuación {db} dB', '{pct} % · atenuação {db} dB'],
  'out.unavailable': [
    'Livello di uscita non disponibile (serve il comando radio e Aurora acceso).',
    'Output level not available (needs radio control and Aurora running).',
    'Ausgangspegel nicht verfügbar (Funkgerätesteuerung und laufendes Aurora nötig).',
    "Niveau de sortie indisponible (il faut la commande radio et Aurora lancé).",
    'Nivel de salida no disponible (necesita el control de radio y Aurora en marcha).',
    'Nível de saída indisponível (precisa do controlo do rádio e da Aurora ligado).',
  ],
  'out.saved': ['Livello di uscita impostato.', 'Output level set.', 'Ausgangspegel gesetzt.', 'Niveau de sortie réglé.', 'Nivel de salida fijado.', 'Nível de saída definido.'],
  'tip.out': [
    "Livello audio di uscita verso la radio. Con l'ALC a zero o appena visibile il livello è giusto; se l'ALC è alto abbassa questo livello, non alzare la potenza. I watt si regolano con RF Power sulla radio. Si regola anche a trasmissione ferma.",
    "Audio output level to the radio. With the ALC at zero or barely visible the level is right; if the ALC is high, lower this level, do not raise the power. The watts are set with RF Power on the radio. It can be set with transmission stopped too.",
    "Audio-Ausgangspegel zum Funkgerät. Steht der ALC auf null oder kaum sichtbar, stimmt der Pegel; ist der ALC hoch, senke diesen Pegel, erhöhe nicht die Leistung. Die Watt stellst du mit RF Power am Funkgerät ein. Auch bei gestopptem Senden einstellbar.",
    "Niveau audio de sortie vers la radio. Avec l'ALC à zéro ou à peine visible, le niveau est bon ; si l'ALC est haut, baisse ce niveau, n'augmente pas la puissance. Les watts se règlent avec RF Power sur la radio. Réglable aussi à l'arrêt de l'émission.",
    "Nivel de audio de salida hacia la radio. Con el ALC a cero o apenas visible el nivel es correcto; si el ALC está alto, baja este nivel, no subas la potencia. Los vatios se ajustan con RF Power en la radio. Se ajusta también con la transmisión parada.",
    "Nível de áudio de saída para o rádio. Com o ALC a zero ou apenas visível o nível está certo; se o ALC estiver alto, baixa este nível, não subas a potência. Os watts regulam-se com RF Power no rádio. Ajusta-se também com a transmissão parada.",
  ],
  'tip.out.mode': [
    "Spento (di fabbrica, come WSJT-X e JTDX): il cursore attenua il segnale generato e Aurora non tocca mai il volume di Windows, che resta dove lo hai messo. Acceso: il segnale resta pieno e il cursore comanda il volume di Windows dell'uscita verso la radio, come nelle versioni precedenti.",
    "Off (factory setting, like WSJT-X and JTDX): the slider lowers the generated signal and Aurora never touches the Windows volume, which stays where you set it. On: the signal stays full and the slider sets the Windows volume of the output to the radio, as in earlier versions.",
    "Aus (Werkseinstellung, wie WSJT-X und JTDX): Der Regler senkt das erzeugte Signal, und Aurora berührt die Windows-Lautstärke nie; sie bleibt, wo du sie eingestellt hast. Ein: Das Signal bleibt voll und der Regler stellt die Windows-Lautstärke des Ausgangs zum Funkgerät ein, wie in früheren Versionen.",
    "Désactivé (réglage d'usine, comme WSJT-X et JTDX) : le curseur baisse le signal généré et Aurora ne touche jamais au volume de Windows, qui reste où tu l'as mis. Activé : le signal reste plein et le curseur règle le volume Windows de la sortie vers la radio, comme dans les versions précédentes.",
    "Desactivado (de fábrica, como WSJT-X y JTDX): el control baja la señal generada y Aurora nunca toca el volumen de Windows, que queda donde lo pusiste. Activado: la señal queda plena y el control ajusta el volumen de Windows de la salida hacia la radio, como en las versiones anteriores.",
    "Desligado (de fábrica, como o WSJT-X e o JTDX): o cursor baixa o sinal gerado e a Aurora nunca toca no volume do Windows, que fica onde o puseste. Ligado: o sinal fica cheio e o cursor regula o volume do Windows da saída para o rádio, como nas versões anteriores.",
  ],
  'out.short': ["Lvl TX", "TX Lvl", "TX-Pegel", "Niv TX", "Niv TX", "Niv TX"],
  'out.mode': ["Lvl TX = volume di Windows", "TX Lvl = Windows volume", "TX-Pegel = Windows-Lautstärke", "Niv TX = volume Windows", "Niv TX = volumen de Windows", "Niv TX = volume do Windows"],
} as const satisfies Record<string, Entry>;
