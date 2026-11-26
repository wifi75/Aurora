// SITOR-B (Amtor FEC) di fldigi 4.2.13 (ricevitore e trasmettitore del modo SITORB), estratti per le prove di Aurora («oracolo»): si
// compila solo con la feature `fldigi-oracle` del crate mode-digi (build.rs) e non entra mai nel programma.
//
// Il codice qui sotto e' ripreso da fldigi, copia in terze-parti/fldigi:
//   src/navtex/navtex.cxx   tabelle code_to_ltrs e USTTY_code_to_figs, classe CCIR476, navtex_implementation (costruttore, filtri,
//                           fec_offset, flip_smallest_bit, find_alpha_characters, handle_bit_value, process_bytes, process_char,
//                           filter_print, process_multicorrelator, process_data, mixer, noise_decay, envelope_decay,
//                           process_fft_output, create_fec, encode, send_sine, send_bit, send_string) nel ramo «solo SITOR-B»;
//   src/filters/fftfilt.cxx fftfilt (rtty_filter), compilato cosi' com'e';
//   src/include/fftfilt.h, complex.h, misc.h (decayavg).
// Autori: Remi Chateauneu F4ECW e Rik van Riel AB1KW (da JNX di Paul Lutus), nei sorgenti di fldigi di Dave Freese W1HKJ
// e contributori; licenza GPL 3 o successiva, come Aurora.
//
// Tolti perche' non servono a decidere i caratteri: messaggi NAVTEX (intestazioni ZCZC/NNNN, catalogo delle stazioni, KML), AFC
// (spento), metrica, FLTK. USB senza Rev (mark = tono alto), cifre USTTY (ITA2 spento: di base in fldigi).

#include <climits>
#include <cmath>
#include <cstddef>
#include <cstring>
#include <string>
#include <vector>

#include "complex.h"
#include "fftfilt.h"
#include "misc.h"

