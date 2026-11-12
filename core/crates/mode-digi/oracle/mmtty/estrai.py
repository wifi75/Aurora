# Estrae da MMTTY 1.70L le parti del ricevitore in un solo file C++ (oracle/mmtty/mmtty_dsp.inc).
# Uso, dalla radice del repository: python core/crates/mode-digi/oracle/mmtty/estrai.py terze-parti/mmtty core/crates/mode-digi/oracle/mmtty/mmtty_dsp.inc
import sys, pathlib
src = pathlib.Path(sys.argv[1])
out = pathlib.Path(sys.argv[2])
def lines(name):
    return src.joinpath(name).read_bytes().decode('cp932', errors='replace').replace('\r\n', '\n').split('\n')
parts = [
    ('CLX.h', 26, 180),       # classe CLX
    ('CLX.cpp', 26, 81),
    ('fir.h', 27, 319),       # dichiarazioni dei filtri (senza ComLib.h, inifiles.hpp e DrawGraph)
    ('Rtty.h', 40, 756),      # CFAVG ... CFSKDEM
    ('fir.cpp', 29, 75),      # DoFIR, CIIRTANK
    ('fir.cpp', 261, 315),    # CDECM2
    ('fir.cpp', 605, 775),    # I0, MakeFilter, MakeHilbert
    ('fir.cpp', 1402, 1520),  # asinh, MakeIIR, CIIR
    ('fir.cpp', 1825, 2104),  # CDECM4, CINTP4, COVERLIMIT
    ('Rtty.cpp', 26, 240),    # CSinTable, CTICK, CVCO, CVCOX
    ('Rtty.cpp', 630, 1433),  # CFSKDEM
    ('Rtty.cpp', 1462, 1474), # _LTR, _FIG
    ('Rtty.cpp', 1651, 1700), # CScope
    ('Rtty.cpp', 1725, 2624), # CSamplePeak, CAGC, CATC, CFIR, CFIR2, CFIRX, CSlideFFT, CPHASE, CFAVG, CAGCX, CAMPCONT, CAA6YQ
]
buf = ['// Estratto da MMTTY 1.70L (terze-parti/mmtty, commit e2c1327) per il solo arbitro delle prove di Aurora: vedi mmtty_rtty_rx.cpp.']
buf.extend(lines('Rtty.cpp')[0:17])
for name, a, b in parts:
    L = lines(name)
    buf.append(f'// ===== {name}, righe {a}-{b} =====')
    buf.extend(L[a-1:b])
text = '\n'.join(buf) + '\n'
# l'unica riga che il C++ standard non accetta (Borland: tipo di ritorno sottinteso)
text = text.replace('inline __fastcall IsMax(void)', 'inline int __fastcall IsMax(void)')
skip_prefixes = ('void DrawGraph',)
text = '\n'.join(l for l in text.split('\n') if not l.startswith(skip_prefixes)) + '\n'
out.write_bytes(text.encode('utf-8'))
print(len(text.split('\n')))
