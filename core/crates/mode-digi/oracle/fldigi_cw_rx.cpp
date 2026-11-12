// Ricevitore CW di fldigi 4.2.13, estratto per le prove di Aurora («oracolo»): si compila solo con la feature `fldigi-oracle` del crate
// mode-digi (build.rs) e non entra mai nel programma.
//
// Il codice qui sotto e' ripreso da fldigi, copia in terze-parti/fldigi:
//   src/cw/cw.cxx           cw::cw e cw::reset_rx_filter (filtro, bitfilter, tempi), cw::sync_parameters (parte di ricezione),
//                           cw::update_tracking, cw::rx_FFTprocess, cw::decode_stream (AGC, metrica, squelch, isteresi),
//                           cw::handle_event (stati, punti e linee, inseguimento della velocita', carattere e spazio);
//   src/cw/morse.cxx        la tabella cw_table (lettere, cifre, punteggiatura, segni di procedura) e cMorse::rx_lookup;
//   src/filters/filters.cxx C_FIR_filter e Cmovavg, compilato cosi' com'e';
//   src/include/filters.h, complex.h, misc.h, cw.h (costanti).
// Autori: Dave Freese W1HKJ e i contributori di fldigi (fra gli altri Lawrence Glaister VE7IT per l'inseguimento della velocita');
// licenza GPL 3 o successiva, come Aurora.
//
// Valori di base di fldigi: banda 150 Hz, filtro di 512 coefficienti, inseguimento acceso con escursione di 10 WPM (limiti 5-50), attacco
// e discesa «medium» (200 e 1000), carattere d'errore '*', SOM spento, segni di procedura stampati come <SK>. Tolti: trasmissione,
// oscilloscopi, filtro adattato, SOM, FLTK.

#include <cmath>
#include <cstddef>
#include <cstdlib>
#include <cstring>
#include <string>

#include "complex.h"
#include "filters.h"
#include "misc.h"

namespace {

const int CW_SAMPLERATE = 8000;
const int KWPM = 12 * CW_SAMPLERATE / 10;
const int FIR_DECIMATE = 10;
const int TRACKING_FILTER_SIZE = 16;
const size_t MAX_MORSE_ELEMENTS = 6;

enum { RS_IDLE = 0, RS_IN_TONE, RS_AFTER_TONE };
enum { CW_RESET_EVENT, CW_KEYDOWN_EVENT, CW_KEYUP_EVENT, CW_QUERY_EVENT };

struct MorseEntry {
	const char *prt;
	const char *rpr;
};

// morse.cxx: cw_table (voci attive in ricezione; i segni di procedura come li stampa fldigi di base)
const MorseEntry cw_table[] = {
	{"<BT>", "-...-"}, {"<AS>", ".-..."}, {"<AR>", ".-.-."}, {"<SK>", "...-.-"}, {"<KN>", "-.--."}, {"<INT>", "..-.-"},
	{"<HM>", "....--"}, {"<VE>", "...-."},
	{"A", ".-"}, {"B", "-..."}, {"C", "-.-."}, {"D", "-.."}, {"E", "."}, {"F", "..-."}, {"G", "--."}, {"H", "...."},
	{"I", ".."}, {"J", ".---"}, {"K", "-.-"}, {"L", ".-.."}, {"M", "--"}, {"N", "-."}, {"O", "---"}, {"P", ".--."},
	{"Q", "--.-"}, {"R", ".-."}, {"S", "..."}, {"T", "-"}, {"U", "..-"}, {"V", "...-"}, {"W", ".--"}, {"X", "-..-"},
	{"Y", "-.--"}, {"Z", "--.."},
	{"0", "-----"}, {"1", ".----"}, {"2", "..---"}, {"3", "...--"}, {"4", "....-"}, {"5", "....."}, {"6", "-...."},
	{"7", "--..."}, {"8", "---.."}, {"9", "----."},
	{"\"", ".-..-."}, {"'", ".----."}, {"$", "...-..-"}, {")", "-.--.-"}, {",", "--..--"}, {"-", "-....-"},
	{".", ".-.-.-"}, {"/", "-..-."}, {":", "---..."}, {";", "-.-.-."}, {"?", "..--.."}, {"_", "..--.-"}, {"@", ".--.-."},
	{"!", "-.-.--"},
	{nullptr, nullptr}};

std::string rx_lookup(const std::string &rx) {
	for (int i = 0; cw_table[i].rpr; i++)
		if (rx == cw_table[i].rpr) return cw_table[i].prt;
	return "";
}

struct FldigiCw {
	double samplerate;
	double frequency;
	double bandwidth;
	int cw_speed;
	C_FIR_filter cw_filter;
	Cmovavg bitfilter;
	Cmovavg trackingfilter;
	double sig_avg, noise_floor, agc_peak, metric;
	bool sqlonoff;
	double sqlvalue;
	double upper, lower;
	int cw_receive_state;
	int smpl_ctr;
	int cw_rr_start_timestamp, cw_rr_end_timestamp;
	int two_dots;
	int cw_receive_speed;
	int cw_receive_dot_length;
	int cw_noise_spike_threshold;
	int cw_lower_limit, cw_upper_limit;
	bool space_sent;
	int last_element;
	std::string rx_rep_buf;
	std::string out;