namespace {

const double TWOPI = 2.0 * M_PI;
const int deviation_f = 85;

const unsigned char code_to_ltrs[128] = {
	'_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_',
	'_', '_', '_', '_', '_', '_', '_', 'J', '_', '_', '_', 'F', '_', 'C', 'K', '_',
	'_', '_', '_', '_', '_', '_', '_', 'W', '_', '_', '_', 'Y', '_', 'P', 'Q', '_',
	'_', '_', '_', '_', '_', 'G', '_', '_', '_', 'M', 'X', '_', 'V', '_', '_', '_',
	'_', '_', '_', '_', '_', '_', '_', 'A', '_', '_', '_', 'S', '_', 'I', 'U', '_',
	'_', '_', '_', 'D', '_', 'R', 'E', '_', '_', 'N', '_', '_', ' ', '_', '_', '_',
	'_', '_', '_', 'Z', '_', 'L', '_', '_', '_', 'H', '_', '_', '\n', '_', '_', '_',
	'_', 'O', 'B', '_', 'T', '_', '_', '_', '\r', '_', '_', '_', '_', '_', '_', '_'};

const unsigned char USTTY_code_to_figs[128] = {
	'_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_', '_',
	'_', '_', '_', '_', '_', '_', '_', '\'', '_', '_', '_', '!', '_', ':', '(', '_',
	'_', '_', '_', '_', '_', '_', '_', '2', '_', '_', '_', '6', '_', '0', '1', '_',
	'_', '_', '_', '_', '_', '&', '_', '_', '_', '.', '/', '_', ';', '_', '_', '_',
	'_', '_', '_', '_', '_', '_', '_', '-', '_', '_', '_', '\07', '_', '8', '7', '_',
	'_', '_', '_', '$', '_', '4', '3', '_', '_', ',', '_', '_', ' ', '_', '_', '_',
	'_', '_', '_', '"', '_', ')', '_', '_', '_', '#', '_', '_', '\n', '_', '_', '_',
	'_', '9', '?', '_', '5', '_', '_', '_', '\r', '_', '_', '_', '_', '_', '_', '_'};

const int code_ltrs = 0x5a;
const int code_figs = 0x36;
const int code_alpha = 0x0f;
const int code_beta = 0x33;
const int code_char32 = 0x6a;
const int code_rep = 0x66;
const int char_bell = 0x07;

bool check_bits(int v) {
	int bc = 0;
	while (v != 0) {
		bc++;
		v &= v - 1;
	}
	return bc == 4;
}

struct CCIR476 {
	unsigned char m_ltrs_to_code[128];
	unsigned char m_figs_to_code[128];
	CCIR476() {
		memset(m_ltrs_to_code, 0, 128);
		memset(m_figs_to_code, 0, 128);
		for (int code = 0; code < 128; code++) {
			if (check_bits(code)) {
				unsigned char figv = USTTY_code_to_figs[code];
				unsigned char ltrv = code_to_ltrs[code];
				if (figv != '_') m_figs_to_code[figv] = code;
				if (ltrv != '_') m_ltrs_to_code[ltrv] = code;
			}
		}
	}
	void char_to_code(std::string &str, int ch, bool &ex_shift) const {
		ch = toupper(ch);
		if (ex_shift && m_figs_to_code[ch] != '\0') str.push_back(m_figs_to_code[ch]);
		else if (!ex_shift && m_ltrs_to_code[ch] != '\0') str.push_back(m_ltrs_to_code[ch]);
		else if (m_figs_to_code[ch] != '\0') {
			ex_shift = true;
			str.push_back(code_figs);
			str.push_back(m_figs_to_code[ch]);
		} else if (m_ltrs_to_code[ch] != '\0') {
			ex_shift = false;
			str.push_back(code_ltrs);
			str.push_back(m_ltrs_to_code[ch]);
		}
	}
	int code_to_char(int code, bool shift) const {
		const unsigned char *target = shift ? USTTY_code_to_figs : code_to_ltrs;
		if (target[code] != '_') return target[code];
		return -code;
	}
	int bytes_to_code(int *pos) {
		int code = 0;
		for (int i = 0; i < 7; i++) code |= ((pos[i] > 0) << i);
		return code;
	}
	bool valid_char_at(int *pos) {
		int count = 0;
		for (int i = 0; i < 7; i++)
			if (pos[i] > 0) count++;
		return count == 4;
	}
};

enum State { SYNC_SETUP, SYNC, READ_DATA };

struct FldigiSitorb {
	CCIR476 m_ccir476;
	double m_center_frequency_f, m_mark_f, m_space_f, m_mark_phase, m_space_phase;
	double m_sample_rate, m_baud_rate;
	int m_bit_sample_count;
	double m_next_early_event, m_next_prompt_event, m_next_late_event;
	double m_average_early_signal, m_average_prompt_signal, m_average_late_signal;
	int m_error_count;
	long m_sample_count;
	std::vector<int> m_bit_values;
	int m_bit_cursor;
	State m_state;
	bool m_shift, m_alpha_phase, m_pulse_edge_event;
	fftfilt *m_mark_lowpass, *m_space_lowpass;
	double mark_env, space_env, mark_noise, space_noise;
	int last_char;
	std::string out;
	// trasmissione
	double tx_phase;

	FldigiSitorb(double samplerate, double centre) {
		m_sample_rate = samplerate;
		m_state = SYNC_SETUP;
		m_shift = false;
		m_alpha_phase = false;
		m_center_frequency_f = centre;
		m_baud_rate = 100;
		m_bit_sample_count = (int)(m_sample_rate * (1.0 / m_baud_rate));
		m_next_early_event = 0;
		m_next_prompt_event = m_bit_sample_count / 5;
		m_next_late_event = m_bit_sample_count * 2 / 5;
		m_average_early_signal = m_average_prompt_signal = m_average_late_signal = 0;
		m_error_count = 0;
		m_sample_count = 0;
		m_bit_values.resize((size_t)m_baud_rate);
		m_bit_cursor = 0;
		m_mark_f = m_center_frequency_f + deviation_f;
		m_space_f = m_center_frequency_f - deviation_f;
		m_mark_phase = m_space_phase = 0;
		const int filtlen = 512;
		m_mark_lowpass = new fftfilt(m_baud_rate / m_sample_rate, filtlen);
		m_mark_lowpass->rtty_filter(m_baud_rate / m_sample_rate);
		m_space_lowpass = new fftfilt(m_baud_rate / m_sample_rate, filtlen);
		m_space_lowpass->rtty_filter(m_baud_rate / m_sample_rate);
		mark_env = space_env = mark_noise = space_noise = 0;
		last_char = 0;
		tx_phase = 0;
	}
	~FldigiSitorb() {
		delete m_mark_lowpass;
		delete m_space_lowpass;
	}

