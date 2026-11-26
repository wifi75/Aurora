// MT63 di fldigi 4.2.13 (trasmettitore e ricevitore), per le prove di Aurora («oracolo»): si compila solo con la feature
// `fldigi-oracle` del crate mode-digi (build.rs) e non entra mai nel programma.
//
// I sorgenti di fldigi, copia in terze-parti/fldigi, si compilano cosi' come sono (build.rs): src/mt63/mt63base.cxx (MT63tx, MT63rx,
// codificatore e decodificatore, con symbol.dat e mt63intl.dat) e src/mt63/dsp.cxx (FFT, filtri, finestre). Autori: Pawel Jalocha SP9VRC
// e Dave Freese W1HKJ; licenza GPL 3 o successiva, come Aurora.
//
// Qui sotto solo l'uso che ne fa mt63::tx_process e mt63::rx_process (src/mt63/mt63.cxx): toni iniziali spenti, 7 bit, uscita grezza del
// trasmettitore (senza la normalizzazione sul massimo). I numeri a caso del trasmettitore (riempimento dell'interleaving e simbolo di
// disturbo) vengono da rand(): si registrano con srand() e si restituiscono, perche' la prova li dia anche al nostro.

#include <cmath>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "dsp.h"
#include "mt63base.h"

namespace {

struct FldigiMt63Rx {
	MT63rx rx;
	double_buff inp;
	std::string out;
	double snr;
};

} // namespace

extern "C" {

// bandwidth 500/1000/2000, long_intlv 0/1, integ 16 o 32
void *aurora_fldigi_mt63_new(int bandwidth, int long_intlv, double freq, int integ) {
	FldigiMt63Rx *r = new FldigiMt63Rx();
	if (r->rx.Preset(freq, bandwidth, long_intlv, integ) != 0) {
		delete r;
		return nullptr;
	}
	r->snr = 0.0;
	return r;
}

void aurora_fldigi_mt63_process(void *p, const float *samples, size_t len) {
	if (!p || !samples) return;
	FldigiMt63Rx *r = static_cast<FldigiMt63Rx *>(p);
	r->inp.EnsureSpace((int)len);
	for (size_t i = 0; i < len; i++) r->inp.Data[i] = samples[i];
	r->inp.Len = (int)len;
	r->rx.Process(&r->inp);
	r->snr = r->rx.FEC_SNR();
	for (int i = 0; i < r->rx.Output.Len; i++) {
		unsigned char c = (unsigned char)r->rx.Output.Data[i];
		// come fldigi con gli 8 bit: sotto 8 non si stampa
		if (c >= 8 && c < 127) r->out.push_back((char)c);
	}
}

size_t aurora_fldigi_mt63_take(void *p, char *buf, size_t cap) {
	if (!p || !buf) return 0;
	std::string &out = static_cast<FldigiMt63Rx *>(p)->out;
	size_t n = out.size() < cap ? out.size() : cap;
	memcpy(buf, out.data(), n);
	out.erase(0, n);
	return n;
}

double aurora_fldigi_mt63_snr(void *p) {
	return p ? static_cast<FldigiMt63Rx *>(p)->snr : 0.0;
}

void aurora_fldigi_mt63_free(void *p) {
	delete static_cast<FldigiMt63Rx *>(p);
}

// Il trasmettitore di fldigi come in tx_process: interleaving riempito di NUL in silenzio, il testo, interleave-1 NUL, il disturbo.
// Scrive fino a `cap` campioni in `out` e restituisce quanti ne servono in tutto; `fill` (interleave*64 byte) e `jam` (64 byte) ricevono
// i numeri a caso usati.
size_t aurora_fldigi_mt63_tx(int bandwidth, int long_intlv, double freq, const char *text, float *out, size_t cap, unsigned char *fill,
                             unsigned char *jam) {
	if (!text) return 0;
	MT63tx tx;
	int intlv = long_intlv ? 64 : 32;
	srand(12345);
	for (int i = 0; i < intlv * 64; i++) {
		int b = rand() & 1;
		if (fill) fill[i] = (unsigned char)b;
	}
	srand(12345);
	if (tx.Preset(freq, bandwidth, long_intlv) != 0) return 0;
	std::vector<float> wave;
	for (int i = 0; i < tx.DataInterleave; i++) tx.SendChar(0);
	auto take = [&]() {
		for (int i = 0; i < tx.Comb.Output.Len; i++) wave.push_back((float)tx.Comb.Output.Data[i]);
	};
	for (const char *q = text; *q; q++) {
		int c = (unsigned char)*q;
		if (c > 127) c = '.';
		tx.SendChar((char)c);
		take();
	}
	int flush = tx.DataInterleave;
	while (--flush) {
		tx.SendChar(0);
		take();
	}
	srand(777);
	for (int i = 0; i < 64; i++) {
		int b = (rand() & 0x100) ? 1 : 0;
		if (jam) jam[i] = (unsigned char)b;
	}
	srand(777);
	tx.SendJam();
	take();
	if (out) memcpy(out, wave.data(), (wave.size() < cap ? wave.size() : cap) * sizeof(float));
	return wave.size();
}

}