	FldigiCw(double freq, double wpm, double bw, double sr, bool squelch, double squelch_value)
		: samplerate(sr), frequency(freq), bandwidth(bw), cw_speed((int)(wpm + 0.5)), bitfilter(1),
		  trackingfilter(TRACKING_FILTER_SIZE), sig_avg(0.5), noise_floor(1.0), agc_peak(0.0), metric(0.0), sqlonoff(squelch),
		  sqlvalue(squelch_value), upper(0.6), lower(0.4), cw_receive_state(RS_IDLE), smpl_ctr(0), cw_rr_start_timestamp(0),
		  cw_rr_end_timestamp(0), space_sent(true), last_element(0) {
		cw_filter.init_bandpass(512, FIR_DECIMATE, 1.0 * (frequency - bandwidth / 2) / samplerate,
					1.0 * (frequency + bandwidth / 2) / samplerate);
		int symbollen = (int)round(samplerate * 1.2 / cw_speed);
		int bfv = (symbollen / FIR_DECIMATE) / 3;
		if (bfv < 1) bfv = 1;
		bitfilter.setLength(bfv);
		two_dots = 2 * KWPM / cw_speed;
		sync_parameters();
	}

	void sync_parameters() {
		int lowerwpm = cw_speed - 10, upperwpm = cw_speed + 10;
		if (lowerwpm < 5) lowerwpm = 5;
		if (upperwpm > 50) upperwpm = 50;
		cw_lower_limit = 2 * KWPM / upperwpm;
		cw_upper_limit = 2 * KWPM / lowerwpm;
		cw_receive_speed = KWPM / (two_dots / 2);
		if (cw_receive_speed > 0) cw_receive_dot_length = KWPM / cw_receive_speed;
		else cw_receive_dot_length = KWPM / 5;
		cw_noise_spike_threshold = cw_receive_dot_length / 2;
	}

	void update_tracking(int dur_1, int dur_2) {
		static const int min_dot = KWPM / 200;
		static const int max_dash = 3 * KWPM / 5;
		if ((dur_1 > dur_2) && (dur_1 > 4 * dur_2)) return;
		if ((dur_2 > dur_1) && (dur_2 > 4 * dur_1)) return;
		if (dur_1 < min_dot || dur_2 < min_dot) return;
		if (dur_2 > max_dash || dur_2 > max_dash) return;
		two_dots = (int)trackingfilter.run((dur_1 + dur_2) / 2);
		sync_parameters();
	}

	static int usec_diff(int earlier, int later) { return (earlier >= later) ? 0 : (later - earlier); }

