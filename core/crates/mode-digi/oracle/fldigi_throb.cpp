// Throb e ThrobX di fldigi 4.2.13 (ricevitore e trasmettitore), estratti per le prove di Aurora («oracolo»): si compila solo con la
// feature `fldigi-oracle` del crate mode-digi (build.rs) e non entra mai nel programma.
//
// Il codice qui sotto e' ripreso da fldigi, copia in terze-parti/fldigi:
//   src/throb/throb.cxx     throb::throb (parametri delle sei varianti), mk_rxtone, mixer, findtones, decodechar, rx, sync, rx_process,
//                           mk_semi_pulse, mk_full_pulse, send, tx_process, flip_syms, reset_syms e le tabelle;
//   src/filters/filters.cxx C_FIR_filter (Hilbert e filtro di sincronismo) e Cmovavg, compilati cosi' come sono;
//   src/filters/fftfilt.cxx fftfilt (passabanda complesso), compilato cosi' com'e';
//   src/include/throb.h, filters.h, fftfilt.h, complex.h.
// Autori: Dave Freese W1HKJ (ThrobX di Joe Veldhuis KD8ATU), da gmfsk di Tomi Manninen OH2BNS; licenza GPL 3 o successiva, come Aurora.
//
// Tolti perche' non servono a decidere i caratteri: oscilloscopio, S/N a schermo, FLTK. Squelch e AFC spenti, minuscole spente.
// La trasmissione e' quella di tx_process: quattro simboli di attesa, i caratteri, un simbolo di attesa alla fine.

#include <cctype>
#include <cmath>
#include <cstddef>
#include <cstring>
#include <string>
#include <vector>

#include "complex.h"
#include "fftfilt.h"
#include "filters.h"

namespace {

const double THROB_SAMPLE_RATE = 8000.0;
const int DOWN_SAMPLE = 32;
const int SYMLEN_1 = 8192, SYMLEN_2 = 4096, SYMLEN_4 = 2048;
const int MAX_RX_SYMLEN = SYMLEN_1 / DOWN_SAMPLE;
const int FilterFFTLen = 8192;
const int MAX_TONES = 15;

int ThrobTonePairs[45][2] = {
	{5, 5}, {4, 5}, {1, 2}, {1, 3}, {1, 4}, {4, 6}, {1, 5}, {1, 6}, {1, 7}, {3, 7}, {1, 8}, {2, 3}, {2, 4}, {2, 8}, {2, 5},
	{5, 6}, {2, 6}, {2, 9}, {3, 4}, {3, 5}, {1, 9}, {3, 6}, {8, 9}, {3, 8}, {3, 3}, {2, 2}, {1, 1}, {3, 9}, {4, 7}, {4, 8},
	{4, 9}, {5, 7}, {5, 8}, {5, 9}, {6, 7}, {6, 8}, {6, 9}, {7, 8}, {7, 9}, {8, 8}, {7, 7}, {6, 6}, {4, 4}, {9, 9}, {2, 7}};
int ThrobXTonePairs[55][2] = {
	{6, 11}, {1, 6}, {2, 6}, {2, 5}, {2, 7}, {2, 8}, {5, 6}, {2, 9}, {2, 10}, {4, 8}, {4, 6}, {2, 11}, {3, 4}, {3, 5}, {3, 6},
	{6, 9}, {6, 10}, {3, 7}, {3, 8}, {3, 9}, {6, 8}, {6, 7}, {3, 10}, {3, 11}, {4, 5}, {4, 7}, {4, 9}, {4, 10}, {1, 2}, {1, 3},
	{1, 4}, {1, 5}, {1, 7}, {1, 8}, {1, 9}, {1, 10}, {2, 3}, {2, 4}, {4, 11}, {5, 7}, {5, 8}, {5, 9}, {5, 10}, {5, 11}, {7, 8},
	{7, 9}, {7, 10}, {7, 11}, {8, 9}, {8, 10}, {8, 11}, {9, 10}, {9, 11}, {10, 11}, {1, 11}};
const unsigned char ThrobCharSet[45] = {'\0', 'A', 'B', 'C', 'D', '\0', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q',
	'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', ',', '.', '\'', '/', ')', '(', 'E', ' '};
const unsigned char ThrobXCharSet[55] = {'\0', ' ', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P',
	'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', ',', '.', '\'', '/', ')', '(',
	'#', '"', '+', '-', ';', ':', '?', '!', '@', '=', '\n'};
const double ThrobToneFreqsNar[9] = {-32, -24, -16, -8, 0, 8, 16, 24, 32};
const double ThrobToneFreqsWid[9] = {-64, -48, -32, -16, 0, 16, 32, 48, 64};
const double ThrobXToneFreqsNar[11] = {-39.0625, -31.25, -23.4375, -15.625, -7.8125, 0, 7.8125, 15.625, 23.4375, 31.25, 39.0625};
const double ThrobXToneFreqsWid[11] = {-78.125, -62.5, -46.875, -31.25, -15.625, 0, 15.625, 31.25, 46.875, 62.5, 78.125};

double *mk_semi_pulse(int len) {
	double *pulse = new double[len];
	for (int i = 0; i < len; i++) {
		if (i < len / 5) pulse[i] = 0.5 * (1 - cos(M_PI * i / (len / 5.0)));
		if (i >= len / 5 && i < len * 4 / 5) pulse[i] = 1.0;
		if (i >= len * 4 / 5) pulse[i] = 0.5 * (1 + cos(M_PI * (i - len * 4 / 5) / (len / 5.0)));
	}
	return pulse;
}

double *mk_full_pulse(int len) {
	double *pulse = new double[len];
	for (int i = 0; i < len; i++) pulse[i] = 0.5 * (1 - cos(2 * M_PI * i / len));
	return pulse;
}

// variante: 0 Throb1, 1 Throb2, 2 Throb4, 3 ThrobX1, 4 ThrobX2, 5 ThrobX4 (come ThrobVariant::ALL)
struct FldigiThrob {
	int variant;
	bool x;
	int num_tones, num_chars, idlesym, spacesym, symlen, rxsymlen;
	double freqs[11];
	double *txpulse;
	double frequency;
	int reverse;
	// ricezione
	C_FIR_filter *hilbert, *syncfilt;
	fftfilt *fftfilter;
	Cmovavg *snfilter;
	cmplx *rxtone[11];
	cmplx symbol[MAX_RX_SYMLEN];
	double syncbuf[MAX_RX_SYMLEN], dispbuf[MAX_RX_SYMLEN];
	double rxcntr, phaseacc, metric;
	int symptr, deccntr, shift, waitsync;
	char lastchar;
	std::string out;

