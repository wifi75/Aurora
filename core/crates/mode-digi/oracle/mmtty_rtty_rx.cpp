// Ricevitore RTTY di MMTTY 1.70L, estratto per le prove di Aurora («oracolo», secondo arbitro dopo fldigi): si compila solo con la
// feature `mmtty-oracle` del crate mode-digi (build.rs) e non entra mai nel programma.
//
// MMTTY: Copyright 2000-2013 Makoto Mori JE3HHT e Nobuyuki Oba; manutenzione di Dave Bernstein AA6YQ e JA7UDE (versione 1.70L, commit
// e2c1327 del ramo JA7UDE-1.70L, copia in terze-parti/mmtty). Licenza GNU LGPL 3 o successiva (terze-parti/mmtty/COPYING.LESSER.txt);
// questo file e `mmtty/mmtty_dsp.inc` restano sotto la stessa licenza.
//
// `mmtty/mmtty_dsp.inc` (generato da `mmtty/estrai.py`) contiene, copiate alla lettera (commenti convertiti da Shift-JIS a UTF-8), le classi del ricevitore:
//   CLX.h, CLX.cpp          numeri complessi;
//   fir.h, fir.cpp          DoFIR, CIIRTANK, CDECM2, MakeFilter (Kaiser), MakeHilbert, MakeIIR e CIIR, CDECM4, CINTP4, COVERLIMIT;
//   Rtty.h, Rtty.cpp        CSinTable, CVCO, CVCOX, CSmooz, CFSKDEM (Do, DoFSK con il voto di maggioranza), CScope, CATC, CFIR2, CFIRX,
//                           CSlideFFT, CPHASE (demodulatore FFT), CPLL, CFAVG, CAGCX, CAA6YQ, tabelle _LTR e _FIG.
// Tolti: le dichiarazioni di disegno (DrawGraph) e le inclusioni della VCL; l'unica riga cambiata e' `inline __fastcall IsMax` (Borland,
// tipo di ritorno sottinteso) diventata `inline int __fastcall IsMax`.
//
// Qui sotto: il minimo della VCL che quel codice usa, le variabili globali di ComLib.cpp a 12 kHz (InitSampType: SampFreq 12000,
// DemSamp 6000, DemOver 1), il passa-banda d'ingresso di TSound (Sound.cpp, CalcBPF e il ciclo di lettura), CRTTY::ConvAscii (Rtty.cpp)
// con UOS e la sequenza di TMmttyWd::LoadProfile (Main.cpp) con i valori dei profili di UserPara.ini. Ingresso moltiplicato per 32768
// (MMTTY lavora sui campioni a 16 bit). MMTTY chiama mark il tono basso: per i nostri segnali (mark = tono alto in USB) si usa Rev
// (`m_inv`). AFC spento (in MMTTY sta nel timer della finestra principale, fuori dal ricevitore).

#include <cmath>
#include <cstddef>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <string>

#define __fastcall
typedef int BOOL;
typedef unsigned char BYTE;
typedef const char *LPCSTR;
#ifndef TRUE
#define TRUE 1
#define FALSE 0
#endif
#include <cfloat>
// <values.h> di Borland
#define MAXDOUBLE DBL_MAX
// CVCO tiene la tabella del seno in memoria bloccata (API di Windows): per le prove non serve
static inline BOOL VirtualLock(void *, size_t) { return TRUE; }
static inline BOOL VirtualUnlock(void *, size_t) { return TRUE; }

double SampFreq = 12000.0;
double SampBase = 12000.0;
double DemSamp = 12000.0 * 0.5;
int DemOver = 1;
int SampType = 3;

#include "mmtty/mmtty_dsp.inc"

namespace {

// Un profilo di UserPara.ini (solo i campi del ricevitore).
struct Profile {
	int demtype;      // DEMTYPE: 0 IIR, 1 FIR, 2 PLL, 3 FFT
	double iirbw;     // IIRBW
	int tap;          // Tap
	int bpf;          // RXBPF
	int bpftap;       // RXBPFTAP
	double bpffw;     // RXBPFFW
	int atc;          // ATC
	int stoplen;      // TTYStopLen
	double smooz;     // Smooz
	double smoozIIR;  // SmoozIIR
	double pllLoopFC; // pllLoopFC
	int aa6yq;        // AA6YQ.Enabled
};

// [Define0] Standard RTTY, [Define1] Fluttered signals, [Define2] Fluttered signals (FIR), [Define3] Multi-path, [Define4] AA6YQ FIR512,
// [Define5] FFT demodulator, [Define7] 23Hz RTTY (MTTY).
const Profile PROFILES[] = {
	{0, 60, 72, 0, 56, 100, 0, 4, 70, 40, 250, 0},
	{0, 90, 72, 1, 56, 100, 0, 4, 70, 40, 250, 0},
	{1, 90, 72, 1, 56, 100, 0, 4, 70, 40, 250, 0},
	{0, 90, 72, 0, 56, 100, 1, 3, 70, 40, 250, 0},
	{1, 60, 512, 0, 64, 250, 0, 3, 300, 45, 250, 1},
	{3, 60, 72, 1, 64, 250, 0, 4, 70, 40, 250, 0},
	{2, 20, 72, 1, 80, 0, 0, 3, 70, 40, 40, 0},
};

struct MmttyRx {
	CFSKDEM dem;
	double HBPF[TAPMAX + 1];
	double ZBPF[TAPMAX + 1];
	int bpf;
	int bpftap;
	// CRTTY
	int fig;
	int uos;
	std::string out;

