import type { Entry } from '../core';

/** Diagnosi automatica: cio' che non va nella stazione, detto in chiaro. [it, en, de, fr, es, pt] */
export const diagnosisStrings = {
  'diag.aria': ['Diagnosi della stazione', 'Station diagnosis', 'Stationsdiagnose', 'Diagnostic de la station', 'Diagnóstico de la estación', 'Diagnóstico da estação'],
  'diag.fix': ['Sistema', 'Fix', 'Beheben', 'Corriger', 'Arreglar', 'Corrigir'],
  'diag.clock': [
    'L’orologio del PC è fuori di {ms} ms rispetto al tempo di rete: i messaggi deboli si perdono. Sincronizzalo.',
    'The PC clock is off by {ms} ms from network time: weak messages get lost. Synchronise it.',
    'Die PC-Uhr weicht {ms} ms von der Netzzeit ab: schwache Meldungen gehen verloren. Synchronisiere sie.',
    'L’horloge du PC est décalée de {ms} ms par rapport à l’heure réseau : les messages faibles se perdent. Synchronise-la.',
    'El reloj del PC está desviado {ms} ms respecto a la hora de red: se pierden los mensajes débiles. Sincronízalo.',
    'O relógio do PC está desviado {ms} ms da hora da rede: perdem-se as mensagens fracas. Sincroniza-o.',
  ],
  'diag.callsign': [
    'Nel motore manca il tuo nominativo: senza, Aurora non sa quali messaggi sono per te. Impostalo nelle impostazioni del motore.',
    'Your callsign is missing in the engine: without it Aurora cannot tell which messages are for you. Set it in the engine settings.',
    'Im Motor fehlt dein Rufzeichen: ohne es weiß Aurora nicht, welche Nachrichten für dich sind. Stelle es in den Motoreinstellungen ein.',
    'L’indicatif manque dans le moteur : sans lui Aurora ne sait pas quels messages te sont destinés. Renseigne-le dans les réglages du moteur.',
    'Falta tu indicativo en el motor: sin él Aurora no sabe qué mensajes son para ti. Configúralo en los ajustes del motor.',
    'Falta o teu indicativo no motor: sem ele a Aurora não sabe quais as mensagens para ti. Define-o nas definições do motor.',
  ],
  'diag.audio.overdrive': [
    'L’ingresso audio è saturo: i segnali si distorcono. Abbassa il volume di registrazione (o accendi AUTO).',
    'The audio input is overdriven: signals distort. Lower the recording volume (or turn on AUTO).',
    'Der Audioeingang ist übersteuert: Signale verzerren. Senke die Aufnahmelautstärke (oder schalte AUTO ein).',
    'L’entrée audio est saturée : les signaux se déforment. Baisse le volume d’enregistrement (ou active AUTO).',
    'La entrada de audio está saturada: las señales se distorsionan. Baja el volumen de grabación (o activa AUTO).',
    'A entrada de áudio está saturada: os sinais distorcem. Baixa o volume de gravação (ou liga o AUTO).',
  ],
  'diag.audio.silent': [
    'L’ingresso audio è muto da troppo tempo: controlla il cavo, il dispositivo scelto e il volume di registrazione di Windows.',
    'The audio input has been silent for too long: check the cable, the chosen device and the Windows recording volume.',
    'Der Audioeingang ist seit zu langer Zeit stumm: Prüfe Kabel, gewähltes Gerät und die Windows-Aufnahmelautstärke.',
    'L’entrée audio est muette depuis trop longtemps : vérifie le câble, le périphérique choisi et le volume d’enregistrement de Windows.',
    'La entrada de audio lleva demasiado tiempo muda: comprueba el cable, el dispositivo elegido y el volumen de grabación de Windows.',
    'A entrada de áudio está muda há demasiado tempo: verifica o cabo, o dispositivo escolhido e o volume de gravação do Windows.',
  ],
  'diag.audio.low': [
    'Il segnale in ingresso è debole: alza il volume di registrazione (o accendi AUTO).',
    'The input signal is weak: raise the recording volume (or turn on AUTO).',
    'Das Eingangssignal ist schwach: Erhöhe die Aufnahmelautstärke (oder schalte AUTO ein).',
    'Le signal d’entrée est faible : monte le volume d’enregistrement (ou active AUTO).',
    'La señal de entrada es débil: sube el volumen de grabación (o activa AUTO).',
    'O sinal de entrada é fraco: aumenta o volume de gravação (ou liga o AUTO).',
  ],
  'diag.quiet': [
    'Nessun messaggio decodificato da {min} minuti: banda chiusa oppure ricezione ferma (controlla Monitora e l’audio).',
    'No message decoded for {min} minutes: the band may be closed or reception stalled (check Monitor and the audio).',
    'Seit {min} Minuten keine Nachricht dekodiert: Band geschlossen oder Empfang steht (prüfe Monitor und Audio).',
    'Aucun message décodé depuis {min} minutes : bande fermée ou réception à l’arrêt (vérifie Moniteur et l’audio).',
    'Ningún mensaje decodificado desde hace {min} minutos: banda cerrada o recepción detenida (comprueba Monitor y el audio).',
    'Nenhuma mensagem descodificada há {min} minutos: banda fechada ou receção parada (verifica Monitorar e o áudio).',
  ],
} as const satisfies Record<string, Entry>;
