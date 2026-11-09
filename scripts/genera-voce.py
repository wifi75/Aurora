"""Genera le frasi della voce degli avvisi (app/public/voice) con Azure AI Speech, voce Ava Multilingual.

Le Product Terms di Microsoft permettono di usare l'audio delle voci prebuilt anche a scopo commerciale solo con una risorsa Speech
a PAGAMENTO (livello S0): per questo i file si generano da qui, con la chiave del proprietario, e non con servizi gratuiti.

Uso (PowerShell, dalla cartella del repository):
    $env:AZURE_SPEECH_KEY = '<chiave della risorsa Speech>'
    $env:AZURE_SPEECH_REGION = 'westeurope'
    python scripts/genera-voce.py
La chiave non va mai nei file ne' nei commit. Rigenera tutto (sovrascrive i file esistenti).
"""

import os
import sys
import time
import urllib.error
import urllib.request
from xml.sax.saxutils import escape

VOCE = 'en-US-AvaMultilingualNeural'
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'app', 'public', 'voice')
LINGUE = {'it': 'it-IT', 'en': 'en-US', 'de': 'de-DE', 'fr': 'fr-FR', 'es': 'es-ES', 'pt': 'pt-PT'}
ORDINE = ['it', 'en', 'de', 'fr', 'es', 'pt']

FRASI = {
    'tx_msg': ['Trasmetto', 'Transmitting', 'Sende', 'J’émets', 'Transmitiendo', 'A transmitir'],
    'cq': ['CQ', 'CQ', 'CQ', 'CQ', 'CQ', 'CQ'],
    'my_call': ['Ti chiamano', 'Calling you', 'Du wirst gerufen', 'On t’appelle', 'Te llaman', 'Estão a chamar-te'],
    'standard_call': ['Chiamata', 'Call', 'Anruf', 'Appel', 'Llamada', 'Chamada'],
    'new_zone': ['Nuova zona CQ', 'New CQ zone', 'Neue CQ-Zone', 'Nouvelle zone CQ', 'Nueva zona CQ', 'Nova zona CQ'],
    'new_zone_band': ['Nuova zona CQ sulla banda', 'New CQ zone on band', 'Neue CQ-Zone auf dem Band', 'Nouvelle zone CQ sur la bande', 'Nueva zona CQ en la banda', 'Nova zona CQ na banda'],
    'new_itu': ['Nuova zona ITU', 'New ITU zone', 'Neue ITU-Zone', 'Nouvelle zone UIT', 'Nueva zona UIT', 'Nova zona UIT'],
    'new_itu_band': ['Nuova zona ITU sulla banda', 'New ITU zone on band', 'Neue ITU-Zone auf dem Band', 'Nouvelle zone UIT sur la bande', 'Nueva zona UIT en la banda', 'Nova zona UIT na banda'],
    'new_dxcc': ['Nuovo DXCC!', 'New DXCC!', 'Neues DXCC!', 'Nouveau DXCC !', '¡Nuevo DXCC!', 'Novo DXCC!'],
    'new_dxcc_band': ['Nuovo DXCC sulla banda', 'New DXCC on band', 'Neues DXCC auf dem Band', 'Nouveau DXCC sur la bande', 'Nuevo DXCC en la banda', 'Novo DXCC na banda'],
    'new_grid': ['Nuova griglia', 'New grid', 'Neues Locator-Feld', 'Nouveau carré', 'Nueva cuadrícula', 'Nova quadrícula'],
    'new_grid_band': ['Nuova griglia sulla banda', 'New grid on band', 'Neues Locator-Feld auf dem Band', 'Nouveau carré sur la bande', 'Nueva cuadrícula en la banda', 'Nova quadrícula na banda'],
    'new_px': ['Nuovo prefisso', 'New prefix', 'Neuer Präfix', 'Nouveau préfixe', 'Nuevo prefijo', 'Novo prefixo'],
    'new_px_band': ['Nuovo prefisso sulla banda', 'New prefix on band', 'Neuer Präfix auf dem Band', 'Nouveau préfixe sur la bande', 'Nuevo prefijo en la banda', 'Novo prefixo na banda'],
    'new_call': ['Nuova stazione', 'New station', 'Neue Station', 'Nouvelle station', 'Nueva estación', 'Nova estação'],
    'new_call_band': ['Nuova stazione sulla banda', 'New station on band', 'Neue Station auf dem Band', 'Nouvelle station sur la bande', 'Nueva estación en la banda', 'Nova estação na banda'],
    'worked_call': ['Già lavorata', 'Already worked', 'Schon gearbeitet', 'Déjà contactée', 'Ya trabajada', 'Já trabalhada'],
    'alert_my_call': ['Ti chiamano', 'Calling you', 'Du wirst gerufen', 'On t’appelle', 'Te llaman', 'Estão a chamar-te'],
    'alert_dx_call': ['Il tuo DX', 'Your DX', 'Dein DX', 'Ton DX', 'Tu DX', 'O teu DX'],
    'first_message': ['Primo messaggio', 'First message', 'Erste Nachricht', 'Premier message', 'Primer mensaje', 'Primeira mensagem'],
    'wanted': ['Stazione desiderata!', 'Wanted station!', 'Gesuchte Station!', 'Station recherchée !', '¡Estación deseada!', 'Estação desejada!'],
    'new_dx_popup': ['Attenzione: nuovo DXCC!', 'Heads up: wanted DXCC!', 'Achtung: neues DXCC!', 'Attention : nouveau DXCC !', '¡Atención: nuevo DXCC!', 'Atenção: novo DXCC!'],
}