	int fec_offset(int offset) { return offset - 35; }

	void flip_smallest_bit(int *pos) {
		int min_zero = INT_MIN, min_one = INT_MAX;
		int min_zero_pos = -1, min_one_pos = -1;
		int count_zero = 0, count_one = 1;
		for (int i = 0; i < 7; i++) {
			int val = pos[i];
			if (val < 0) {
				count_zero++;
				if (val > min_zero) {
					min_zero = val;
					min_zero_pos = i;
				}
			} else {
				count_one++;
				if (val < min_one) {
					min_one = val;
					min_one_pos = i;
				}
			}
		}
		if (count_zero == 4) pos[min_zero_pos] = -pos[min_zero_pos];
		else if (count_one == 5) pos[min_one_pos] = -pos[min_one_pos];
	}

	int find_alpha_characters() {
		int best_offset = 0, best_score = 0;
		for (int offset = 35; offset < (35 + 14); offset++) {
			int score = 0, reps = 0;
			int limit = (int)m_bit_values.size() - 7;
			for (int i = offset; i < limit; i += 7) {
				if (m_ccir476.valid_char_at(&m_bit_values[i])) {
					int ri = fec_offset(i);
					int code = m_ccir476.bytes_to_code(&m_bit_values[i]);
					int rep = m_ccir476.bytes_to_code(&m_bit_values[ri]);
					score++;
					if (code == rep) {
						if (code == code_alpha || code == code_rep) {
							score = 0;
							continue;
						}
						reps++;
					} else if (code == code_alpha) {
						int ri2 = i - 7;
						int rep2 = m_ccir476.bytes_to_code(&m_bit_values[ri2]);
						if (rep2 == code_rep) reps++;
					}
				}
			}
			if (reps >= 3 && score + reps > best_score) {
				best_score = score + reps;
				best_offset = offset;
			}
		}
		return best_score > 8 ? best_offset : -1;
	}

	void handle_bit_value(int logic_level) {
		int buffersize = (int)m_bit_values.size();
		for (int i = 0; i < buffersize - 1; i++) m_bit_values[i] = m_bit_values[i + 1];
		m_bit_values[buffersize - 1] = logic_level;
		if (m_bit_cursor > 0) m_bit_cursor--;
		if (m_state == SYNC) {
			int offset = find_alpha_characters();
			if (offset >= 0) {
				m_state = READ_DATA;
				m_bit_cursor = offset;
				m_alpha_phase = true;
			} else
				m_state = SYNC_SETUP;
		}
		if (m_state == READ_DATA) {
			if (m_bit_cursor < buffersize - 7) {
				if (m_alpha_phase) {
					int ret = process_bytes(m_bit_cursor);
					m_error_count -= ret;
					if (m_error_count > 5) m_state = SYNC_SETUP;
					if (m_error_count < 0) m_error_count = 0;
				}
				m_alpha_phase = !m_alpha_phase;
				m_bit_cursor += 7;
			}
		}
	}

	int process_bytes(int cursor) {
		int code = m_ccir476.bytes_to_code(&m_bit_values[cursor]);
		int success = 0;
		if (check_bits(code)) {
			success = 1;
			goto decode;
		}
		if (fec_offset(cursor) < 0) return -1;
		{
			int calc, avg[7];
			int reppos = fec_offset(cursor);
			int rep = m_ccir476.bytes_to_code(&m_bit_values[reppos]);
			if (check_bits(rep)) {
				if (rep == code_rep) return 0;
				code = rep;
				goto decode;
			}
			for (int i = 0; i < 7; i++) avg[i] = m_bit_values[cursor + i] + m_bit_values[reppos + i];
			calc = m_ccir476.bytes_to_code(avg);
			if (check_bits(calc)) {
				code = calc;
				success = -1;
				goto decode;
			}
			flip_smallest_bit(&m_bit_values[cursor]);
			calc = m_ccir476.bytes_to_code(&m_bit_values[cursor]);
			if (check_bits(calc)) {
				code = calc;
				success = -1;
				goto decode;
			}
			flip_smallest_bit(&m_bit_values[reppos]);
			calc = m_ccir476.bytes_to_code(&m_bit_values[reppos]);
			if (check_bits(calc)) {
				code = calc;
				success = -1;
				goto decode;
			}
			flip_smallest_bit(avg);
			calc = m_ccir476.bytes_to_code(avg);
			if (check_bits(calc)) {
				code = calc;
				success = -1;
				goto decode;
			}
			return -2;
		}
	decode:
		process_char(code);
		return success;
	}