	void handle_event(int cw_event, std::string &sc) {
		int element_usec;
		sc.clear();
		switch (cw_event) {
		case CW_KEYDOWN_EVENT:
			if (cw_receive_state == RS_IN_TONE) return;
			if (cw_receive_state == RS_IDLE) {
				smpl_ctr = 0;
				rx_rep_buf.clear();
			}
			cw_rr_start_timestamp = smpl_ctr;
			cw_receive_state = RS_IN_TONE;
			return;
		case CW_KEYUP_EVENT:
			if (cw_receive_state != RS_IN_TONE) return;
			cw_rr_end_timestamp = smpl_ctr;
			element_usec = usec_diff(cw_rr_start_timestamp, cw_rr_end_timestamp);
			sync_parameters();
			if (element_usec && (cw_noise_spike_threshold > 0) && (element_usec < cw_noise_spike_threshold)) {
				cw_receive_state = RS_IDLE;
				return;
			}
			if (last_element > 0) {
				if ((element_usec > 2 * last_element) && (element_usec < 4 * last_element))
					update_tracking(last_element, element_usec);
				if ((last_element > 2 * element_usec) && (last_element < 4 * element_usec))
					update_tracking(element_usec, last_element);
			}
			last_element = element_usec;
			if (element_usec <= two_dots) rx_rep_buf += ".";
			else rx_rep_buf += "-";
			if (rx_rep_buf.length() > MAX_MORSE_ELEMENTS) {
				cw_receive_state = RS_IDLE;
				smpl_ctr = 0;
				return;
			}
			cw_receive_state = RS_AFTER_TONE;
			return;
		case CW_QUERY_EVENT:
			if (cw_receive_state == RS_IN_TONE) return;
			sync_parameters();
			element_usec = usec_diff(cw_rr_end_timestamp, smpl_ctr);
			if (element_usec < (2 * cw_receive_dot_length)) return;
			if (element_usec >= (2 * cw_receive_dot_length) && element_usec <= (4 * cw_receive_dot_length) &&
			    cw_receive_state == RS_AFTER_TONE) {
				sc = rx_lookup(rx_rep_buf);
				if (sc.empty()) sc = "*";
				rx_rep_buf.clear();
				cw_receive_state = RS_IDLE;
				space_sent = false;
				return;
			}
			if ((element_usec > (4 * cw_receive_dot_length)) && !space_sent) {
				sc = " ";
				space_sent = true;
				return;
			}
			return;
		}
	}

	void decode_stream(double value) {
		std::string sc;
		const int attack = 200, decay = 1000;
		sig_avg = decayavg(sig_avg, 0.5 * value, (value > sig_avg ? attack : decay));
		if (value < sig_avg) {
			if (value < noise_floor) noise_floor = decayavg(noise_floor, value, attack);
			else noise_floor = decayavg(noise_floor, value, decay);
		}
		if (value > sig_avg) {
			if (value > agc_peak) agc_peak = decayavg(agc_peak, value, attack);
			else agc_peak = decayavg(agc_peak, value, decay);
		}
		float norm_noise, norm_sig, norm_value;
		if (agc_peak) {
			norm_value = value / agc_peak;
			norm_noise = noise_floor / agc_peak;
			norm_sig = sig_avg / agc_peak;
		} else {
			norm_value = noise_floor;
			norm_noise = noise_floor;
			norm_sig = noise_floor;
		}
		upper = 1.05 * norm_sig;
		lower = 0.95 * norm_sig;
		metric = 0.8 * metric;
		if ((noise_floor > 1e-4) && (noise_floor < sig_avg))
			metric += 0.2 * clamp(2.5 * (20 * log10(norm_sig / norm_noise)), 0, 100);
		if (!sqlonoff || metric > sqlvalue) {
			if ((norm_value > upper) && (cw_receive_state != RS_IN_TONE)) handle_event(CW_KEYDOWN_EVENT, sc);
			else if ((norm_value < lower) && (cw_receive_state == RS_IN_TONE)) handle_event(CW_KEYUP_EVENT, sc);
		}
		handle_event(CW_QUERY_EVENT, sc);
		if (!sc.empty()) out += sc;
	}

	void rx_process(const float *buf, size_t len) {
		for (size_t i = 0; i < len; i++) {
			double fil_in = buf[i], fil_out = 0;
			int n = cw_filter.Irun(fil_in, fil_out);
			smpl_ctr++;
			if (n) decode_stream(bitfilter.run(fabs(fil_out)));
		}
	}
};

} // namespace

extern "C" {

void *aurora_fldigi_cw_new(double freq, double wpm, double bandwidth, double samplerate, int squelch, double squelch_value) {
	return new FldigiCw(freq, wpm, bandwidth, samplerate, squelch != 0, squelch_value);
}

void aurora_fldigi_cw_process(void *rx, const float *samples, size_t len) {
	if (rx && samples) static_cast<FldigiCw *>(rx)->rx_process(samples, len);
}

size_t aurora_fldigi_cw_take(void *rx, char *buf, size_t cap) {
	if (!rx || !buf) return 0;
	std::string &out = static_cast<FldigiCw *>(rx)->out;
	size_t n = out.size() < cap ? out.size() : cap;
	memcpy(buf, out.data(), n);
	out.erase(0, n);
	return n;
}

double aurora_fldigi_cw_wpm(void *rx) {
	return rx ? static_cast<FldigiCw *>(rx)->cw_receive_speed : 0.0;
}

void aurora_fldigi_cw_free(void *rx) {
	delete static_cast<FldigiCw *>(rx);
}

}
