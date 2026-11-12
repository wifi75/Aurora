// Ricevitore RTTY di fldigi 4.2.13, estratto per le prove di Aurora («oracolo»): si compila solo con la feature `fldigi-oracle` del
// crate mode-digi (build.rs) e non entra mai nel programma.
//
// Il codice qui sotto e' ripreso da fldigi, copia in terze-parti/fldigi:
//   src/rtty/rtty.cxx      rtty::rx_process, rtty::rx, rtty::is_mark_space, rtty::is_mark, rtty::decode_char, rtty::baudot_dec,
//                          rtty::mixer, rtty::restart e rtty::reset_filters (parametri), tabelle letters e USTTY_figures;
//   src/filters/fftfilt.cxx fftfilt (filtro FFT a sovrapposizione e somma, rtty_filter), compilato cosi' com'e';
//   src/include/fftfilt.h, gfft.h, complex.h, misc.h.
// Autori: Dave Freese W1HKJ e i contributori di fldigi (fra gli altri Stefan Fendt DO2SMF per il filtro e l'ATC); licenza GPL 3 o
// successiva, come Aurora.
//
// Tolti perche' non servono a decidere i caratteri: AFC, oscilloscopi, metrica e squelch (sempre aperto), Synop, FSK, FLTK, la
// visualizzazione. Il percorso dal campione al carattere e' quello di fldigi: mixer, due fftfilt, ATC «optimal» (v3), macchina dei bit,
// UOS in ricezione acceso (UOSrx di base), CR CR e LF LF scartati. Sempre USB e Reverse spento: reverse = Reverse ^ !USB = false.

#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <string>
#include <vector>

#include "complex.h"
#include "fftfilt.h"
#include "misc.h"

namespace {

const char letters[32] = {
	'\0',	'E',	'\n',	'A',	' ',	'S',	'I',	'U',
	'\r',	'D',	'R',	'J',	'N',	'F',	'C',	'K',
	'T',	'Z',	'L',	'W',	'H',	'Y',	'P',	'Q',
	'O',	'B',	'G',	' ',	'M',	'X',	'V',	' '
};

// U.S. version of the figures case.
const char USTTY_figures[32] = {
	'\0',	'3',	'\n',	'-',	' ',	'\a',	'8',	'7',
	'\r',	'$',	'4',	'\'',	',',	'!',	':',	'(',
	'5',	'"',	')',	'2',	'#',	'6',	'0',	'1',
	'9',	'?',	'&',	' ',	'.',	'/',	';',	' '
};

const double BAUD[]  = {45, 45.45, 50, 56, 75, 100, 110, 150, 200, 300};
const int FILTLEN[] = { 512, 512, 512, 512, 512, 512, 512, 256, 128, 64};
const double TWOPI = 2.0 * M_PI;

enum { RTTY_RX_STATE_IDLE, RTTY_RX_STATE_START, RTTY_RX_STATE_DATA, RTTY_RX_STATE_STOP };
enum { LETTERS = 0x100, FIGURES = 0x200 };

struct FldigiRtty {
	double samplerate;
	double frequency;
	double shift;
	double rtty_baud;
	int symbollen;
	int nbits;
	bool reverse;
	bool UOSrx;
	fftfilt *mark_filt;
	fftfilt *space_filt;
	double mark_phase, space_phase;
	double mark_mag, space_mag, mark_env, space_env, mark_noise, space_noise, noise_floor;
	std::vector<bool> bit_buf;
	int rxstate, counter, bitcntr;
	unsigned int rxdata;
	int rxmode;
	int lastchar;
	std::string out;

	FldigiRtty(double baud, double shift_hz, double centre, double sr)
		: samplerate(sr), frequency(centre), shift(shift_hz), rtty_baud(baud), nbits(5), reverse(false), UOSrx(true),
		  mark_phase(0), space_phase(0), mark_mag(0), space_mag(0), mark_env(0), space_env(0), mark_noise(0), space_noise(0),
		  noise_floor(0), rxstate(RTTY_RX_STATE_IDLE), counter(0), bitcntr(0), rxdata(0), rxmode(LETTERS), lastchar(0) {
		// rtty::restart: lunghezza del simbolo e del filtro dalla tabella delle velocita'
		int filter_length = 512;
		for (int i = 0; i < (int)(sizeof(BAUD) / sizeof(*BAUD)); i++)
			if (std::fabs(BAUD[i] - baud) < 0.01) filter_length = FILTLEN[i];
		symbollen = (int)(samplerate / rtty_baud + 0.5);
		bit_buf.assign(symbollen, false);
		// rtty::reset_filters
		mark_filt = new fftfilt(rtty_baud / samplerate, filter_length);
		mark_filt->rtty_filter(rtty_baud / samplerate);
		space_filt = new fftfilt(rtty_baud / samplerate, filter_length);
		space_filt->rtty_filter(rtty_baud / samplerate);
	}

	~FldigiRtty() {
		delete mark_filt;
		delete space_filt;
	}

	cmplx mixer(double &phase, double f, cmplx in) {
		cmplx z = cmplx(cos(phase), sin(phase)) * in;
		phase -= TWOPI * f / samplerate;
		if (phase < -TWOPI) phase += TWOPI;
		return z;
	}