	void process_char(int chr) {
		switch (chr) {
		case code_rep:
			if (last_char == code_rep) m_alpha_phase = false;
			break;
		case code_alpha:
		case code_beta:
		case code_char32:
			break;
		case code_ltrs:
			m_shift = false;
			break;
		case code_figs:
			m_shift = true;
			break;
		default:
			chr = m_ccir476.code_to_char(chr, m_shift);
			if (chr >= 0) filter_print(chr);
			break;
		}
		last_char = chr;
	}

	void filter_print(int c) {
		if (c == char_bell) out.push_back('\'');
		else if (c != -1 && c != '\r' && c != code_alpha && c != code_rep) out.push_back((char)c);
	}

	void process_multicorrelator() {
		if (m_sample_count % (int)(m_bit_sample_count * 8)) return;
		double slope = m_average_late_signal - m_average_early_signal;
		slope /= (m_average_prompt_signal + 0.1);
		slope *= 22;
		if (m_average_prompt_signal > m_average_early_signal && m_average_prompt_signal > m_average_late_signal) slope /= 2;
		if (slope) {
			m_next_early_event += slope;
			m_next_prompt_event += slope;
			m_next_late_event += slope;
		}
	}

	cmplx mixer(double &phase, double f, cmplx in) {
		cmplx z = cmplx(cos(phase), sin(phase)) * in;
		phase -= TWOPI * f / m_sample_rate;
		if (phase < -TWOPI) phase += TWOPI;
		return z;
	}

	double noise_decay(double avg, double value) {
		int divisor = value < avg ? m_bit_sample_count / 4 : m_bit_sample_count * 48;
		return decayavg(avg, value, divisor);
	}

	double envelope_decay(double avg, double value) {
		int divisor = value > avg ? m_bit_sample_count / 4 : m_bit_sample_count * 16;
		return decayavg(avg, value, divisor);
	}

	void process_data(const float *data, size_t nb_samples) {
		cmplx z, zmark, zspace, *zp_mark, *zp_space;
		for (size_t i = 0; i < nb_samples; ++i) {
			double dv = 32767 * data[i];
			z = cmplx(dv, dv);
			zmark = mixer(m_mark_phase, m_mark_f, z);
			m_mark_lowpass->run(zmark, &zp_mark);
			zspace = mixer(m_space_phase, m_space_f, z);
			int n_out = m_space_lowpass->run(zspace, &zp_space);
			if (n_out) process_fft_output(zp_mark, zp_space, n_out);
		}
	}