	FldigiThrob(int v, double freq, int rev) : variant(v), frequency(freq), reverse(rev) {
		x = v >= 3;
		double bw;
		double *fp;
		const double *tones;
		symlen = (v % 3 == 0) ? SYMLEN_1 : (v % 3 == 1) ? SYMLEN_2 : SYMLEN_4;
		if (v % 3 == 2) {
			txpulse = mk_full_pulse(symlen);
			fp = mk_full_pulse(symlen / DOWN_SAMPLE);
		} else {
			txpulse = mk_semi_pulse(symlen);
			fp = mk_semi_pulse(symlen / DOWN_SAMPLE);
		}
		if (!x) {
			num_tones = 9;
			num_chars = 45;
			tones = v == 2 ? ThrobToneFreqsWid : ThrobToneFreqsNar;
			bw = (v == 2 ? 72.0 : 36.0) / THROB_SAMPLE_RATE;
		} else {
			num_tones = 11;
			num_chars = 55;
			tones = v == 5 ? ThrobXToneFreqsWid : ThrobXToneFreqsNar;
			bw = (v == 5 ? 94.0 : 47.0) / THROB_SAMPLE_RATE;
		}
		for (int i = 0; i < num_tones; i++) freqs[i] = tones[i];
		reset_syms();
		rxsymlen = symlen / DOWN_SAMPLE;
		hilbert = new C_FIR_filter();
		hilbert->init_hilbert(37, 1);
		fftfilter = new fftfilt(0, bw, FilterFFTLen);
		syncfilt = new C_FIR_filter();
		syncfilt->init(symlen / DOWN_SAMPLE, 1, fp, NULL);
		delete[] fp;
		snfilter = new Cmovavg(16);
		for (int i = 0; i < num_tones; i++) rxtone[i] = mk_rxtone(freqs[i], txpulse, symlen);
		phaseacc = 0.0;
		metric = 0.0;
		for (int i = 0; i < MAX_RX_SYMLEN; i++) syncbuf[i] = dispbuf[i] = 0.0;
		// rx_init
		rxcntr = rxsymlen;
		waitsync = 1;
		deccntr = 0;
		symptr = 0;
		shift = 0;
		lastchar = '\0';
	}

	~FldigiThrob() {
		delete hilbert;
		delete syncfilt;
		delete fftfilter;
		delete snfilter;
		delete[] txpulse;
		for (int i = 0; i < num_tones; i++) delete[] rxtone[i];
	}

