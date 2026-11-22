// Il codificatore di Olivia e Contestia di fldigi 4.2.13, usato come arbitro nelle prove di Aurora («oracolo»): si compila solo con la feature
// `fldigi-oracle` del crate mode-digi (build.rs) e non entra mai nel programma.
//
// Il codice vero e' la libreria header-only di Pawel Jalocha che fldigi porta in `terze-parti/fldigi/src/include/jalocha/pj_mfsk.h`
// (MFSK_Encoder); qui c'e' solo l'interfaccia C. Licenza GPL 3 o successiva, come Aurora.

#include <cstddef>
#include <cstdint>

#include "pj_mfsk.h"

extern "C" {

// Codifica un blocco FEC: `bits_per_symbol` caratteri in ingresso (uno per ogni bit del simbolo), `symbols_per_block` simboli in uscita.
// Restituisce il numero di simboli scritti in `out` (0 se `out_cap` non basta o i parametri sono fuori campo).
size_t aurora_fldigi_olivia_encode_block(int bits_per_symbol, int contestia, const uint8_t *chars, uint8_t *out, size_t out_cap) {
	if (bits_per_symbol < 1 || bits_per_symbol > 8) return 0;
	MFSK_Encoder enc;
	enc.bContestia = contestia != 0;
	enc.BitsPerSymbol = (size_t)bits_per_symbol;
	if (enc.Preset() < 0) return 0;
	if (out_cap < enc.SymbolsPerBlock) return 0;
	uint8_t block[8] = {0};
	for (int i = 0; i < bits_per_symbol; i++) block[i] = chars[i];
	enc.EncodeBlock(block);
	for (size_t i = 0; i < enc.SymbolsPerBlock; i++) out[i] = enc.OutputBlock[i];
	return enc.SymbolsPerBlock;
}

}
