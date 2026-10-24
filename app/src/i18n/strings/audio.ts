import type { Entry } from '../core';

/** Livello dell'audio in ingresso (indicatore RX). [it, en, de, fr, es, pt] */
export const audioStrings = {
  'rxl.title': ['Rx', 'Rx', 'Rx', 'Rx', 'Rx', 'Rx'],
  'rxl.label': ['Livello dell’audio in ingresso', 'Input audio level', 'Pegel des Audio-Eingangs', 'Niveau de l’audio d’entrée', 'Nivel del audio de entrada', 'Nível do áudio de entrada'],
  'rxl.value': ['picco {peak} dB · medio {rms} dB', 'peak {peak} dB · average {rms} dB', 'Spitze {peak} dB · Mittel {rms} dB', 'crête {peak} dB · moyen {rms} dB', 'pico {peak} dB · medio {rms} dB', 'pico {peak} dB · médio {rms} dB'],
  'rxl.state.silent': ['Muto', 'Silent', 'Stumm', 'Muet', 'Mudo', 'Mudo'],
  'rxl.state.low': ['Basso', 'Low', 'Niedrig', 'Faible', 'Bajo', 'Baixo'],
  'rxl.state.good': ['Giusto', 'Good', 'Gut', 'Bon', 'Correcto', 'Bom'],
  'rxl.state.high': ['Alto', 'High', 'Hoch', 'Élevé', 'Alto', 'Alto'],
  'rxl.state.overdrive': ['Saturo', 'Overdriven', 'Übersteuert', 'Saturé', 'Saturado', 'Saturado'],
  'rxl.hint.silent': [
    'Nessun segnale da troppo tempo: controlla il cavo, il dispositivo scelto e il volume di registrazione di Windows.',
    'No signal for too long: check the cable, the chosen device and the Windows recording volume.',
    'Seit zu langer Zeit kein Signal: Prüfe Kabel, gewähltes Gerät und die Windows-Aufnahmelautstärke.',
    'Aucun signal depuis trop longtemps : vérifie le câble, le périphérique choisi et le volume d’enregistrement de Windows.',
    'Sin señal desde hace demasiado: comprueba el cable, el dispositivo elegido y el volumen de grabación de Windows.',
    'Sem sinal há demasiado tempo: verifica o cabo, o dispositivo escolhido e o volume de gravação do Windows.',
  ],
  'rxl.hint.low': [
    'Segnale debole: i decodificatori perdono i messaggi deboli. Alza il volume di registrazione.',
    'Weak signal: decoders miss weak messages. Raise the recording volume.',
    'Schwaches Signal: Die Dekodierer verpassen schwache Meldungen. Erhöhe die Aufnahmelautstärke.',
    'Signal faible : les décodeurs perdent les messages faibles. Monte le volume d’enregistrement.',
    'Señal débil: los decodificadores pierden mensajes débiles. Sube el volumen de grabación.',
    'Sinal fraco: os decodificadores perdem mensagens fracas. Aumenta o volume de gravação.',
  ],
  'rxl.hint.good': [
    'Livello giusto per decodificare.',
    'Good level for decoding.',
    'Guter Pegel zum Dekodieren.',
    'Bon niveau pour décoder.',
    'Nivel correcto para decodificar.',
    'Nível certo para decodificar.',
  ],
  'rxl.hint.high': [
    'Livello alto: un po’ di margine in più eviterebbe la saturazione.',
    'High level: a little more headroom would avoid overdrive.',
    'Hoher Pegel: etwas mehr Reserve würde Übersteuerung vermeiden.',
    'Niveau élevé : un peu plus de marge éviterait la saturation.',
    'Nivel alto: algo más de margen evitaría la saturación.',
    'Nível alto: um pouco mais de margem evitaria a saturação.',
  ],
  'rxl.hint.overdrive': [
    'Ingresso saturo: i segnali si distorcono e i decode peggiorano. Abbassa il volume di registrazione di Windows o il livello audio della radio.',
    'Input overdriven: signals distort and decodes get worse. Lower the Windows recording volume or the radio audio level.',
    'Eingang übersteuert: Signale verzerren und die Dekodierung leidet. Senke die Windows-Aufnahmelautstärke oder den Audiopegel des Funkgeräts.',
    'Entrée saturée : les signaux se déforment et les décodages se dégradent. Baisse le volume d’enregistrement de Windows ou le niveau audio de la radio.',
    'Entrada saturada: las señales se distorsionan y los decodes empeoran. Baja el volumen de grabación de Windows o el nivel de audio de la radio.',
    'Entrada saturada: os sinais distorcem e as descodificações pioram. Baixa o volume de gravação do Windows ou o nível de áudio do rádio.',
  ],
} as const satisfies Record<string, Entry>;