	void flip_syms() {
		if (!x) return;
		if (idlesym == 0) {
			idlesym = 1;
			spacesym = 0;
		} else {
			idlesym = 0;
			spacesym = 1;
		}
	}

	void reset_syms() {
		idlesym = 0;
		spacesym = x ? 1 : 44;
	}

	cmplx *mk_rxtone(double freq, double *pulse, int len) {
		cmplx *tone = new cmplx[len / DOWN_SAMPLE];
		for (int i = 0; i < len; i += DOWN_SAMPLE) {
			double xx = -2.0 * M_PI * freq * i / THROB_SAMPLE_RATE;
			tone[i / DOWN_SAMPLE] = cmplx(pulse[i] * cos(xx), pulse[i] * sin(xx));
		}
		return tone;
	}

	cmplx mixer(cmplx in) {
		cmplx z(cos(phaseacc), sin(phaseacc));
		z = z * in;
		phaseacc -= 2.0 * M_PI * frequency / THROB_SAMPLE_RATE;
		if (phaseacc < 0) phaseacc += 2.0 * M_PI;
		return z;
	}

	int findtones(cmplx *word, int &tone1, int &tone2) {
		double max1 = 0, max2 = 0;
		tone1 = 0;
		for (int i = 0; i < num_tones; i++)
			if (abs(word[i]) > max1) {
				max1 = abs(word[i]);
				tone1 = i;
			}
		int maxtone = tone1;
		tone2 = 0;
		for (int i = 0; i < num_tones; i++) {
			if (i == tone1) continue;
			if (abs(word[i]) > max2) {
				max2 = abs(word[i]);
				tone2 = i;
			}
		}
		if (!x && max1 > max2 * 2) tone2 = tone1;
		if (tone1 > tone2) {
			int i = tone1;
			tone1 = tone2;
			tone2 = i;
		}
		double signal = 0, noise = 0;
		for (int i = 0; i < num_tones; i++) {
			if (i == tone1 || i == tone2) signal += abs(word[i]) / 2.0;
			else noise += abs(word[i]) / (num_tones - 2.0);
		}
		metric = snfilter->run(signal / (noise + 1e-6));
		return maxtone;
	}

	void show_char(int c) { out.push_back((char)c); }

	void decodechar(int tone1, int tone2) {
		if (!x) {
			if (shift) {
				if (tone1 == 0 && tone2 == 8) show_char('?');
				if (tone1 == 1 && tone2 == 7) show_char('@');
				if (tone1 == 2 && tone2 == 6) show_char('=');
				if (tone1 == 4 && tone2 == 4) show_char('\n');
				shift = 0;
				return;
			}
			if (tone1 == 3 && tone2 == 5) {
				shift = 1;
				return;
			}
			for (int i = 0; i < num_chars; i++)
				if (ThrobTonePairs[i][0] == tone1 + 1 && ThrobTonePairs[i][1] == tone2 + 1) {
					show_char(ThrobCharSet[i]);
					break;
				}
			return;
		}
		for (int i = 0; i < num_chars; i++) {
			if (ThrobXTonePairs[i][0] == tone1 + 1 && ThrobXTonePairs[i][1] == tone2 + 1) {
				if (i == spacesym || i == idlesym) {
					if (lastchar != '\0' && lastchar != ' ') {
						show_char(ThrobXCharSet[1]);
						lastchar = ' ';
					} else
						lastchar = '\0';
					flip_syms();
				} else {
					show_char(ThrobXCharSet[i]);
					lastchar = ThrobXCharSet[i];
				}
			}
		}
	}

	void rx(cmplx in) {
		cmplx rxword[MAX_TONES];
		int tone1, tone2;
		symbol[symptr] = in;
		if (rxcntr > 0.0) return;
		for (int i = 0; i < num_tones; i++) rxword[i] = cmac(rxtone[i], symbol, symptr + 1, rxsymlen);
		findtones(rxword, tone1, tone2);
		if (reverse) decodechar(num_tones - 1 - tone2, num_tones - 1 - tone1);
		else decodechar(tone1, tone2);
		rxcntr = rxsymlen;
		waitsync = 1;
	}

