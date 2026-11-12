import type { Entry } from '../core';

/**
 * I testi che nominano Windows, nella variante per gli altri sistemi (Linux, macOS): stessa chiave con `.sys` in fondo.
 * La scelta la fa la traduzione (`translate`) dal sistema su cui gira il motore di Aurora (`/health`, campo `os`): chi scrive l'interfaccia
 * usa la chiave di sempre. Fuori da Windows il volume di registrazione non si regola da Aurora (si usa il mixer del sistema) e il cursore
 * Pwr abbassa il segnale generato invece del volume della scheda. [it, en, de, fr, es, pt]
 */
export const systemStrings = {
  'rxl.volume.sys': [
    'Volume di registrazione del sistema',
    'System recording volume',
    'Aufnahmelautstärke des Systems',
    'Volume d’enregistrement du système',
    'Volumen de grabación del sistema',
    'Volume de gravação do sistema',
  ],
  'rxl.volume.value.sys': [
    '{pct} % del volume di registrazione del sistema ({device})',
    '{pct} % of the system recording volume ({device})',
    '{pct} % der Aufnahmelautstärke des Systems ({device})',
    '{pct} % du volume d’enregistrement du système ({device})',
    '{pct} % del volumen de grabación del sistema ({device})',
    '{pct} % do volume de gravação do sistema ({device})',
  ],
  'rxl.hint.silent.sys': [
    'Nessun segnale da troppo tempo: controlla il cavo, il dispositivo scelto e il volume di registrazione nel mixer audio del sistema.',
    'No signal for too long: check the cable, the chosen device and the recording volume in the system audio mixer.',
    'Seit zu langer Zeit kein Signal: Prüfe Kabel, gewähltes Gerät und die Aufnahmelautstärke im Audiomixer des Systems.',
    'Aucun signal depuis trop longtemps : vérifie le câble, le périphérique choisi et le volume d’enregistrement dans le mélangeur audio du système.',
    'Sin señal desde hace demasiado: comprueba el cable, el dispositivo elegido y el volumen de grabación en el mezclador de audio del sistema.',
    'Sem sinal há demasiado tempo: verifica o cabo, o dispositivo escolhido e o volume de gravação no misturador de áudio do sistema.',
  ],
  'rxl.hint.overdrive.sys': [
    'Ingresso saturo: i segnali si distorcono e i decode peggiorano. Abbassa il volume di registrazione nel mixer audio del sistema o il livello audio della radio.',
    'Input overdriven: signals distort and decodes get worse. Lower the recording volume in the system audio mixer or the radio audio level.',
    'Eingang übersteuert: Signale verzerren und die Dekodierung leidet. Senke die Aufnahmelautstärke im Audiomixer des Systems oder den Audiopegel des Funkgeräts.',
    'Entrée saturée : les signaux se déforment et les décodages se dégradent. Baisse le volume d’enregistrement dans le mélangeur audio du système ou le niveau audio de la radio.',
    'Entrada saturada: las señales se distorsionan y los decodes empeoran. Baja el volumen de grabación en el mezclador de audio del sistema o el nivel de audio de la radio.',
    'Entrada saturada: os sinais distorcem e as descodificações pioram. Baixa o volume de gravação no misturador de áudio do sistema ou o nível de áudio do rádio.',
  ],
  'diag.audio.silent.sys': [
    'L’ingresso audio è muto da troppo tempo: controlla il cavo, il dispositivo scelto e il volume di registrazione nel mixer audio del sistema.',
    'The audio input has been silent for too long: check the cable, the chosen device and the recording volume in the system audio mixer.',
    'Der Audioeingang ist seit zu langer Zeit stumm: Prüfe Kabel, gewähltes Gerät und die Aufnahmelautstärke im Audiomixer des Systems.',
    'L’entrée audio est muette depuis trop longtemps : vérifie le câble, le périphérique choisi et le volume d’enregistrement dans le mélangeur audio du système.',
    'La entrada de audio lleva demasiado tiempo muda: comprueba el cable, el dispositivo elegido y el volumen de grabación en el mezclador de audio del sistema.',
    'A entrada de áudio está muda há demasiado tempo: verifica o cabo, o dispositivo escolhido e o volume de gravação no misturador de áudio do sistema.',
  ],
  'jxh.SoundOutName.sys': [
    'La scheda audio che manda il segnale alla radio per trasmettere. Il cursore Pwr abbassa il segnale generato; il volume dell’uscita si regola una volta nel mixer audio del sistema.',
    'The sound card that sends the signal to the radio to transmit. The Pwr slider lowers the generated signal; the output volume is set once in the system audio mixer.',
    'Die Soundkarte, die zum Senden das Signal ans Funkgerät schickt. Der Pwr-Regler senkt das erzeugte Signal; die Lautstärke des Ausgangs wird einmal im Audiomixer des Systems eingestellt.',
    'La carte son qui envoie le signal à la radio pour émettre. Le curseur Pwr baisse le signal généré ; le volume de la sortie se règle une fois dans le mélangeur audio du système.',
    'La tarjeta de sonido que envía la señal a la radio para transmitir. El cursor Pwr baja la señal generada; el volumen de la salida se ajusta una vez en el mezclador de audio del sistema.',
    'A placa de som que envia o sinal ao rádio para transmitir. O cursor Pwr baixa o sinal gerado; o volume da saída regula-se uma vez no misturador de áudio do sistema.',
  ],
  'tip.snd.volume.sys': [
    'Volume di tutti i suoni di Aurora, senza toccare il volume del sistema.',
    'Volume of all of Aurora’s sounds, without touching the system volume.',
    'Lautstärke aller Aurora-Töne, ohne die Lautstärke des Systems zu ändern.',
    'Volume de tous les sons d’Aurora, sans toucher au volume du système.',
    'Volumen de todos los sonidos de Aurora, sin tocar el volumen del sistema.',
    'Volume de todos os sons da Aurora, sem mexer no volume do sistema.',
  ],
  'ens.audio.help.sys': [
    'Ingresso e uscita si scelgono qui e restano al riavvio. Il livello dell’ingresso si regola dal mixer audio del sistema.',
    'Input and output are chosen here and are kept after a restart. The input level is set from the system audio mixer.',
    'Eingang und Ausgang werden hier gewählt und bleiben nach einem Neustart erhalten. Der Eingangspegel wird im Audiomixer des Systems eingestellt.',
    'L’entrée et la sortie se choisissent ici et sont conservées au redémarrage. Le niveau d’entrée se règle dans le mélangeur audio du système.',
    'La entrada y la salida se eligen aquí y se conservan al reiniciar. El nivel de entrada se ajusta en el mezclador de audio del sistema.',
    'A entrada e a saída escolhem-se aqui e mantêm-se ao reiniciar. O nível de entrada regula-se no misturador de áudio do sistema.',
  ],
  'aud.vol.in.sys': ['Volume di registrazione (sistema)', 'Recording volume (system)', 'Aufnahmelautstärke (System)', 'Volume d’enregistrement (système)', 'Volumen de grabación (sistema)', 'Volume de gravação (sistema)'],
  'aud.vol.out.sys': ['Livello di trasmissione (Pwr)', 'Transmit level (Pwr)', 'Sendepegel (Pwr)', 'Niveau d’émission (Pwr)', 'Nivel de transmisión (Pwr)', 'Nível de transmissão (Pwr)'],
  'tip.aud.vol.in.sys': [
    'Il volume di registrazione di questo ingresso nel sistema: lo stesso che si vede nel mixer audio del sistema.',
    'The recording volume of this input in the system, the same as in the system audio mixer.',
    'Die Aufnahmelautstärke dieses Eingangs im System, wie im Audiomixer des Systems.',
    'Le volume d’enregistrement de cette entrée dans le système, le même que dans le mélangeur audio du système.',
    'El volumen de grabación de esta entrada en el sistema, el mismo que en el mezclador de audio del sistema.',
    'O volume de gravação desta entrada no sistema, o mesmo do misturador de áudio do sistema.',
  ],
  'tip.aud.vol.out.sys': [
    'Il livello Pwr della trasmissione (lo stesso cursore del pannello Trasmissione): abbassa il segnale che Aurora genera. Il volume dell’uscita si regola una volta nel mixer audio del sistema.',
    'The transmit Pwr level (the same slider as in the Transmit panel): it lowers the signal Aurora generates. The output volume is set once in the system audio mixer.',
    'Der Sendepegel Pwr (derselbe Regler wie im Feld Senden): Er senkt das von Aurora erzeugte Signal. Die Lautstärke des Ausgangs wird einmal im Audiomixer des Systems eingestellt.',
    'Le niveau Pwr d’émission (le même curseur que dans le panneau Émission) : il baisse le signal qu’Aurora génère. Le volume de la sortie se règle une fois dans le mélangeur audio du système.',
    'El nivel Pwr de transmisión (el mismo control del panel Transmisión): baja la señal que genera Aurora. El volumen de la salida se ajusta una vez en el mezclador de audio del sistema.',
    'O nível Pwr de transmissão (o mesmo controlo do painel Transmissão): baixa o sinal que a Aurora gera. O volume da saída regula-se uma vez no misturador de áudio do sistema.',
  ],
  'aud.vol.off.sys': [
    'Il volume di questo dispositivo si regola dal mixer audio del sistema.',
    'The volume of this device is set from the system audio mixer.',
    'Die Lautstärke dieses Geräts wird im Audiomixer des Systems eingestellt.',
    'Le volume de ce périphérique se règle dans le mélangeur audio du système.',
    'El volumen de este dispositivo se ajusta en el mezclador de audio del sistema.',
    'O volume deste dispositivo regula-se no misturador de áudio do sistema.',
  ],
  'tip.aud.input.sys': [
    'Il dispositivo da cui arrivano cascata, decodifica e livello d’ingresso. «Predefinito del sistema» segue quello scelto nelle impostazioni audio del sistema.',
    'The device that feeds the waterfall, the decoder and the input level. “System default” follows the one chosen in the system sound settings.',
    'Das Gerät für Wasserfall, Dekodierung und Eingangspegel. „Systemstandard“ folgt der Wahl in den Soundeinstellungen des Systems.',
    'Le périphérique qui alimente la cascade, le décodage et le niveau d’entrée. « Par défaut du système » suit celui choisi dans les réglages son du système.',
    'El dispositivo que alimenta la cascada, la decodificación y el nivel de entrada. «Predeterminado del sistema» sigue el elegido en la configuración de sonido del sistema.',
    'O dispositivo que alimenta a cascata, a descodificação e o nível de entrada. «Predefinição do sistema» segue o escolhido nas definições de som do sistema.',
  ],
} as const satisfies Record<string, Entry>;
