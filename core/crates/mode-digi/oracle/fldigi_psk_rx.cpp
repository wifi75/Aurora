// Ricevitore BPSK (PSK31, PSK63) di fldigi 4.2.13, estratto per le prove di Aurora («oracolo»): si compila solo con la feature
// `fldigi-oracle` del crate mode-digi (build.rs) e non entra mai nel programma.
//
// Il codice qui sotto e' ripreso da fldigi, copia in terze-parti/fldigi:
//   src/psk/psk.cxx         psk::psk (parametri di PSK31/PSK63, filtri PSK_CORE), psk::rx_process (mixer, fir1, fir2, sincronismo
//                           sul syncbuf), psk::rx_symbol (fase, metrica, DCD, ramo BPSK), psk::phaseafc, psk::afc, psk::rx_bit;
//   src/psk/pskcoeff.cxx    pskcore_filter e raisedcosfilt (copiati qui);
//   src/psk/pskvaricode.cxx psk_varicode_decode, compilato cosi' com'e';
//   src/filters/filters.cxx C_FIR_filter, compilato cosi' com'e';
//   src/include/filters.h, complex.h, misc.h, pskvaricode.h.
// Autori: Dave Freese W1HKJ e i contributori di fldigi (il ricevitore deriva da gmfsk di Tomi Manninen OH2BNS e dal PSK31 di Peter
// Martinez G3PLX); licenza GPL 3 o successiva, come Aurora.
//
// Tolti perche' non servono a decidere i caratteri: modi QPSK/8PSK/PSKR/multiportante, oscilloscopi, S/N e IMD, visore, FLTK. Il
// percorso dal campione al carattere e' quello di fldigi; AFC e squelch si scelgono alla creazione (di base in fldigi: AFC acceso).

#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <string>

#include "complex.h"
#include "filters.h"
#include "misc.h"
#include "pskvaricode.h"

namespace {

const double TWOPI = 2.0 * M_PI;
const int FIRLEN = 64;
const int SQLDECAY = 50;

// pskcoeff.cxx: pskcore_filter (filtro adattato di 65 coefficienti, 4 simboli a 16 campioni per simbolo)
const double pskcore_filter[FIRLEN + 1] = {
	4.3453566e-005, -0.00049122414, -0.00078771292, -0.0013507826, -0.0021287814, -0.003133466, -0.004366817, -0.0058112187,
	-0.0074249976, -0.0091398882, -0.010860157, -0.012464086, -0.013807772, -0.014731191, -0.015067057, -0.014650894,
	-0.013333425, -0.01099166, -0.0075431246, -0.0029527849, 0.0027546292, 0.0094932775, 0.017113308, 0.025403511,
	0.034099681, 0.042895839, 0.051458575, 0.059444853, 0.066521003, 0.072381617, 0.076767694, 0.079481619,
	0.080420311,
	0.079481619, 0.076767694, 0.072381617, 0.066521003, 0.059444853, 0.051458575, 0.042895839, 0.034099681,
	0.025403511, 0.017113308, 0.0094932775, 0.0027546292, -0.0029527849, -0.0075431246, -0.01099166, -0.013333425,
	-0.014650894, -0.015067057, -0.014731191, -0.013807772, -0.012464086, -0.010860157, -0.0091398882, -0.0074249976,
	-0.0058112187, -0.004366817, -0.003133466, -0.0021287814, -0.0013507826, -0.00078771292, -0.00049122414, 4.3453566e-005};

// pskcoeff.cxx: raisedcosfilt
void raisedcosfilt(double *firc, int len) {
	double k1 = M_PI * 2.0 / len;
	double k2 = 2.0 * len;
	for (int i = 0; i <= len; i++) firc[i] = (1.0 - cos(k1 * i)) / k2;
}

struct FldigiPsk {
	double samplerate;
	double frequency;
	int symbollen;
	int dcdbits;
	bool afconoff;
	bool sqlonoff;
	double sqlvalue;
	C_FIR_filter fir1, fir2;
	double phaseacc;
	double syncbuf[16];
	double bitclk;
	cmplx prevsymbol;
	cmplx quality;
	double phase;
	int bits;
	double metric, afcmetric;
	unsigned int dcdshreg;
	bool dcd;
	int dcdOFFcounter;
	int acquire;
	unsigned int shreg;
	double sc_bw;
	std::string out;

	FldigiPsk(double baud, double freq, double sr, bool afc, bool squelch, double squelch_value)
		: samplerate(sr), frequency(freq), afconoff(afc), sqlonoff(squelch), sqlvalue(squelch_value), phaseacc(0), bitclk(0),
		  prevsymbol(1.0, 0.0), quality(0.0, 0.0), phase(0), bits(0), metric(0), afcmetric(0), dcdshreg(0), dcd(false),
		  dcdOFFcounter(0), acquire(0), shreg(0) {
		// psk::psk: PSK31 256 campioni per simbolo e dcdbits 32, PSK63 128 e 64
		symbollen = baud > 40.0 ? 128 : 256;
		dcdbits = baud > 40.0 ? 64 : 32;
		sc_bw = samplerate / symbollen;
		double fir1c[FIRLEN + 1], fir2c[FIRLEN + 1];
		raisedcosfilt(fir1c, FIRLEN);
		for (int i = 0; i <= FIRLEN; i++) fir2c[i] = pskcore_filter[i];
		fir1.init(FIRLEN + 1, symbollen > 15 ? symbollen / 16 : 1, fir1c, fir1c);
		fir2.init(FIRLEN + 1, 1, fir2c, fir2c);
		for (int i = 0; i < 16; i++) syncbuf[i] = 0.0;
	}