	MmttyRx(int profile, double baud, double shift, double centre, int squelch) : bpf(0), bpftap(56), fig(0), uos(1) {
		memset(HBPF, 0, sizeof(HBPF));
		memset(ZBPF, 0, sizeof(ZBPF));
		if (profile < 0 || profile >= (int)(sizeof(PROFILES) / sizeof(*PROFILES))) profile = 0;
		const Profile &p = PROFILES[profile];
		// TMmttyWd::LoadProfile, nello stesso ordine (UpdateLimit: ricezione, limitatore acceso con il guadagno del profilo)
		dem.m_Limit = 1;
		dem.m_LimitGain = 200.0;
		dem.m_LimitOverSampling = 0;
		dem.m_atc = p.atc;
		dem.m_atcMark.m_Max = 4;
		dem.m_atcSpace.m_Max = dem.m_atcMark.m_Max;
		dem.m_majority = 1;
		dem.m_ignoreFream = 0;
		dem.SetSQ(squelch);
		dem.SetSQLevel(600.0);
		// MMTTY: mark = tono basso; i nostri segnali hanno il mark alto, quindi Rev
		double mark = centre - shift / 2.0;
		double space = mark + shift;
		dem.SetSpaceFreq(space);
		dem.SetMarkFreq(mark);
		bpf = p.bpf;
		bpftap = p.bpftap;
		// TSound::CalcBPF
		MakeFilter(HBPF, bpftap, ffBPF, SampFreq, dem.GetMarkFreq() - p.bpffw, dem.GetSpaceFreq() + p.bpffw, 60, 1.0);
		dem.m_type = p.demtype;
		dem.SetBaudRate(baud);
		dem.m_lpf = 0;
		dem.m_lpfOrder = 5;
		dem.SetLPFFreq(p.smoozIIR);
		dem.SetSmoozFreq(p.smooz);
		dem.m_BitLen = 5;
		dem.m_StopLen = p.stoplen;
		dem.m_Parity = 0;
		dem.SetFilterTap(p.tap);
		dem.m_iirfw = p.iirbw;
		dem.SetIIR(dem.m_iirfw);
		dem.m_Phase.m_TONES = 4;
		dem.m_Phase.Create();
		dem.m_pll.SetVcoGain(3.0);
		dem.m_pll.m_loopOrder = 2;
		dem.m_pll.m_loopFC = p.pllLoopFC;
		dem.m_pll.m_outOrder = 4;
		dem.m_pll.m_outFC = 200.0;
		dem.m_pll.MakeLoopLPF();
		dem.m_pll.MakeOutLPF();
		CAA6YQ *a = &dem.m_AA6YQ;
		a->m_fEnabled = p.aa6yq;
		a->m_bpfTaps = 512;
		a->m_bpfFW = 35.0;
		a->m_befTaps = 256;
		a->m_befFW = 15.0;
		a->m_afcERR = 5.0;
		a->Create();
		dem.SetRev(1);
	}

	// CRTTY::ConvAscii (codice americano: sys.m_CodeSet = 0)
	char conv_ascii(int d) {
		char c = 0;
		d &= 0x1f;
		if (d == 0x1b) {
			fig = 1;
		} else if (d == 0x1f) {
			fig = 0;
		} else if (fig) {
			c = _FIG[d];
			if (uos && c == ' ') fig = 0;
		} else {
			c = _LTR[d];
		}
		return c;
	}

	void process(const float *x, size_t n) {
		for (size_t i = 0; i < n; i++) {
			double d = x[i] * 32768.0;
			// TSound: il passa-banda d'ingresso (RXBPF) prima del demodulatore
			if (bpf) d = DoFIR(HBPF, ZBPF, d, bpftap);
			dem.Do(d);
			while (dem.GetCount()) {
				int v = dem.GetData();
				if (v < 0) break;
				char c = conv_ascii(v);
				if (c) out.push_back(c);
			}
		}
	}
};

}  // namespace

extern "C" {

void *aurora_mmtty_rtty_new(int profile, double baud, double shift, double centre, int squelch) {
	return new MmttyRx(profile, baud, shift, centre, squelch);
}

void aurora_mmtty_rtty_process(void *rx, const float *samples, size_t len) {
	static_cast<MmttyRx *>(rx)->process(samples, len);
}

size_t aurora_mmtty_rtty_take(void *rx, unsigned char *buf, size_t cap) {
	MmttyRx *r = static_cast<MmttyRx *>(rx);
	size_t n = r->out.size() < cap ? r->out.size() : cap;
	memcpy(buf, r->out.data(), n);
	r->out.erase(0, n);
	return n;
}

double aurora_mmtty_rtty_level(void *rx) {
	return static_cast<MmttyRx *>(rx)->dem.m_avgdeff;
}

void aurora_mmtty_rtty_free(void *rx) {
	delete static_cast<MmttyRx *>(rx);
}

}