	char baudot_dec(unsigned char data) {
		int o = 0;
		switch (data) {
		case 0x1F:		/* letters */
			rxmode = LETTERS;
			break;
		case 0x1B:		/* figures */
			rxmode = FIGURES;
			break;
		case 0x04:		/* unshift-on-space */
			if (UOSrx)
				rxmode = LETTERS;
			return ' ';
		default:
			if (rxmode == LETTERS)
				o = letters[data];
			else
				o = USTTY_figures[data];
			break;
		}
		return (char)o;
	}

	int decode_char() {
		unsigned int data = rxdata & ((1 << nbits) - 1);
		return baudot_dec((unsigned char)data);
	}

	bool is_mark_space(int &correction) {
		correction = 0;
		// test for rough bit position
		if (bit_buf[0] && !bit_buf[symbollen - 1]) {
			// test for mark/space straddle point
			for (int i = 0; i < symbollen; i++)
				correction += bit_buf[i];
			if (abs(symbollen / 2 - correction) < 6) // too small & bad signals are not decoded
				return true;
		}
		return false;
	}

	bool is_mark() {
		return bit_buf[symbollen / 2];
	}

	bool rx(bool bit) {
		bool flag = false;
		int c = 0;
		int correction;

		for (int i = 1; i < symbollen; i++) bit_buf[i - 1] = bit_buf[i];
		bit_buf[symbollen - 1] = bit;

		switch (rxstate) {
		case RTTY_RX_STATE_IDLE:
			if (is_mark_space(correction)) {
				rxstate = RTTY_RX_STATE_START;
				counter = correction;
			}
			break;
		case RTTY_RX_STATE_START:
			if (--counter == 0) {
				if (!is_mark()) {
					rxstate = RTTY_RX_STATE_DATA;
					counter = symbollen;
					bitcntr = 0;
					rxdata = 0;
				} else {
					rxstate = RTTY_RX_STATE_IDLE;
				}
			}
			break;
		case RTTY_RX_STATE_DATA:
			if (--counter == 0) {
				rxdata |= is_mark() << bitcntr++;
				counter = symbollen;
			}
			if (bitcntr == nbits)
				rxstate = RTTY_RX_STATE_STOP;
			break;
		case RTTY_RX_STATE_STOP:
			if (--counter == 0) {
				if (is_mark()) {
					c = decode_char();
					if (c != 0) {
						// supress <CR><CR> and <LF><LF> sequences
						if (c == '\r' && lastchar == '\r');
						else if (c == '\n' && lastchar == '\n');
						else
							out.push_back((char)c);
						lastchar = c;
					}
					flag = true;
				}
				rxstate = RTTY_RX_STATE_IDLE;
			}
			break;
		default: break;
		}
		return flag;
	}

	void rx_process(const float *buf, size_t len) {
		cmplx z, zmark, zspace, *zp_mark, *zp_space;
		for (size_t k = 0; k < len; k++) {
			z = cmplx(buf[k], buf[k]);
			zmark = mixer(mark_phase, frequency + shift / 2.0, z);
			mark_filt->run(zmark, &zp_mark);
			zspace = mixer(space_phase, frequency - shift / 2.0, z);
			int n_out = space_filt->run(zspace, &zp_space);
			for (int i = 0; i < n_out; i++) {
				mark_mag = abs(zp_mark[i]);
				mark_env = decayavg(mark_env, mark_mag, (mark_mag > mark_env) ? symbollen / 4 : symbollen * 16);
				mark_noise = decayavg(mark_noise, mark_mag, (mark_mag < mark_noise) ? symbollen / 4 : symbollen * 48);
				space_mag = abs(zp_space[i]);
				space_env = decayavg(space_env, space_mag, (space_mag > space_env) ? symbollen / 4 : symbollen * 16);
				space_noise = decayavg(space_noise, space_mag, (space_mag < space_noise) ? symbollen / 4 : symbollen * 48);
				noise_floor = std::min(space_noise, mark_noise);

				double mclipped = 0, sclipped = 0;
				mclipped = mark_mag > mark_env ? mark_env : mark_mag;
				sclipped = space_mag > space_env ? space_env : space_mag;
				if (mclipped < noise_floor) mclipped = noise_floor;
				if (sclipped < noise_floor) sclipped = noise_floor;

				// Optimal ATC
				double v3 = (mclipped - noise_floor) * (mark_env - noise_floor) -
						(sclipped - noise_floor) * (space_env - noise_floor) - 0.25 * (
						(mark_env - noise_floor) * (mark_env - noise_floor) -
						(space_env - noise_floor) * (space_env - noise_floor));
				bool bit = v3 > 0;
				rx(reverse ? !bit : bit);
			}
		}
	}
};

} // namespace

extern "C" {

void *aurora_fldigi_rtty_new(double baud, double shift, double centre, double samplerate) {
	return new FldigiRtty(baud, shift, centre, samplerate);
}

void aurora_fldigi_rtty_process(void *rx, const float *samples, size_t len) {
	if (rx && samples) static_cast<FldigiRtty *>(rx)->rx_process(samples, len);
}

// Copia in `buf` (al massimo `cap` byte) i caratteri ricevuti e li toglie; restituisce quanti.
size_t aurora_fldigi_rtty_take(void *rx, char *buf, size_t cap) {
	if (!rx || !buf) return 0;
	std::string &out = static_cast<FldigiRtty *>(rx)->out;
	size_t n = out.size() < cap ? out.size() : cap;
	memcpy(buf, out.data(), n);
	out.erase(0, n);
	return n;
}

void aurora_fldigi_rtty_free(void *rx) {
	delete static_cast<FldigiRtty *>(rx);
}

}
