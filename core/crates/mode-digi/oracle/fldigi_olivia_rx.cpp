// Il ricevitore di Olivia e Contestia di fldigi 4.2.13 (MFSK_Receiver della libreria di Pawel Jalocha, `terze-parti/fldigi/src/include/jalocha`)
// usato come arbitro nelle prove di Aurora: si compila solo con la feature `fldigi-oracle` del crate mode-digi e non entra mai nel programma.
// Qui c'e' solo l'interfaccia C. Licenza GPL 3 o successiva, come Aurora.

#include <cmath>
#include <cstddef>
#include <cstdint>
#include <string>
#include <vector>

#include "pj_mfsk.h"

namespace {

struct OliviaRx {
	MFSK_Receiver<float> rx;
	std::string out;
	bool ok = false;
};

void drain(OliviaRx *r) {
	uint8_t c;
	while (r->rx.GetChar(c) > 0) r->out.push_back((char)c);
}

}

extern "C" {

// `center_hz`: il centro della banda; l'audio e' a 8000 Hz (la frequenza interna della libreria).
void *aurora_fldigi_olivia_rx_new(int tones, int bandwidth, double center_hz, int sync_margin, int sync_integ, double threshold, int contestia) {
	OliviaRx *r = new OliviaRx();
	r->rx.bContestia = contestia != 0;
	r->rx.Tones = (size_t)tones;
	r->rx.Bandwidth = (size_t)bandwidth;
	r->rx.SyncMargin = (size_t)sync_margin;
	r->rx.SyncIntegLen = (size_t)sync_integ;
	r->rx.SyncThreshold = (float)threshold;
	r->rx.SampleRate = 8000.0f;
	r->rx.InputSampleRate = 8000.0f;
	r->rx.Reverse = 0;
	double fc_offset = bandwidth * (1.0 - 0.5 / tones) / 2.0;
	r->rx.FirstCarrierMultiplier = (float)((center_hz - fc_offset) / 500.0);
	r->ok = r->rx.Preset() >= 0;
	return r;
}

void aurora_fldigi_olivia_rx_process(void *p, const float *samples, size_t len) {
	OliviaRx *r = (OliviaRx *)p;
	if (!r->ok) return;
	std::vector<float> buf(samples, samples + len);
	r->rx.Process(buf.data(), len);
	drain(r);
}

void aurora_fldigi_olivia_rx_flush(void *p) {
	OliviaRx *r = (OliviaRx *)p;
	if (!r->ok) return;
	r->rx.Flush();
	drain(r);
}

size_t aurora_fldigi_olivia_rx_take(void *p, uint8_t *buf, size_t cap) {
	OliviaRx *r = (OliviaRx *)p;
	size_t n = r->out.size() < cap ? r->out.size() : cap;
	for (size_t i = 0; i < n; i++) buf[i] = (uint8_t)r->out[i];
	r->out.erase(0, n);
	return n;
}

double aurora_fldigi_olivia_rx_snr(void *p) {
	OliviaRx *r = (OliviaRx *)p;
	return r->ok ? (double)r->rx.SignalToNoiseRatio() : 0.0;
}

void aurora_fldigi_olivia_rx_free(void *p) { delete (OliviaRx *)p; }

}