	void process_fft_output(cmplx *zp_mark, cmplx *zp_space, int samples) {
		for (int i = 0; i < samples; i++) {
			double mark_abs = abs(zp_mark[i]);
			double space_abs = abs(zp_space[i]);
			process_multicorrelator();
			mark_env = envelope_decay(mark_env, mark_abs);
			mark_noise = noise_decay(mark_noise, mark_abs);
			space_env = envelope_decay(space_env, space_abs);
			space_noise = noise_decay(space_noise, space_abs);
			double noise_floor = (space_noise + mark_noise) / 2;
			mark_abs = std::min(mark_abs, mark_env);
			mark_abs = std::max(mark_abs, noise_floor);
			space_abs = std::min(space_abs, space_env);
			space_abs = std::max(space_abs, noise_floor);
			double logic_level = (mark_abs - noise_floor) * (mark_env - noise_floor) - (space_abs - noise_floor) * (space_env - noise_floor) -
				0.5 * ((mark_env - noise_floor) * (mark_env - noise_floor) - (space_env - noise_floor) * (space_env - noise_floor));
			if (m_sample_count >= m_next_early_event) {
				m_average_early_signal = decayavg(m_average_early_signal, fabs(logic_level), 64);
				m_next_early_event += m_bit_sample_count;
			}
			if (m_sample_count >= m_next_late_event) {
				m_average_late_signal = decayavg(m_average_late_signal, fabs(logic_level), 64);
				m_next_late_event += m_bit_sample_count;
			}
			m_pulse_edge_event = m_sample_count >= m_next_prompt_event;
			if (m_pulse_edge_event) {
				m_average_prompt_signal = decayavg(m_average_prompt_signal, fabs(logic_level), 64);
				m_next_prompt_event += m_bit_sample_count;
			}
			switch (m_state) {
			case SYNC_SETUP:
				m_error_count = 0;
				m_shift = false;
				m_state = SYNC;
				break;
			case SYNC:
			case READ_DATA:
				if (m_pulse_edge_event) handle_bit_value((int)logic_level);
			}
			m_sample_count++;
		}
	}

	// trasmissione (ramo m_only_sitor_b di send_message: send_string senza fasatura lunga)
	std::string create_fec(const std::string &str) const {
		std::string res;
		const size_t sz = str.size();
		static const size_t offset = 2;
		for (size_t i = 0; i < offset; ++i) {
			res.push_back(code_rep);
			res.push_back(code_alpha);
		}
		for (size_t i = 0; i < sz; ++i) {
			res.push_back(str[i]);
			res.push_back(i >= offset ? str[i - offset] : code_alpha);
		}
		for (size_t i = 0; i < offset; ++i) {
			res.push_back(code_char32);
			res.push_back(str[sz - offset + i]);
		}
		return res;
	}

	std::string encode(const std::string &str) const {
		std::string res;
		bool shift = false;
		for (size_t i = 0; i < str.size(); ++i) m_ccir476.char_to_code(res, str[i], shift);
		return res;
	}

	void send_sine(double seconds, double freq, std::vector<float> &tx) {
		int nb_samples = (int)(seconds * m_sample_rate);
		double max_level = 0.9;
		double ratio = 2.0 * M_PI * freq / m_sample_rate;
		for (int i = 0; i < nb_samples; ++i) {
			tx.push_back((float)(max_level * sin(tx_phase += ratio)));
			if (tx_phase > 2.0 * M_PI) tx_phase -= 2.0 * M_PI;
		}
	}

	void send_string(const std::string &msg, std::vector<float> &tx) {
		std::string sevenbits = create_fec(encode(msg));
		for (size_t i = 0; i < sevenbits.size(); i++) {
			char c = sevenbits[i];
			for (size_t j = 0; j < 7; ++j, c >>= 1) send_sine(1.0 / m_baud_rate, (c & 1) ? m_mark_f : m_space_f, tx);
		}
	}
};

} // namespace

extern "C" {

void *aurora_fldigi_sitorb_new(double samplerate, double centre) {
	return new FldigiSitorb(samplerate, centre);
}

void aurora_fldigi_sitorb_process(void *rx, const float *samples, size_t len) {
	if (rx && samples) static_cast<FldigiSitorb *>(rx)->process_data(samples, len);
}

size_t aurora_fldigi_sitorb_take(void *rx, char *buf, size_t cap) {
	if (!rx || !buf) return 0;
	std::string &out = static_cast<FldigiSitorb *>(rx)->out;
	size_t n = out.size() < cap ? out.size() : cap;
	memcpy(buf, out.data(), n);
	out.erase(0, n);
	return n;
}

void aurora_fldigi_sitorb_free(void *rx) {
	delete static_cast<FldigiSitorb *>(rx);
}

size_t aurora_fldigi_sitorb_tx(double samplerate, double centre, const char *text, float *out, size_t cap) {
	if (!text) return 0;
	FldigiSitorb t(samplerate, centre);
	std::vector<float> tx;
	t.send_string(text, tx);
	if (out) memcpy(out, tx.data(), (tx.size() < cap ? tx.size() : cap) * sizeof(float));
	return tx.size();
}

}