NATO = dict(zip('ABCDEFGHIJKLMNOPQRSTUVWXYZ', 'Alfa Bravo Charlie Delta Echo Foxtrot Golf Hotel India Juliett Kilo Lima Mike November Oscar Papa Quebec Romeo Sierra Tango Uniform Victor Whiskey X-ray Yankee Zulu'.split()))
NATO.update(dict(zip('0123456789', 'Zero One Two Three Four Five Six Seven Eight Nine'.split())))
NATO['slash'] = 'Stroke'


def sintetizza(chiave: str, regione: str, testo: str, lingua: str, dove: str) -> None:
    ssml = (
        f"<speak version='1.0' xmlns='http://www.w3.org/2001/10/synthesis' xml:lang='en-US'>"
        f"<voice name='{VOCE}'><lang xml:lang='{lingua}'><prosody rate='+8%'>{escape(testo)}</prosody></lang></voice></speak>"
    )
    req = urllib.request.Request(
        f'https://{regione}.tts.speech.microsoft.com/cognitiveservices/v1',
        data=ssml.encode('utf-8'),
        headers={
            'Ocp-Apim-Subscription-Key': chiave,
            'Content-Type': 'application/ssml+xml',
            'X-Microsoft-OutputFormat': 'audio-24khz-48kbitrate-mono-mp3',
            'User-Agent': 'Aurora-genera-voce',
        },
    )
    for tentativo in range(4):
        try:
            with urllib.request.urlopen(req, timeout=30) as r:
                dati = r.read()
            os.makedirs(os.path.dirname(dove), exist_ok=True)
            with open(dove, 'wb') as f:
                f.write(dati)
            return
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 503) and tentativo < 3:
                time.sleep(2 * (tentativo + 1))
                continue
            raise


def main() -> int:
    chiave = os.environ.get('AZURE_SPEECH_KEY', '').strip()
    regione = os.environ.get('AZURE_SPEECH_REGION', '').strip()
    if not chiave or not regione:
        print('Imposta AZURE_SPEECH_KEY e AZURE_SPEECH_REGION (risorsa Speech a pagamento, livello S0).')
        return 1
    n = 0
    for i, sigla in enumerate(ORDINE):
        for evento, testi in FRASI.items():
            sintetizza(chiave, regione, testi[i], LINGUE[sigla], os.path.join(OUT, sigla, f'{evento}.mp3'))
            n += 1
    for nome, parola in NATO.items():
        sintetizza(chiave, regione, parola + '.', 'en-US', os.path.join(OUT, 'nato', f'{nome}.mp3'))
        n += 1
    print(f'{n} file generati con Azure AI Speech ({VOCE}) in {os.path.normpath(OUT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
