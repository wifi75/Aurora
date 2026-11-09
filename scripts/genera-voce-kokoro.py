"""Genera le frasi della voce degli avvisi (app/public/voice) con Kokoro, voce libera (licenza Apache 2.0): si possono distribuire.

Kokoro non ha il tedesco: per `de` si usano le frasi inglesi. Il nominativo si dice con l'alfabeto fonetico in inglese (`voice/nato`).
Uso: Python 3.11 con `pip install kokoro-onnx soundfile`, e i file del modello (kokoro-v1.0.onnx, voices-v1.0.bin) dalla release
«model-files-v1.0» di github.com/thewh1teagle/kokoro-onnx in una cartella a scelta:
    python scripts/genera-voce-kokoro.py <cartella del modello>
Le frasi stanno in `genera-voce.py` (le stesse di sempre). Esce in OGG Vorbis, piccolo e letto da WebView2.
"""

import importlib.util
import os
import sys

import soundfile as sf
from kokoro_onnx import Kokoro

QUI = os.path.dirname(os.path.abspath(__file__))
OUT = os.path.join(QUI, '..', 'app', 'public', 'voice')

spec = importlib.util.spec_from_file_location('frasi', os.path.join(QUI, 'genera-voce.py'))
frasi = importlib.util.module_from_spec(spec)
spec.loader.exec_module(frasi)

# voce femminile per ogni lingua (scelta del proprietario, 2026-11-09)
VOCI = {'it': ('if_sara', 'it'), 'en': ('af_heart', 'en-us'), 'fr': ('ff_siwis', 'fr-fr'), 'es': ('ef_dora', 'es'), 'pt': ('pf_dora', 'pt-br'), 'de': ('af_heart', 'en-us')}


def scrivi(k: Kokoro, testo: str, voce: str, lingua: str, dove: str) -> None:
    samples, rate = k.create(testo, voice=voce, speed=1.05, lang=lingua)
    os.makedirs(os.path.dirname(dove), exist_ok=True)
    sf.write(dove, samples, rate, format='OGG', subtype='VORBIS')


def main() -> int:
    if len(sys.argv) < 2:
        print('uso: python scripts/genera-voce-kokoro.py <cartella con kokoro-v1.0.onnx e voices-v1.0.bin>')
        return 1
    m = sys.argv[1]
    k = Kokoro(os.path.join(m, 'kokoro-v1.0.onnx'), os.path.join(m, 'voices-v1.0.bin'))
    n = 0
    for i, sigla in enumerate(frasi.ORDINE):
        voce, lingua = VOCI[sigla]
        idx = frasi.ORDINE.index('en') if sigla == 'de' else i
        for evento, testi in frasi.FRASI.items():
            scrivi(k, testi[idx], voce, lingua, os.path.join(OUT, sigla, f'{evento}.ogg'))
            n += 1
    for nome, parola in frasi.NATO.items():
        scrivi(k, parola + '.', 'af_heart', 'en-us', os.path.join(OUT, 'nato', f'{nome}.ogg'))
        n += 1
    print(f'{n} file generati con Kokoro in {os.path.normpath(OUT)}')
    return 0


if __name__ == '__main__':
    sys.exit(main())