	void rx_bit(int bit) {
		shreg = (shreg << 1) | !!bit;
		if ((shreg & 3) == 0) {
			int c = psk_varicode_decode(shreg >> 2);
			if ((c != -1) && (dcd == true)) out.push_back((char)c);
			shreg = 0;
		}
	}

	void phaseafc() {
		if (afcmetric < 0.05) return;
		double error = (phase - bits * M_PI / 2.0);
		if (error < -M_PI / 2.0 || error > M_PI / 2.0) return;
		error *= samplerate / (TWOPI * symbollen);
		if (fabs(error) < sc_bw) {
			double freqerr = error / dcdbits;
			frequency -= freqerr;
		}
		if (acquire) acquire--;
	}

	void afc() {
		if (!afconoff) return;
		if (dcd == true || acquire) phaseafc();
	}

	void rx_symbol(cmplx symbol) {
		const int n = 2;
		phase = arg(conj(prevsymbol) * symbol);
		prevsymbol = symbol;
		if (phase < 0) phase += TWOPI;
		bits = (((int)(phase / M_PI + 0.5)) & (n - 1)) << 1;

		double cval = cos(n * phase);
		double sval = sin(n * phase);
		double decay = SQLDECAY, attack = SQLDECAY;
		quality = cmplx(decayavg(quality.real(), cval, cval > quality.real() ? attack : decay),
				decayavg(quality.imag(), sval, sval > quality.real() ? attack : decay));
		metric = 100.0 * norm(quality);
		if (metric > 100) metric = 100;
		afcmetric = decayavg(afcmetric, norm(quality), 50);

		dcdshreg = (dcdshreg << 2) | bits;
		int set_dcd = -1;
		switch (dcdshreg) {
		case 0xAAAAAAAA: set_dcd = 1; break;
		case 0x00000000: set_dcd = 0; break;
		default:
			if (metric > sqlvalue || sqlonoff == false) dcd = true;
			else dcd = false;
			dcdOFFcounter -= 1;
			if (dcdOFFcounter < 0) dcdOFFcounter = 0;
			break;
		}
		if (1 == set_dcd) {
			dcdOFFcounter = 0;
			dcd = true;
			acquire = 0;
			quality = cmplx(1.0, 0.0);
		} else if (0 == set_dcd) {
			if (++dcdOFFcounter > 5) {
				dcdOFFcounter = 0;
				dcd = false;
				acquire = 0;
				quality = cmplx(0.0, 0.0);
			}
		}
		if (dcd == true) rx_bit(!bits);
	}

	void rx_process(const float *buf, size_t len) {
		cmplx z, z2;
		double delta = TWOPI * frequency / samplerate;
		for (size_t k = 0; k < len; k++) {
			bool can_rx_symbol = false;
			z = cmplx(buf[k] * cos(phaseacc), buf[k] * sin(phaseacc));
			phaseacc += delta;
			if (phaseacc > TWOPI) phaseacc -= TWOPI;
			if (fir1.run(z, z)) {
				fir2.run(z, z2);
				int idx = (int)bitclk;
				double sum = 0.0, ampsum = 0.0;
				syncbuf[idx] = 0.8 * syncbuf[idx] + 0.2 * abs(z2);
				double bitsteps = 16;
				int symsteps = 8;
				for (int i = 0; i < symsteps; i++) {
					sum += (syncbuf[i] - syncbuf[i + symsteps]);
					ampsum += (syncbuf[i] + syncbuf[i + symsteps]);
				}
				sum = (ampsum == 0 ? 0 : sum / ampsum);
				bitclk -= sum / (5.0 * 16 / bitsteps);
				bitclk += 1;
				if (bitclk < 0) bitclk += bitsteps;
				if (bitclk >= bitsteps) {
					bitclk -= bitsteps;
					can_rx_symbol = true;
					afc(); // la frequenza nuova vale dal blocco dopo, come in fldigi
				}
			}
			if (can_rx_symbol) rx_symbol(z2);
		}
	}
};

} // namespace

extern "C" {

void *aurora_fldigi_psk_new(double baud, double freq, double samplerate, int afc, int squelch, double squelch_value) {
	return new FldigiPsk(baud, freq, samplerate, afc != 0, squelch != 0, squelch_value);
}

void aurora_fldigi_psk_process(void *rx, const float *samples, size_t len) {
	if (rx && samples) static_cast<FldigiPsk *>(rx)->rx_process(samples, len);
}

size_t aurora_fldigi_psk_take(void *rx, char *buf, size_t cap) {
	if (!rx || !buf) return 0;
	std::string &out = static_cast<FldigiPsk *>(rx)->out;
	size_t n = out.size() < cap ? out.size() : cap;
	memcpy(buf, out.data(), n);
	out.erase(0, n);
	return n;
}

double aurora_fldigi_psk_frequency(void *rx) {
	return rx ? static_cast<FldigiPsk *>(rx)->frequency : 0.0;
}

void aurora_fldigi_psk_free(void *rx) {
	delete static_cast<FldigiPsk *>(rx);
}

}