	void sync(cmplx in) {
		double f, maxval = 0;
		int maxpos = 0;
		double mag = abs(in);
		syncfilt->Irun(mag, f);
		syncbuf[symptr] = f;
		if (waitsync == 0 || rxcntr > (rxsymlen / 2.0)) return;
		for (int i = 0; i < rxsymlen; i++) dispbuf[i] = syncbuf[(i + symptr + 1) % rxsymlen];
		for (int i = 0; i < rxsymlen; i++)
			if (dispbuf[i] > maxval) {
				maxpos = i;
				maxval = dispbuf[i];
			}
		rxcntr += (maxpos - rxsymlen / 2) / (num_tones - 1);
		waitsync = 0;
	}

	void rx_process(const float *buf, size_t len) {
		cmplx z, *zp;
		for (size_t k = 0; k < len; k++) {
			z = cmplx(buf[k], buf[k]);
			hilbert->run(z, z);
			z = mixer(z);
			int n = fftfilter->run(z, &zp);
			for (int i = 0; i < n; i++) {
				if (++deccntr >= DOWN_SAMPLE) {
					rxcntr -= 1.0;
					sync(zp[i]);
					rx(zp[i]);
					symptr = (symptr + 1) % rxsymlen;
					deccntr = 0;
				}
			}
		}
	}

	// trasmissione: throb::send con ModulateXmtr che accoda in `tx`
	void send(int sym, std::vector<float> &tx) {
		if (sym < 0 || sym >= num_chars) return;
		int tone1 = (x ? ThrobXTonePairs : ThrobTonePairs)[sym][0] - 1;
		int tone2 = (x ? ThrobXTonePairs : ThrobTonePairs)[sym][1] - 1;
		if (reverse) {
			tone1 = (num_tones - 1) - tone1;
			tone2 = (num_tones - 1) - tone2;
		}
		double w1 = 2.0 * M_PI * (frequency + freqs[tone1]) / THROB_SAMPLE_RATE;
		double w2 = 2.0 * M_PI * (frequency + freqs[tone2]) / THROB_SAMPLE_RATE;
		for (int i = 0; i < symlen; i++) tx.push_back((float)(txpulse[i] * (sin(w1 * i) + sin(w2 * i)) / 2.0));
	}

	// throb::tx_init e tx_process fino alla fine del testo
	void transmit(const char *text, std::vector<float> &tx) {
		reset_syms();
		for (int p = 0; p < 4; p++) {
			send(idlesym, tx);
			flip_syms();
		}
		for (const char *q = text; *q; q++) {
			int c = (unsigned char)*q;
			if (!x) {
				switch (c) {
				case '?': send(5, tx); send(20, tx); continue;
				case '@': send(5, tx); send(13, tx); continue;
				case '-': send(5, tx); send(9, tx); continue;
				case '\r': continue;
				case '\n': send(5, tx); send(0, tx); continue;
				default: break;
				}
			}
			if (islower(c)) c = toupper(c);
			int sym = -1;
			const unsigned char *set = x ? ThrobXCharSet : ThrobCharSet;
			for (int i = 0; i < num_chars; i++)
				if (c == set[i]) sym = i;
			if (sym == -1) c = ' ';
			if (c == ' ') {
				sym = spacesym;
				flip_syms();
			}
			send(sym, tx);
		}
		send(idlesym, tx);
	}
};

} // namespace

extern "C" {

void *aurora_fldigi_throb_new(int variant, double freq, int reverse) {
	if (variant < 0 || variant > 5) return nullptr;
	return new FldigiThrob(variant, freq, reverse);
}

void aurora_fldigi_throb_process(void *rx, const float *samples, size_t len) {
	if (rx && samples) static_cast<FldigiThrob *>(rx)->rx_process(samples, len);
}

size_t aurora_fldigi_throb_take(void *rx, char *buf, size_t cap) {
	if (!rx || !buf) return 0;
	std::string &out = static_cast<FldigiThrob *>(rx)->out;
	size_t n = out.size() < cap ? out.size() : cap;
	memcpy(buf, out.data(), n);
	out.erase(0, n);
	return n;
}

void aurora_fldigi_throb_free(void *rx) {
	delete static_cast<FldigiThrob *>(rx);
}

// Il trasmettitore di fldigi: scrive in `out` fino a `cap` campioni a 8 kHz e restituisce quanti ne servono in tutto.
size_t aurora_fldigi_throb_tx(int variant, double freq, int reverse, const char *text, float *out, size_t cap) {
	if (variant < 0 || variant > 5 || !text) return 0;
	FldigiThrob t(variant, freq, reverse);
	std::vector<float> tx;
	t.transmit(text, tx);
	if (out) memcpy(out, tx.data(), (tx.size() < cap ? tx.size() : cap) * sizeof(float));
	return tx.size();
}

}
