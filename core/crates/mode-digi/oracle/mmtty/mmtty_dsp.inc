// Estratto da MMTTY 1.70L (terze-parti/mmtty, commit e2c1327) per il solo arbitro delle prove di Aurora: vedi mmtty_rtty_rx.cpp.
//Copyright+LGPL

//-----------------------------------------------------------------------------------------------------------------------------------------------
// Copyright 2000-2013 Makoto Mori, Nobuyuki Oba
//-----------------------------------------------------------------------------------------------------------------------------------------------
// This file is part of MMTTY.

// MMTTY is free software: you can redistribute it and/or modify it under the terms of the GNU Lesser General Public License
// as published by the Free Software Foundation, either version 3 of the License, or (at your option) any later version.

// MMTTY is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of 
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU Lesser General Public License for more details.

// You should have received a copy of the GNU Lesser General Public License along with MMTTY.  If not, see 
// <http://www.gnu.org/licenses/>.
//-----------------------------------------------------------------------------------------------------------------------------------------------

// ===== CLX.h, righe 26-180 =====
class CLX {
public:
	double	r;
	double	j;
	inline CLX(void){};
	inline CLX(const double R){
		r = R;
		j = 0;
	};
	inline CLX(const double R, const double J){
		r = R;
		j = J;
	};
	inline CLX(CLX &ref){
		r = ref.r;
		j = ref.j;
	};
	inline void Add(const CLX &ref){
		r += ref.r;
		j += ref.j;
	};
	inline void Add(const double &R){
		r += R;
	};
	inline void Sub(const CLX &ref){
		r -= ref.r;
		j -= ref.j;
	};
	inline void Sub(const double &R){
		r -= R;
	};
    inline double Abs(void){
		return sqrt(r * r + j * j);
    };
    inline double vAbs(void){
		return r * r + j * j;
    };
	inline void Mul(CLX &z, const CLX &ref){
		z.r = r * ref.r - j * ref.j;
		z.j = r * ref.j + j * ref.r;
	}
	inline void Mul(const CLX &ref){
		CLX z;
		z.r = r * ref.r - j * ref.j;
		z.j = r * ref.j + j * ref.r;
		r = z.r; j = z.j;
    };
	void Div(const CLX &ref);
    inline void Mul(const double &R){
		r *= R; j *= R;
    }
    inline void Div(const double &R){
		r /= R; j /= R;
    }
    CLX Sqrt(void);
	inline void PhDiff(CLX &z, const CLX &ref){
		z.r = r * ref.r + j * ref.j;
		z.j = j * ref.r - r * ref.j;

    };
    inline double Phase(void){
		if( r && j ){
			return atan2(j, r);
        }
        else {
			return 0;
        }
    };
    inline BOOL IsSame(const CLX &ref){
		return ((r == ref.r) && (j == ref.j));
    };
	inline CLX& operator=(const CLX &ref){
		if( this != &ref ){
			r = ref.r;
			j = ref.j;
		}
		return *this;
	};
	inline CLX& operator+=(const CLX &ref){
		Add(ref);
		return *this;
	};
	inline CLX& operator+=(const double R){
		Add(R);
		return *this;
	};
	inline CLX& operator-=(const CLX &ref){
		Sub(ref);
		return *this;
	};
	inline CLX& operator-=(const double R){
		Sub(R);
		return *this;
	};
	inline CLX& operator*=(const CLX &ref){
		Mul(ref);
		return *this;
	};
	inline CLX& operator*=(const double R){
		Mul(R);
		return *this;
	};
	inline CLX& operator/=(const CLX &ref){
		Div(ref);
		return *this;
	};
	inline CLX& operator/=(const double R){
		Div(R);
		return *this;
	};
	inline CLX operator+(const CLX &ref) const {
		CLX a(r, j);
		a+=ref;
		return a;
	};
	inline CLX operator+(const double R) const {
		CLX a(r, j);
		a+=R;
		return a;
	};
	inline CLX operator-(const CLX &ref) const {
		CLX a(r, j);
		a-=ref;
		return a;
	};
	inline CLX operator-(const double R) const {
		CLX a(r, j);
		a-=R;
		return a;
	};
	inline CLX operator*(const CLX &ref) const {
		CLX a(r, j);
		a*=ref;
		return a;
	};
	inline CLX operator*(const double R) const {
		CLX a(r, j);
		a*=R;
		return a;
	};
	inline CLX operator/(const CLX &ref) const {
		CLX a(r, j);
		a/=ref;
		return a;
	};
	inline CLX operator/(const double R) const {
		CLX a(r, j);
		a/=R;
		return a;
	};
	friend CLX operator+(const double R, const CLX &ref);
	friend CLX operator-(const double R, const CLX &ref);
	friend CLX operator*(const double R, const CLX &ref);
	friend CLX operator/(const double R, const CLX &ref);
};
// ===== CLX.cpp, righe 26-81 =====

///----------------------------------------------------------------
///	複素数演算クラス
void CLX::Div(const CLX &ref)
{
	double R, J, D;
	R = r * ref.r + j * ref.j;
	J = j * ref.r - r * ref.j;
	D = ref.r * ref.r + ref.j * ref.j;
    if( D ){
		r = R/D;
		j = J/D;
    }
    else {
		r = R >= 0 ? 1e100 : -1e100;
		j = J >= 0 ? 1e100 : -1e100;
    }
}

CLX CLX::Sqrt(void)
{
	CLX	a;

    double	R, ST;
    R = sqrt(sqrt(r*r + j*j));
	ST = 0;
    if( r ) ST = atan2(j, r)/2.0;
    a.r = R*cos(ST);
    a.j = R*sin(ST);
	return a;
}

CLX operator+(const double R, const CLX &ref)
{
	CLX a(R);
	a+=ref;
	return a;
}
CLX operator-(const double R, const CLX &ref)
{
	CLX a(R);
	a-=ref;
	return a;
}
CLX operator*(const double R, const CLX &ref)
{
	CLX a(R);
	a*=ref;
	return a;
}
CLX operator/(const double R, const CLX &ref)
{
	CLX a(R);
	a/=ref;
	return a;
}
// ===== fir.h, righe 27-319 =====
#define	TAPMAX	512

#define	PI	3.1415926535897932384626433832795

enum {
	ffLPF,
	ffHPF,
	ffBPF,
	ffBEF,
	ffUSER,
	ffLMS,
};

typedef struct {
	int		n;
	int		typ;
	double	fs;
	double	fcl;
	double	fch;
	double	att;
	double	gain;
	double	fc;
	double	hp[TAPMAX+1];		/* 係数配列		*/
}FIR;
void MakeFilter(double *HP, int tap, int type, double fs, double fcl, double fch, double att, double gain);
void MakeFilter(double *HP, FIR *fp);
void __fastcall MakeHilbert(double *H, int N, double fs, double fc1, double fc2);

double __fastcall DoFIR(double *hp, double *zp, double d, int tap);

class CDECM2{			// 36 Tap FIR 1/2 DECM
private:
	double	Z1[19];
	double	Z2[18];
	double	H[37];
public:
	CDECM2();
	double Do(double d1, double d2);
};

#if 0
class CDECM2H{			// 64 Tap FIR 1/2 DECM
private:
	double	Z1[33];
	double	Z2[32];
	double	H[65];
public:
	CDECM2H();
	double Do(double d1, double d2);
};
#endif

#if 0
class CINTP2{			// 36 Tap FIR x2 INTP
private:
	double	Z[19];
	double	H[37];
public:
	CINTP2();
	void Do(double &d1, double &d2, double d);
};

class CDECM3{			// 48 Tap FIR 1/3 DECM
private:
	double	Z1[17];
	double	Z2[16];
	double	Z3[16];
	double	H[49];
public:
	CDECM3();
	double Do(double d1, double d2, double d3);
};

class CINTP3{			// 48 Tap FIR x3 INTP
private:
	double	Z[17];
	double	H[49];
public:
	CINTP3();
	void Do(double &d1, double &d2, double &d3, double d);
};
#endif

#define	DELAYMAX	4096
class CLMS
{
private:
	double	*Z;					// FIR Z Application
	double	*D;					// LMS Delay;

	double	m_MarkFreq;			// for RTTY
	double	m_SpaceFreq;		// for RTTY
	double	HBPF[TAPMAX+1];		// for RTTY

	double	m_lmsADJSC;			// スケール調整値
	double	m_lmsErr;			// LMS 誤差データ
	double	m_lmsMErr;			// LMS 誤差データ（×２μ）

	void GetFW(double &fl, double &fh, double fq);
public:
	int		m_Type;
	int		m_Tap;
	int		m_NotchTap;			// Notch Taps
	int		m_lmsInv;			// LMS InvOutput
	int		m_lmsDelay;			// LMS Delay
	int		m_lmsAGC;			// LMS AGC
	double	m_lmsMU2;			// LMS 2μ
	double	m_lmsGM;			// LMS γ
	double	*H;					// アプリケーションフィルタの係数

	int		m_lmsNotch;			// ノッチ周波数
	int		m_lmsNotch2;		// ノッチ周波数
	int		m_twoNotch;			// ２つめのノッチの許可
	int		m_bpf;				// For RTTY
public:
	CLMS();
	~CLMS();

	void Copy(CLMS &other);
	double Do(double d);
	void SetWindow(double mfq, double sfq);
	inline double *GetHBPF(void){return HBPF;};
};

class CIIRTANK
{
private:
	double	z1, z2;
public:
	double	a0;
	double	b1, b2;
public:
	CIIRTANK();
	void SetFreq(double f, double smp, double bw);
	double Do(double d);
};

#define	IIRMAX	16
class CIIR
{
private:
	double	*Z;
public:
	double	*A;
	double	*B;
	int		m_order;
	int		m_bc;
	double	m_rp;
	CIIR();
	~CIIR();
	void MakeIIR(double fc, double fs, int order, int bc, double rp);
	double Do(double d);
	void Clear(void);
};

class CINTPXY2{			// IIR x2 INTP for XY Scope
private:
	CIIR	iir;
public:
	CINTPXY2();
	inline void Clear(void){iir.Clear();};
	void __fastcall Do(double *p, double d);
};

#if 0
class CINTPXY4{			// IIR x4 INTP for XY Scope
private:
	CIIR	iir;
public:
	CINTPXY4();
	inline void Clear(void){iir.Clear();};
	void __fastcall Do(double *p, double d);
};

class CINTPXY8{			// IIR x8 INTP for XY Scope
private:
	CIIR	iir;
public:
	CINTPXY8();
	inline void Clear(void){iir.Clear();};
	void __fastcall Do(double *p, double d);
};

class CINTPXY2FIR{			// 32 Tap FIR x2 INTP
private:
	double	Z[17];
	double	H[33];
public:
	CINTPXY2FIR();
	inline void Clear(void){memset(Z, 0, sizeof(Z));};
	void __fastcall Do(double *p, double d);
};
#endif

class CINTPXY4FIR{			// 64 Tap FIR x4 INTP
private:
	double	Z[17];
	double	H[65];
public:
	CINTPXY4FIR();
	inline void Clear(void){memset(Z, 0, sizeof(Z));};
	void __fastcall Do(double *p, double d);
};

class CINTPXY8FIR{			// 96 Tap FIR x8 INTP
private:
	double	Z[13];
	double	H[97];
public:
	CINTPXY8FIR();
	inline void Clear(void){memset(Z, 0, sizeof(Z));};
	void __fastcall Do(double *p, double d);
};

#define	OVERFIR	1
#if OVERFIR
class CDECM4{				// 80 Tap FIR 1/4 DECM
private:
	double	Z1[21];
	double	Z2[20];
	double	Z3[20];
	double	Z4[20];
	double	H[81];
public:
	CDECM4();
	double __fastcall Do(double *dp);
};

class CINTP4{			// 80 Tap FIR x4 INTP
private:
	double	Z[21];
	double	H[81];
public:
	CINTP4();
	inline void Clear(void){memset(Z, 0, sizeof(Z));};
	void __fastcall Do(double *p, double d);
};
#endif

class COVERLIMIT{
private:
#if OVERFIR
	CDECM4	decm;
	CINTP4	intp;
#else
	CIIR	iira;
	CIIR	iirb;
#endif
public:
	COVERLIMIT();
	double Do(double d, double lmt);
};

class CDECM2IIR{			// IIR 1/2 DECM
private:
	CIIR	iir;
public:
	CDECM2IIR();
	double Do(double d1, double d2);
};

#if 0
#define HILLTAP 48
class CHILL
{
public:
	double  Z[HILLTAP+1];
	double  H[HILLTAP+1];
	double  m_A[4];
	double  m_OFF;
	double  m_OUT;
	double  *m_ph;
	int     m_htap;
	int     m_tap;
	double	m_SampFreq;
	double	m_CenterFreq;
	double	m_MarkFreq;
	double	m_SpaceFreq;
public:
	CHILL();
	void SetSampFreq(double samp);
	void SetFreq(double mark, double space);
	void SetMarkFreq(double mark);
	void SetSpaceFreq(double space);

	double Do(double d);
};
#endif

// ===== Rtty.h, righe 40-756 =====
class CFAVG
{
private:
    double	m_Sum;
    double	m_Avg;
    double	m_Mul;
	int		m_Max;
    int		m_Cnt;
public:
	__fastcall CFAVG();
	void __fastcall Reset(void);
	void __fastcall Reset(double d);
    void __fastcall Create(int max);
	double __fastcall DoZ(double d);
	double __fastcall Do(double d);
    inline double __fastcall GetAvg(void){return m_Avg;};
    inline BOOL __fastcall IsFull(void){return m_Max == m_Cnt;};
    inline BOOL __fastcall IsHalf(void){return m_Max <= (m_Cnt*2);};
    inline int __fastcall GetCount(void){return m_Cnt;};
};
//--------------------------------------------------------
// CAGCクラス
// Added by JE3HHT on Aug.2010
class CAGCX
{
private:
	double  m_Max;
	double  m_Min;
	double  m_d;
	double  m_agc;

	double	m_fc;
	double	m_Gain;
    CIIR	m_LPF;
//    CIIR	m_Level;
    int		m_Count;
    int		m_TLimit;
    CFAVG	m_AvgOver;
public:
    double	m_LimitGain;
	double	m_MonitorFreq;
	double	m_CarrierFreq;
    double	m_SampleFreq;
public:
	__fastcall CAGCX();
    void __fastcall Create(void);
	inline void __fastcall SetSampleFreq(double f){m_SampleFreq = f; Create();};
	void __fastcall SetCarrierFreq(double f);
    void __fastcall SetFC(double fc);
    void __fastcall Reset(void);
    double __fastcall Do(double d);
	inline void __fastcall ResetOver(void){m_AvgOver.Reset(1.0);};
    BOOL __fastcall GetOver(void);
};

//---------------------------------------------------------------------------
class CSinTable
{
public:
	double	*m_tSin;
	int		m_Size;
public:
	__fastcall CSinTable();
    __fastcall ~CSinTable();
};

//---------------------------------------------------------------------------
class CAMPCONT
{
private:
	double	m_Max;
    double	m_Cnt;
    double	m_ADD;
    int		m_S;
    int		m_iMax;
public:
	__fastcall CAMPCONT();
	void __fastcall SetMax(int max);
    void __fastcall Reset(void);
    double __fastcall Do(int d);

    inline int __fastcall IsMax(void){return m_Cnt >= m_Max;};
};

//---------------------------------------------------------------------------
class CTICK
{
public:
	int		*ptbl[2];
	int		m_wsel;
	int		*m_wp;
	int		m_wcnt;
	int		m_Trig;
	int		m_Samp;
public:
	CTICK();
	~CTICK();
	void Init(void);
	void Write(double d);
	inline int GetTrig(void){return m_Trig;};
	int *GetData(void);
};


class CVCO
{
private:
	double	m_c1;	// VCOの利得
	double	m_c2;	// フリーランニング周波数
	int 	m_z;

	int		m_TableSize;
	double	*pSinTbl;
	double	m_FreeFreq;
	double	m_SampleFreq;
	int     m_vlock;
public:
	CVCO();
	~CVCO();
	void VirtualLock(void);
	void InitPhase(void);
	void SetGain(double gain);
	void SetSampleFreq(double f);
	void SetFreeFreq(double f);
	double Do(double d);
};

//---------------------------------------------------------------------------
// Added by JE3HHT on Aug.2010
class CVCOX
{
private:
	double	m_c1;	// VCOの利得
	double	m_c2;	// フリーランニング周波数
	double	m_z;

	double	m_FreeFreq;
	double	m_SampleFreq;
	int		m_TableSize;
    double	m_TableCOS;
public:
	__fastcall CVCOX();
	__fastcall ~CVCOX();
	void __fastcall InitPhase(void);
	void __fastcall SetGain(double gain);
	void __fastcall SetSampleFreq(double f);
	void __fastcall SetFreeFreq(double f);
	double __fastcall Do(void);
	double __fastcall Do(double d);
	double __fastcall DoCos(void);
};

class CSmooz{
private:
	double	*bp;
	int	Wp;
	int	Max;
	int	Cnt;
	inline void IncWp(void){
		Wp++;
		if( Wp >= Max ) Wp = 0;
	};
	inline double Avg(void){
		double	d = 0.0;
		int		i;
		for( i = 0; i < Cnt; i++ ){
			d += bp[i];
		}
		if( Cnt ){
			return d/double(Cnt);
		}
		else {
			return 0;
		}
	};
public:
	inline CSmooz(int max = 60){
		Max = max;
		bp = new double[max];
		Cnt = 0;
		Wp = 0;
	};
	inline ~CSmooz(void){
		delete[] bp;
	};
	inline void SetCount(int n){
		double *np = new double[n];
		Max = n;
		Cnt = Wp = 0;
		double *op = bp;
		bp = np;
		delete[] op;	//JA7UDE 0428
		//delete[] np;	//JA7UDE 0428
	};
	inline double Avg(double d){
		bp[Wp] = d;
		IncWp();
		if( Cnt < Max ){
			Cnt++;
		}
		return Avg();
	};
};

//---------------------------------------------------------------------------
// Added by JE3HHT on Aug.2010
class CFIR
{
private:
	int		m_Tap;
	double	*m_pZ;
    double	*m_pH;
public:
	__fastcall CFIR();
    __fastcall ~CFIR();
	void __fastcall Create(int tap, int type, double fs, double fcl, double fch, double att, double gain);
	double __fastcall Do(double d);
    void __fastcall SaveCoef(LPCSTR pName);
    inline double __fastcall GetHD(int n){return m_pH[n];};
    inline double *__fastcall GetHP(void){return m_pH;};
    inline int __fastcall GetTap(void){return m_Tap;};
};

//---------------------------------------------------------------------------
// ダブルバッファによるFIRフィルタ
// Added by JE3HHT on Aug.2010
class CFIR2
{
private:
	int		m_Tap;
    int		m_TapHalf;
	double	*m_pZ;
    double	*m_pH;
	double	*m_pZP;

    int		m_W;
    double	m_fs;
public:
	__fastcall CFIR2();
    __fastcall ~CFIR2();
	void __fastcall Delete(void);
	void __fastcall Clear(void);
	void __fastcall Create(int tap, int type, double fs, double fcl, double fch, double att, double gain);
	void __fastcall Create(int tap, double fs, double fcl, double fch);
	void __fastcall CreateSamp(int tap, double fs, const double *pSmpFQ);
	double __fastcall Do(double d);
	double __fastcall Do(double *hp);
	void __fastcall Do(CLX &z, double d);

    inline double __fastcall GetHD(int n){return m_pH[n];};
    inline double *__fastcall GetHP(void){return m_pH;};
    inline int __fastcall GetTap(void){return m_Tap;};
    inline double __fastcall GetSampleFreq(void){return m_fs;};
};
//---------------------------------------------------------------------------
// ダブルバッファによるFIRフィルタ(複素数用)
// Added by JE3HHT on Aug.2010
class CFIRX
{
private:
	int		m_Tap;
    int		m_TapHalf;
	CLX		*m_pZ;
    double	*m_pH;
	CLX		*m_pZP;

    int		m_W;
    double	m_fs;
public:
	__fastcall CFIRX();
    __fastcall ~CFIRX();
	void __fastcall Clear(void);
	void __fastcall Create(int tap, int type, double fs, double fcl, double fch, double att, double gain);
	void __fastcall Do(CLX &d);

    inline double __fastcall GetHD(int n){return m_pH[n];};
    inline double *__fastcall GetHP(void){return m_pH;};
    inline int __fastcall GetTap(void){return m_Tap;};
    inline double __fastcall GetSampleFreq(void){return m_fs;};
};
//---------------------------------------------------------------------------
// スライディングFFT
// Added by JE3HHT on Aug.2010
class CSlideFFT
{
private:
	int 	m_Length;
	int 	m_Base;
    int		m_Tones;

	CLX		m_tData[16];
	CLX 	m_tWindow[16];

    CLX		*m_pCur;
    CLX		*m_pEnd;
	CLX		*m_pBase;

	double	m_kWindow;
private:
public:
	__fastcall CSlideFFT();
	__fastcall ~CSlideFFT();

	void __fastcall Create(int len, int base, int tones);
	CLX* __fastcall Do(const CLX &zIn);
};

/*=============================================================================
  CPHASEクラス
=============================================================================*/
#define	CPHASE_BASEFREQ	0.0
class CPHASE
{
private:
//	CFIRX	m_LPF;
	CVCOX	m_VCO;
    CFIR2	m_Hilbert;
    CAGCX	m_AGC;

    FILE	*m_fp;
public:
    CLX		m_sig;
	CSlideFFT	m_SlideFFT;

    int			m_TONES;
	double		m_SHIFT;
    double		m_fftSHIFT;
	int			m_BASEPOINT;

	double	m_MixerFreq;
    double	m_SymbolLen;
	double	m_CarrierFreq;
	double	m_SampleFreq;

    double	m_dm;
    double	m_ds;
private:
public:
	__fastcall CPHASE();
    void __fastcall Create(void);
    void __fastcall SetSampleFreq(double f);
	void __fastcall SetCarrierFreq(double f);
	inline void __fastcall AddCarrierFreq(double f){
		SetCarrierFreq(m_CarrierFreq + f);
    };
	CLX* __fastcall Do(double d);
    inline CFIR2* __fastcall GetFIR(void){return &m_Hilbert;};

	void __fastcall DoFSK(double d);
    void __fastcall SetShift(double f);
	void __fastcall ShowPara(void);

    inline void SetTones(int n){
		m_TONES = n;
        Create();
    }
};

class CPLL
{
private:
	CIIR	loopLPF;
	CIIR	outLPF;

	double	m_err;
	double	m_out;
	double	m_vcoout;
	double	m_SampleFreq;
	double	m_FreeFreq;
	double	m_Shift;
public:
	int		m_loopOrder;
	double	m_loopFC;
	int		m_outOrder;
	double	m_outFC;
	double	m_vcogain;
public:
	CVCO	vco;
	inline CPLL(){
		m_err = 0;
		m_out = 0;
		m_vcoout = 0;
		m_vcogain = 3.0;
		m_SampleFreq = SampFreq * 0.5;
		m_Shift = 170.0;
		m_FreeFreq = (2125 + 2295)/2;
		m_loopOrder = 2;
		m_outOrder = 4;
		m_loopFC = 250.0;
		m_outFC = 200.0;
		SetFreeFreq(2125, 2295);
		SetSampleFreq(m_SampleFreq);
	};
	inline void SetVcoGain(double g){
		m_vcogain = g;
		vco.SetGain(-m_Shift * g);
	};
	inline void MakeLoopLPF(void){
		loopLPF.MakeIIR(m_loopFC, m_SampleFreq, m_loopOrder, 0, 0);
	}
	inline void MakeOutLPF(void){
		outLPF.MakeIIR(m_outFC, m_SampleFreq, m_outOrder, 0, 0);
	}
	inline void SetFreeFreq(double f1, double f2){
		m_FreeFreq = (f1 + f2)/2.0;
		m_Shift = f2 - f1;
		vco.SetFreeFreq(m_FreeFreq);
		vco.SetGain(-m_Shift * m_vcogain);
	}
	void SetSampleFreq(double f){
		m_SampleFreq = f;
		vco.SetSampleFreq(f);
		vco.SetFreeFreq(m_FreeFreq);
		SetVcoGain(3.0);
		MakeLoopLPF();
		MakeOutLPF();
	};
	double Do(double d){
		d /= 32768.0;
		// Loop Filter
		m_out = loopLPF.Do(m_err);
		// VCO
		m_vcoout = vco.Do(m_out);
		// 位相比較
		m_err = m_vcoout * d;
		return outLPF.Do(m_out) * 32768.0;
	};
	inline double GetErr(void){return m_err*32768;};	// Phase Det
	inline double GetOut(void){return m_out*32768;};	// Phase Det
	inline double GetVco(void){return m_vcoout;};
};


#define	SCOPESIZE	8192
class CScope
{
private:
	int		m_DataFlag;

	int		m_wp;

public:
	int		m_ScopeSize;
	double	*pScopeData;

public:
	CScope();
	~CScope();
	void WriteData(double d);
	void UpdateData(double d);
	inline int GetFlag(void){return m_DataFlag;};	//JA7UDE 0428, edited by JA7UDE (Jan 17, 2016)
	void Collect(int size);
};

#define	SAMPLEPEAKMAX	16
class CSamplePeak
{
private:
	double	Strage[SAMPLEPEAKMAX];
	double	m_CurPeak;
	double	m_Peak;

	int		m_Strage;
	int		m_Count;
	int		m_ReCount;
public:
	CSamplePeak();

	inline int GetStrage(void){ return m_Strage + 1;};
	inline void SetStrage(int n){
		if( n > SAMPLEPEAKMAX ) n = SAMPLEPEAKMAX;
		m_Strage = n ? (n - 1) : 0;
	};
	double GetPeak(void){return m_Peak;};
	void SetBaudRate(double b);
	void Sync(int Delay);
	int Sample(double d);
};

class CAGC
{
private:
	double	m_MarkGain;
	double	m_SpaceGain;
public:
	int		m_Sync;
	double	m_StepGain;
	double	m_MaxGain;
	double	m_DeffGain;
	CSamplePeak	Mark;
	CSamplePeak	Space;
public:
	CAGC();
	void Sync(int Delay);
	double SampleMark(double d);
	double SampleSpace(double d);

};

#define	ATCMAX	17
#define ATCC	(8192.0)
#define	ATCW	(1024.0)
class CATC
{
private:
	double	m_Low;
	double	m_High;
	double	m_CurLow;
	double	m_CurHigh;
	double	m_LowList[ATCMAX];
	double	m_HighList[ATCMAX];
	int		m_Cnt;
	CIIR	m_iir;
public:
	int		m_Max;
public:
	CATC();
	double Do(double d);
};

/********************************************************
CAA6YQ class by JE3HHT on Sep.2010
*********************************************************/
class CAA6YQ {		// Added by JE3HHT on Sep.2010
public:
	BOOL	m_fEnabled;
    int		m_bpfTaps;
    int		m_befTaps;
	double	m_bpfFW;
    double	m_befFW;
	double	m_afcERR;

    double	m_dblMark;
    double	m_dblSpace;
	double	m_dblMarkAFC;
    double	m_dblSpaceAFC;

	CFIR2	m_BPF;
    CFIR2	m_BEF;
private:
public:
	__fastcall CAA6YQ(void);
	void __fastcall Create(void);
	void __fastcall SetMarkFreq(double f);
	void __fastcall SetSpaceFreq(double f);
	void __fastcall SetMarkFreqByAFC(double f);
	void __fastcall SetSpaceFreqByAFC(double f);
	double __fastcall Do(double d);
};

#define	DEMBUFMAX	512
#define	DEMBPFTAP	36
class CFSKDEM
{
private:
	double	HMark[TAPMAX+1];
	double	ZMark[TAPMAX+1];
	double	HSpace[TAPMAX+1];
	double	ZSpace[TAPMAX+1];

	int		Count;		// デモジュレータディスパッチ
	double	i2;			// デモジュレータ2ndInput

	CDECM2	DECM2;		// デモジュレータ

	int		m_Smooz;
	CSmooz	avgMark;
	CSmooz	avgSpace;

	double	m_BaudRate;
	int		m_Count;
	int		m_ReCount;

	int		m_mode;
	int		m_inv;
	int		m_sq;
	double	m_SQLevel;

	int		m_Tap;
	double	m_FilWidth;
	double	m_MarkFreq;
	double	m_SpaceFreq;
	double	m_SetMarkFreq;
	double	m_SetSpaceFreq;
	double	m_AFCMarkFreq;
	double	m_AFCSpaceFreq;

	int		m_DataCount;
	BYTE	m_Data;

	int		m_BufCount;
	BYTE	m_Buff[DEMBUFMAX];
	int		m_wp;
	int		m_rp;

	double	m_dMark;
	double	m_dSpace;
	int		m_mark;
	int		m_space;

	double	m_SmoozFreq;

	int		m_SumParity;

	CIIRTANK	m_iirm;
	CIIRTANK	m_iirs;

	int		m_sqcount;
	CSmooz	SmoozSQ;

	COVERLIMIT	OverLimit;
public:
	int		m_type;
	double	m_iirfw;

	int 	m_OverFlow;
//	int		m_Bit;
	int		m_Scope;
	int		m_XYScope;

	int		m_BitLen;
	int		m_StopLen;
	int		m_Parity;

	int		m_majority;		// 多数決ロジック
	int		m_ignoreFream;	// フレーミングエラーの無視

	int		m_Limit;
	int		m_LimitAGC;
	double	m_LimitGain;

	double	m_limitMax;
	double	m_limitMin;
	double	m_d;
	double	m_limitagc;

	double	m_deff;
	double	m_avgdeff;

	CScope	m_XYScopeMark;
	CScope	m_XYScopeSpace;
	CScope	m_ScopeMark[4];
	CScope	m_ScopeSpace[4];
	CScope	m_ScopeSync;
	CScope	m_ScopeBit;

	CPLL	m_pll;

	int		m_lpf;
	int		m_lpfOrder;
	double	m_lpffreq;
	CIIR	LpfMark;
	CIIR	LpfSpace;

	int		m_LimitOverSampling;
	int		m_Tick;
	CTICK	Tick;
	int		m_atc;
	CATC	m_atcMark;
	CATC	m_atcSpace;
	CATC	m_atcPLL;

    CPHASE	m_Phase;
	CAA6YQ	m_AA6YQ;
#if BITDEBUG
	int		m_bitCount;
	int		m_bitCountA;
#endif
private:
	void DoFSK(void);


public:
	CFSKDEM();
	void Do(double d);

	inline void ClearMode(void){m_mode = 0;};
	double GetFilWidth(int tap);

	void SetIIR(double b);

	void AFCMarkFreq(double d);
	void AFCSpaceFreq(double d);
	void SetMarkFreq(double d);
	void SetSpaceFreq(double d);
	inline double GetMarkFreq(void){return m_MarkFreq;};
	inline double GetSpaceFreq(void){return m_SpaceFreq;};
	inline double GetSetMarkFreq(void){return m_SetMarkFreq;};
	inline double GetSetSpaceFreq(void){return m_SetSpaceFreq;};
	inline double GetAFCMarkFreq(void){return m_AFCMarkFreq;};
	inline double GetAFCSpaceFreq(void){return m_AFCSpaceFreq;};
	inline int GetRev(void){ return m_inv; };
	inline void SetRev(int inv){ m_inv = inv;};
	void SetBaudRate(double b);
	inline double GetBaudRate(void){return m_BaudRate;};
	inline double GetSmoozFreq(void){return m_SmoozFreq;};
	inline int GetSmoozCount(void){return m_Smooz;};
	void SetSmoozCount(int n);
	void SetSmoozFreq(double f);
	void SetLPFFreq(double f);
	inline int GetSQ(void){return m_sq;};
	inline void SetSQ(int sq){m_sq = sq;};
	inline double GetSQLevel(void){return m_SQLevel;};
	inline void SetSQLevel(double lvl){m_SQLevel = lvl;};

	inline int GetFilterTap(void){return m_Tap;};
	void SetFilterTap(int tap);
	inline int GetCount(void){return m_BufCount;};
	int GetData();
	inline void ClearRXBuf(void){
		m_BufCount = 0;
		m_rp = m_wp = 0;
		m_mode = 0;
	};
	void WriteData(BYTE d);
};

// ===== fir.cpp, righe 29-75 =====
double __fastcall DoFIR(double *hp, double *zp, double d, int tap)
{
	memcpy(zp, &zp[1], sizeof(double)*tap);
	zp[tap] = d;
	d = 0.0;
	for( int i = 0; i <= tap; i++, hp++, zp++ ){
		d += (*zp) * (*hp);
	}
	return d;
}
//---------------------------------------------------------------------------
CIIRTANK::CIIRTANK()
{
	b1 = b2 = a0 = z1 = z2 = 0;
	SetFreq(2000.0, SampFreq, 50.0);
}
//---------------------------------------------------------------------------
void CIIRTANK::SetFreq(double f, double smp, double bw)
{
	double lb1, lb2, la0;
	lb1 = 2 * exp(-PI * bw/smp) * cos(2 * PI * f / smp);
	lb2 = -exp(-2*PI*bw/smp);
	if( bw ){
#if 0
		const double _gt[]={18.0, 26.0, 20.0, 20.0};
		la0 = sin(2 * PI * f/smp) / (_gt[SampType] * 50 / bw);
#else
		la0 = sin(2 * PI * f/smp) / ((smp/6.0) / bw);
#endif
	}
	else {
		la0 = sin(2 * PI * f/smp);
	}
	b1 = lb1; b2 = lb2; a0 = la0;
}
//---------------------------------------------------------------------------
double CIIRTANK::Do(double d)
{
	d *= a0;
	d += (z1 * b1);
	d += (z2 * b2);
	z2 = z1;
	if( fabs(d) < 1e-37 ) d = 0.0;
	z1 = d;
	return d;
}
//---------------------------------------------------------------------------
// ===== fir.cpp, righe 261-315 =====
CDECM2::CDECM2()
{
	memset(Z1, 0, sizeof(Z1));
	memset(Z2, 0, sizeof(Z2));
	::MakeFilter(H, 36, ffLPF, SampFreq, 2900, 3000, 70, 1.0);
}

double CDECM2::Do(double d1, double d2)
{
	memcpy(Z1, &Z1[1], sizeof(double)*18);
	memcpy(Z2, &Z2[1], sizeof(double)*17);
	Z1[18] = d1;
	Z2[17] = d2;

	double a;
	a = Z1[0] * H[36];
	a += Z2[0] * H[35];
	a += Z1[1] * H[34];
	a += Z2[1] * H[33];
	a += Z1[2] * H[32];
	a += Z2[2] * H[31];
	a += Z1[3] * H[30];
	a += Z2[3] * H[29];
	a += Z1[4] * H[28];
	a += Z2[4] * H[27];
	a += Z1[5] * H[26];
	a += Z2[5] * H[25];
	a += Z1[6] * H[24];
	a += Z2[6] * H[23];
	a += Z1[7] * H[22];
	a += Z2[7] * H[21];
	a += Z1[8] * H[20];
	a += Z2[8] * H[19];
	a += Z1[9] * H[18];
	a += Z2[9] * H[17];
	a += Z1[10] * H[16];
	a += Z2[10] * H[15];
	a += Z1[11] * H[14];
	a += Z2[11] * H[13];
	a += Z1[12] * H[12];
	a += Z2[12] * H[11];
	a += Z1[13] * H[10];
	a += Z2[13] * H[9];
	a += Z1[14] * H[8];
	a += Z2[14] * H[7];
	a += Z1[15] * H[6];
	a += Z2[15] * H[5];
	a += Z1[16] * H[4];
	a += Z2[16] * H[3];
	a += Z1[17] * H[2];
	a += Z2[17] * H[1];
	a += Z1[18] * H[0];
	return a;
}

// ===== fir.cpp, righe 605-775 =====
static double I0(double x)
{
	double	sum, xj;
	int		j;

	sum = 1.0;
	xj = 1.0;
	j = 1;
	while(1){
		xj *= ((0.5 * x) / (double)j);
		sum += (xj*xj);
		j++;
		if( ((0.00000001 * sum) - (xj*xj)) > 0 ) break;
	}
	return(sum);
}

/*
====================================================
	ＦＩＲフィルタの設計
====================================================
*/
void MakeFilter(double *HP, int tap, int type, double fs, double fcl, double fch, double att, double gain)
{
	FIR	fir;

	fir.typ = type;
	fir.n = tap;
	fir.fs = fs;
	fir.fcl = fcl;
	fir.fch = fch;
	fir.att = att;
	fir.gain = gain;
	MakeFilter(HP, &fir);
}

void MakeFilter(double *HP, FIR *fp)
{
	int		j, m;
	double	alpha, win, fm, w0, sum;
	double	*hp;

	if( fp->typ == ffHPF ){
		fp->fc = 0.5*fp->fs - fp->fcl;
	}
	else if( fp->typ != ffLPF ){
		fp->fc = (fp->fch - fp->fcl)/2.0;
	}
	else {
		fp->fc = fp->fcl;
	}
	if( fp->att >= 50.0 ){
		alpha = 0.1102 * (fp->att - 8.7);
	}
	else if( fp->att >= 21 ){
		alpha = (0.5842 * pow(fp->att - 21.0, 0.4)) + (0.07886 * (fp->att - 21.0));
	}
	else {
		alpha = 0.0;
	}

	hp = fp->hp;
	sum = PI*2.0*fp->fc/fp->fs;
	if( fp->att >= 21 ){		// インパルス応答と窓関数を計算
		for( j = 0; j <= (fp->n/2); j++, hp++ ){
			fm = (double)(2 * j)/(double)fp->n;
			win = I0(alpha * sqrt(1.0-(fm*fm)))/I0(alpha);
			if( !j ){
				*hp = fp->fc * 2.0/fp->fs;
			}
			else {
				*hp = (1.0/(PI*(double)j))*sin((double)j*sum)*win;
			}
		}
	}
	else {						// インパルス応答のみ計算
		for( j = 0; j <= (fp->n/2); j++, hp++ ){
			if( !j ){
				*hp = fp->fc * 2.0/fp->fs;
			}
			else {
				*hp = (1.0/(PI*(double)j))*sin((double)j*sum);
			}
		}
	}
	hp = fp->hp;
	sum = *hp++;
	for( j = 1; j <= (fp->n/2); j++, hp++ ) sum += 2.0 * (*hp);
	hp = fp->hp;
	if( sum > 0.0 ){
		for( j = 0; j <= (fp->n/2); j++, hp++ ) (*hp) /= sum;
	}

	// 周波数変換

	if( fp->typ == ffHPF ){
		hp = fp->hp;
		for( j = 0; j <= (fp->n/2); j++, hp++ ) (*hp) *= cos((double)j*PI);
	}
	else if( fp->typ != ffLPF ){
		w0 = PI * (fp->fcl + fp->fch) / fp->fs;
		if( fp->typ == ffBPF ){
			hp = fp->hp;
			for( j = 0; j <= (fp->n/2); j++, hp++ ) (*hp) *= 2.0*cos((double)j*w0);
		}
		else {
			hp = fp->hp;
			*hp = 1.0 - (2.0 * (*hp));
			for( hp++, j = 1; j <= (fp->n/2); j++, hp++ ) (*hp) *= -2.0*cos((double)j*w0);
		}
	}
	for( m = fp->n/2, hp = &fp->hp[m], j = m; j >= 0; j--, hp-- ){
		*HP++ = (*hp) * fp->gain;
	}
	for( hp = &fp->hp[1], j = 1; j <= (fp->n/2); j++, hp++ ){
		*HP++ = (*hp) * fp->gain;
	}
}

//---------------------------------------------------------------------------
//ＦＩＲフィルタ（ヒルベルト変換フィルタ）の設計
// Added by JE3HHT on Aug.2010
void __fastcall MakeHilbert(double *H, int N, double fs, double fc1, double fc2)
{
	int L = N / 2;
    double T = 1.0 / fs;

	double W1 = 2 * PI * fc1;
	double W2 = 2 * PI * fc2;

	// 2*fc2*T*SA((n-L)*W2*T) - 2*fc1*T*SA((n-L)*W1*T)

	double w;
	int n;
	double x1, x2;
	for( n = 0; n <= N; n++ ){
		if( n == L ){
			x1 = x2 = 0.0;
		}
		else if( (n - L) ){
			x1 = ((n - L) * W1 * T);
			x1 = cos(x1) / x1;
			x2 = ((n - L) * W2 * T);
			x2 = cos(x2) / x2;
		}
		else {
			x1 = x2 = 1.0;
		}
		w = 0.54 - 0.46 * cos(2*PI*n/(N));
		H[n] = -(2 * fc2 * T * x2 - 2 * fc1 * T * x1) * w;
	}

	if( N < 8 ){
		w = 0;
    	for( n = 0; n <= N; n++ ){
			w += fabs(H[n]);
    	}
		if( w ){
			w = 1.0 / w;
			for( n = 0; n <= N; n++ ){
				H[n] *= w;
			}
    	}
	}
}

//---------------------------------------------------------------------
// 周波数特性グラフ（フィルタスレッド内でコールしてはいけない）
//
//	H(ejωT) = [Σ0]Hm cos(mωT) - j[Σ1]Hm sin(mωT)
//
// ===== fir.cpp, righe 1402-1520 =====
double asinh(double x)
{
	return log(x + sqrt(x*x+1.0));
}

//------------------------------------------------------------------
// bc : 0-バターワース, 1-チェビシフ
// rp : 通過域のリップル
void MakeIIR(double *A, double *B, double fc, double fs, int order, int bc, double rp)
{
	double	w0, wa, u, zt, x;
	int		j, n;

	if( bc ){		// チェビシフ
		u = 1.0/double(order)*asinh(1.0/sqrt(pow(10.0,0.1*rp)-1.0));
	}
	wa = tan(PI*fc/fs);
	w0 = 1.0;
	n = (order & 1) + 1;
	double *pA = A;
	double *pB = B;
	double d1, d2;
	for( j = 1; j <= order/2; j++, pA+=3, pB+=2 ){
		if( bc ){	// チェビシフ
			d1 = sinh(u)*cos(n*PI/(2*order));
			d2 = cosh(u)*sin(n*PI/(2*order));
			w0 = sqrt(d1 * d1 + d2 * d2);
			zt = sinh(u)*cos(n*PI/(2*order))/w0;
		}
		else {		// バターワース
			w0 = 1.0;
			zt = cos(n*PI/(2*order));
		}
		pA[0] = 1 + wa*w0*2*zt + wa*w0*wa*w0;
		pA[1] = -2 * (wa*w0*wa*w0 - 1)/pA[0];
		pA[2] = -(1.0 - wa*w0*2*zt + wa*w0*wa*w0)/pA[0];
		pB[0] = wa*w0*wa*w0 / pA[0];
		pB[1] = 2*pB[0];
		n += 2;
	}
	if( bc && !(order & 1) ){
		x = pow( 1.0/pow(10.0,rp/20.0), 1/double(order/2) );
		pB = B;
		for( j = 1; j <= order/2; j++, pB+=2 ){
			pB[0] *= x;
			pB[1] *= x;
		}
	}
	if( order & 1 ){
		if( bc ) w0 = sinh(u);
		j = (order / 2);
		pA = A + (j*3);
		pB = B + (j*2);
		pA[0] = 1 + wa*w0;
		pA[1] = -(wa*w0 - 1)/pA[0];
		pB[0] = wa*w0/pA[0];
		pB[1] = pB[0];
	}
}

//---------------------------------------------------------------------------
CIIR::CIIR()
{
	m_order = 0;
	A = new double[IIRMAX*3];
	B = new double[IIRMAX*2];
	Z = new double[IIRMAX*2];
	memset(A, 0, sizeof(double[IIRMAX*3]));
	memset(B, 0, sizeof(double[IIRMAX*2]));
	memset(Z, 0, sizeof(double[IIRMAX*2]));
}

CIIR::~CIIR()
{
	if( A != NULL ) delete[] A;
	if( B != NULL ) delete[] B;
	if( Z != NULL ) delete[] Z;
}

void CIIR::Clear(void)
{
	memset(Z, 0, sizeof(double[IIRMAX*2]));
}

void CIIR::MakeIIR(double fc, double fs, int order, int bc, double rp)
{
	m_order = order;
	m_bc = bc;
	m_rp = rp;
	::MakeIIR(A, B, fc, fs, order, bc, rp);
}

double CIIR::Do(double d)
{
	double *pA = A;
	double *pB = B;
	double *pZ = Z;
	double o;
	for( int i = 0; i < m_order/2; i++, pA+=3, pB+=2, pZ+=2 ){
		d += pZ[0] * pA[1] + pZ[1] * pA[2];
		o = d * pB[0] + pZ[0] * pB[1] + pZ[1] * pB[0];
		pZ[1] = pZ[0];
		if( fabs(d) < 1e-37 ) d = 0.0;
		pZ[0] = d;
		d = o;
	}
	if( m_order & 1 ){
		d += pZ[0] * pA[1];
		o = d * pB[0] + pZ[0] * pB[0];
		if( fabs(d) < 1e-37 ) d = 0.0;
		pZ[0] = d;
		d = o;
	}
	return d;
}

//*********************************************************************
// CINTPXY	XYScope 用 ×2インタポーレータ処理クラス
//
// ===== fir.cpp, righe 1825-2104 =====
#if OVERFIR
//*********************************************************************
// CDECM4	1/4デシメータ処理クラス
//
CDECM4::CDECM4()
{
	const double _mt[]={1.11, 1.25, 1.40, 1.11};
	memset(Z1, 0, sizeof(Z1));
	memset(Z2, 0, sizeof(Z2));
	memset(Z3, 0, sizeof(Z3));
	memset(Z4, 0, sizeof(Z4));
	::MakeFilter(H, 80, ffLPF, SampFreq*4, 2800, 3000, 50, _mt[SampType]);
}

double __fastcall CDECM4::Do(double *dp)
{
	memcpy(Z1, &Z1[1], sizeof(double)*20);
	memcpy(Z2, &Z2[1], sizeof(double)*19);
	memcpy(Z3, &Z3[1], sizeof(double)*19);
	memcpy(Z4, &Z4[1], sizeof(double)*19);
#if 1
	Z4[19] = *dp++;
	Z3[19] = *dp++;
	Z2[19] = *dp++;
	Z1[20] = *dp;
#else
	Z1[20] = *dp++;
	Z2[19] = *dp++;
	Z3[19] = *dp++;
	Z4[19] = *dp;
#endif
	double a;
	a = Z1[0] * H[80];
	a += Z4[0] * H[79];
	a += Z3[0] * H[78];
	a += Z2[0] * H[77];
	a += Z1[1] * H[76];
	a += Z4[1] * H[75];
	a += Z3[1] * H[74];
	a += Z2[1] * H[73];
	a += Z1[2] * H[72];
	a += Z4[2] * H[71];
	a += Z3[2] * H[70];
	a += Z2[2] * H[69];
	a += Z1[3] * H[68];
	a += Z4[3] * H[67];
	a += Z3[3] * H[66];
	a += Z2[3] * H[65];
	a += Z1[4] * H[64];
	a += Z4[4] * H[63];
	a += Z3[4] * H[62];
	a += Z2[4] * H[61];
	a += Z1[5] * H[60];
	a += Z4[5] * H[59];
	a += Z3[5] * H[58];
	a += Z2[5] * H[57];
	a += Z1[6] * H[56];
	a += Z4[6] * H[55];
	a += Z3[6] * H[54];
	a += Z2[6] * H[53];
	a += Z1[7] * H[52];
	a += Z4[7] * H[51];
	a += Z3[7] * H[50];
	a += Z2[7] * H[49];
	a += Z1[8] * H[48];
	a += Z4[8] * H[47];
	a += Z3[8] * H[46];
	a += Z2[8] * H[45];
	a += Z1[9] * H[44];
	a += Z4[9] * H[43];
	a += Z3[9] * H[42];
	a += Z2[9] * H[41];
	a += Z1[10] * H[40];
	a += Z4[10] * H[39];
	a += Z3[10] * H[38];
	a += Z2[10] * H[37];
	a += Z1[11] * H[36];
	a += Z4[11] * H[35];
	a += Z3[11] * H[34];
	a += Z2[11] * H[33];
	a += Z1[12] * H[32];
	a += Z4[12] * H[31];
	a += Z3[12] * H[30];
	a += Z2[12] * H[29];
	a += Z1[13] * H[28];
	a += Z4[13] * H[27];
	a += Z3[13] * H[26];
	a += Z2[13] * H[25];
	a += Z1[14] * H[24];
	a += Z4[14] * H[23];
	a += Z3[14] * H[22];
	a += Z2[14] * H[21];
	a += Z1[15] * H[20];
	a += Z4[15] * H[19];
	a += Z3[15] * H[18];
	a += Z2[15] * H[17];
	a += Z1[16] * H[16];
	a += Z4[16] * H[15];
	a += Z3[16] * H[14];
	a += Z2[16] * H[13];
	a += Z1[17] * H[12];
	a += Z4[17] * H[11];
	a += Z3[17] * H[10];
	a += Z2[17] * H[9];
	a += Z1[18] * H[8];
	a += Z4[18] * H[7];
	a += Z3[18] * H[6];
	a += Z2[18] * H[5];
	a += Z1[19] * H[4];
	a += Z4[19] * H[3];
	a += Z3[19] * H[2];
	a += Z2[19] * H[1];
	a += Z1[20] * H[0];
	return a;
}

//*********************************************************************
// CINTP4	x4インタポーレータ処理クラス
//
CINTP4::CINTP4()
{
	memset(Z, 0, sizeof(Z));
	::MakeFilter(H, 80, ffLPF, SampFreq*4, 2800, 3000, 50, 4.0);
}

void __fastcall CINTP4::Do(double *dp, double d)
{
	memcpy(Z, &Z[1], sizeof(double)*20);
	Z[20] = d;

	dp[0] = Z[0] * H[80];
	dp[1] = Z[1] * H[79];
	dp[2] = Z[1] * H[78];
	dp[3] = Z[1] * H[77];
	dp[0] += Z[1] * H[76];
	dp[1] += Z[2] * H[75];
	dp[2] += Z[2] * H[74];
	dp[3] += Z[2] * H[73];
	dp[0] += Z[2] * H[72];
	dp[1] += Z[3] * H[71];
	dp[2] += Z[3] * H[70];
	dp[3] += Z[3] * H[69];
	dp[0] += Z[3] * H[68];
	dp[1] += Z[4] * H[67];
	dp[2] += Z[4] * H[66];
	dp[3] += Z[4] * H[65];
	dp[0] += Z[4] * H[64];
	dp[1] += Z[5] * H[63];
	dp[2] += Z[5] * H[62];
	dp[3] += Z[5] * H[61];
	dp[0] += Z[5] * H[60];
	dp[1] += Z[6] * H[59];
	dp[2] += Z[6] * H[58];
	dp[3] += Z[6] * H[57];
	dp[0] += Z[6] * H[56];
	dp[1] += Z[7] * H[55];
	dp[2] += Z[7] * H[54];
	dp[3] += Z[7] * H[53];
	dp[0] += Z[7] * H[52];
	dp[1] += Z[8] * H[51];
	dp[2] += Z[8] * H[50];
	dp[3] += Z[8] * H[49];
	dp[0] += Z[8] * H[48];
	dp[1] += Z[9] * H[47];
	dp[2] += Z[9] * H[46];
	dp[3] += Z[9] * H[45];
	dp[0] += Z[9] * H[44];
	dp[1] += Z[10] * H[43];
	dp[2] += Z[10] * H[42];
	dp[3] += Z[10] * H[41];
	dp[0] += Z[10] * H[40];
	dp[1] += Z[11] * H[39];
	dp[2] += Z[11] * H[38];
	dp[3] += Z[11] * H[37];
	dp[0] += Z[11] * H[36];
	dp[1] += Z[12] * H[35];
	dp[2] += Z[12] * H[34];
	dp[3] += Z[12] * H[33];
	dp[0] += Z[12] * H[32];

	dp[1] += Z[13] * H[31];
	dp[2] += Z[13] * H[30];
	dp[3] += Z[13] * H[29];
	dp[0] += Z[13] * H[28];
	dp[1] += Z[14] * H[27];
	dp[2] += Z[14] * H[26];
	dp[3] += Z[14] * H[25];
	dp[0] += Z[14] * H[24];
	dp[1] += Z[15] * H[23];
	dp[2] += Z[15] * H[22];
	dp[3] += Z[15] * H[21];
	dp[0] += Z[15] * H[20];
	dp[1] += Z[16] * H[19];
	dp[2] += Z[16] * H[18];
	dp[3] += Z[16] * H[17];
	dp[0] += Z[16] * H[16];
	dp[1] += Z[17] * H[15];
	dp[2] += Z[17] * H[14];
	dp[3] += Z[17] * H[13];
	dp[0] += Z[17] * H[12];
	dp[1] += Z[18] * H[11];
	dp[2] += Z[18] * H[10];
	dp[3] += Z[18] * H[9];
	dp[0] += Z[18] * H[8];
	dp[1] += Z[19] * H[7];
	dp[2] += Z[19] * H[6];
	dp[3] += Z[19] * H[5];
	dp[0] += Z[19] * H[4];
	dp[1] += Z[20] * H[3];
	dp[2] += Z[20] * H[2];
	dp[3] += Z[20] * H[1];
	dp[0] += Z[20] * H[0];
}
#endif

//*********************************************************************
// COVERLIMIT	高品質リミッタ
//
COVERLIMIT::COVERLIMIT()
{
#if !OVERFIR
	iira.MakeIIR(2800, SampFreq * 4.0, 24, 0, 0);
	iirb.MakeIIR(2800, SampFreq * 4.0, 24, 0, 0);
#endif
}

double COVERLIMIT::Do(double d, double lmt)
{
#if OVERFIR
	double dt[4];

	intp.Do(dt, d);

	dt[0] *= lmt;
	if( dt[0] > 16384.0 ) dt[0] = 16384.0;
	if( dt[0] < -16384.0 ) dt[0] = -16384.0;
	dt[1] *= lmt;
	if( dt[1] > 16384.0 ) dt[1] = 16384.0;
	if( dt[1] < -16384.0 ) dt[1] = -16384.0;
	dt[2] *= lmt;
	if( dt[2] > 16384.0 ) dt[2] = 16384.0;
	if( dt[2] < -16384.0 ) dt[2] = -16384.0;
	dt[3] *= lmt;
	if( dt[3] > 16384.0 ) dt[3] = 16384.0;
	if( dt[3] < -16384.0 ) dt[3] = -16384.0;

	return decm.Do(dt);
#else
	double d1, d2, d3, d4;

	// X4 インターポーレータ
	d1 = iira.Do(d);
	d2 = iira.Do(d);
	d3 = iira.Do(d);
	d4 = iira.Do(d);

	d1 *= lmt;
	if( d1 > 16384.0 ) d1 = 16384.0;
	if( d1 < -16384.0 ) d1 = -16384.0;
	d2 *= lmt;
	if( d2 > 16384.0 ) d2 = 16384.0;
	if( d2 < -16384.0 ) d2 = -16384.0;
	d3 *= lmt;
	if( d3 > 16384.0 ) d3 = 16384.0;
	if( d3 < -16384.0 ) d3 = -16384.0;
	d4 *= lmt;
	if( d4 > 16384.0 ) d4 = 16384.0;
	if( d4 < -16384.0 ) d4 = -16384.0;

	// 1/4 デシメータ
	iirb.Do(d1);
	iirb.Do(d2);
	iirb.Do(d3);
	return iirb.Do(d4);
#endif
}

//*********************************************************************
// CDECM2IIR	1/2デシメータ処理クラス
//
// ===== Rtty.cpp, righe 26-240 =====
#define	LOGFFT	FALSE

CSinTable	g_SinTable;

//---------------------------------------------------------------------------
// CSinTableクラス
// Added by JE3HHT on Aug.2010
__fastcall CSinTable::CSinTable()
{
	m_Size = 48000;
    m_tSin = new double[m_Size];
	double pi2t = 2 * PI / double(m_Size);
	for( int i = 0; i < m_Size; i++ ){
		m_tSin[i] = sin(double(i) * pi2t);
	}
}
//---------------------------------------------------------------------------
__fastcall CSinTable::~CSinTable()
{
	delete[] m_tSin;
}
//---------------------------------------------------------------------------
CTICK::CTICK()
{
	ptbl[0] = new int[12500];
	ptbl[1] = new int[12500];
	memset(ptbl[0], 0, sizeof(int[12500]));
	memset(ptbl[1], 0, sizeof(int[12500]));
}
CTICK::~CTICK()
{
	delete[] ptbl[0];	//JA7UDE 0428
	delete[] ptbl[1];	//JA7UDE 0428
}
void CTICK::Init(void)
{
	m_wsel = 0;
	m_wp = ptbl[0];
	m_wcnt = 0;
	m_Trig = 0;
}
void CTICK::Write(double d)
{
	*m_wp = int(d);
	m_wp++;
	m_wcnt++;
	if( m_wcnt >= m_Samp ){
		m_wcnt = 0;
		m_wsel++;
		m_wsel &= 1;
		m_wp = ptbl[m_wsel];
		m_Trig = 1;
	}
}
int *CTICK::GetData(void)
{
	if( !m_Trig ) return NULL;
	m_Trig = 0;
	int sel = m_wsel + 1;
	sel &= 1;
	return ptbl[sel];
}
//---------------------------------------------------------------------------
// VCOクラス
CVCO::CVCO()
{
	m_vlock = 0;
	m_SampleFreq = SampFreq;
	m_FreeFreq = 1900.0;
	m_TableSize = int(SampFreq*2);
	pSinTbl = new double[m_TableSize];
	m_c1 = m_TableSize/16.0;
	m_c2 = int( double(m_TableSize) * m_FreeFreq / m_SampleFreq );
	m_z = 0;
	double pi2t = 2 * PI / double(m_TableSize);
	for( int i = 0; i < m_TableSize; i++ ){
		pSinTbl[i] = sin(double(i) * pi2t);
	}
}

CVCO::~CVCO()
{
	if( m_vlock ) ::VirtualUnlock(pSinTbl, sizeof(double)*m_TableSize);
	delete[] pSinTbl;
}

void CVCO::SetGain(double gain)
{
	m_c1 = double(m_TableSize) * gain / m_SampleFreq;
}

void CVCO::VirtualLock(void)
{
	if( !m_vlock ){
		::VirtualLock(pSinTbl, sizeof(double)*m_TableSize);
		m_vlock = 1;
	}
}

void CVCO::SetSampleFreq(double f)
{
	m_SampleFreq = f;
	int size = int(m_SampleFreq*2);
	if( m_TableSize != size ){
		if( pSinTbl != NULL ){
			if( m_vlock ) ::VirtualUnlock(pSinTbl, sizeof(double)*m_TableSize);
			delete[] pSinTbl;
		}
		m_TableSize = size;
		pSinTbl = new double[m_TableSize];
		if( m_vlock ) ::VirtualLock(pSinTbl, sizeof(double)*m_TableSize);
		double pi2t = 2 * PI / double(m_TableSize);
		for( int i = 0; i < m_TableSize; i++ ){
			pSinTbl[i] = sin(double(i) * pi2t);
		}
	}
	SetFreeFreq(m_FreeFreq);
}

void CVCO::SetFreeFreq(double f)
{
	m_FreeFreq = f;
	m_c2 = double(m_TableSize) * m_FreeFreq / m_SampleFreq;
}

void CVCO::InitPhase(void)
{
	m_z = 0;
}

double CVCO::Do(double d)
{		// -1 to 1
	m_z += (d * m_c1 + m_c2) + 0.5;
	while( m_z >= m_TableSize ){
		m_z -= m_TableSize;
	}
	while( m_z < 0 ){
		m_z += m_TableSize;
	}
	return pSinTbl[int(m_z)];
}

//---------------------------------------------------------------------------
// VCOXクラス
__fastcall CVCOX::CVCOX()
{
	m_SampleFreq = SampFreq;
	m_FreeFreq = 2000.0;
	m_TableSize = g_SinTable.m_Size;
    m_TableCOS = m_TableSize / 4;
	m_c1 = 0.0;
	m_c2 = double(m_TableSize) * m_FreeFreq / m_SampleFreq;
	m_z = 0.0;
}

__fastcall CVCOX::~CVCOX()
{
}

void __fastcall CVCOX::SetGain(double gain)
{
	m_c1 = double(m_TableSize) * gain / m_SampleFreq;
}

void __fastcall CVCOX::SetSampleFreq(double f)
{
	m_SampleFreq = f;
	SetFreeFreq(m_FreeFreq);
}

void __fastcall CVCOX::SetFreeFreq(double f)
{
	m_FreeFreq = f;
	m_c2 = double(m_TableSize) * m_FreeFreq / m_SampleFreq;
	if( f < 1.0 ) m_z = 0;
}

void __fastcall CVCOX::InitPhase(void)
{
	m_z = 0.0;
}
//------------------------------------------------------------------
double __fastcall CVCOX::Do(void)
{
	m_z += m_c2;
	if( m_z >= m_TableSize ){
		m_z -= m_TableSize;
	}
	if( m_z < 0 ){
		m_z += m_TableSize;
	}
	return g_SinTable.m_tSin[int(m_z)];
}
//------------------------------------------------------------------
double __fastcall CVCOX::Do(double d)
{
	// -1 to 1
	m_z += (d * m_c1 + m_c2);
	while( m_z >= m_TableSize ){
		m_z -= m_TableSize;
	}
	while( m_z < 0 ){
		m_z += m_TableSize;
	}
	return g_SinTable.m_tSin[int(m_z)];
}
//------------------------------------------------------------------
double __fastcall CVCOX::DoCos(void)
{
	double z = m_z + m_TableCOS;
    if(z >= m_TableSize ) z -= m_TableSize;
	return g_SinTable.m_tSin[int(z)];
}

//---------------------------------------------------------------------------
// ===== Rtty.cpp, righe 630-1433 =====

//---------------------------------------------------------------------------
// CFSKDEMクラス
CFSKDEM::CFSKDEM()
{
	m_OverFlow = 0;
	m_lpf = 0;
	m_lpfOrder = 5;
	SetLPFFreq(40.0);

	m_pll.SetSampleFreq(DemSamp);
    m_Phase.SetSampleFreq(DemSamp);

	memset(ZMark, 0, sizeof(ZMark));
	memset(ZSpace, 0, sizeof(ZSpace));
	memset(HMark, 0, sizeof(HMark));
	memset(HSpace, 0, sizeof(HSpace));
	i2 = 0;
//	o1 = o2 = 0;
	Count = 0;

	SetSmoozFreq(70.0);
	avgMark.SetCount(m_Smooz);
	avgSpace.SetCount(m_Smooz);

	m_BitLen = 5;
	m_StopLen = 4;		// 0-1bit, 1-1.5bit, 2-2bit, 3-1bit, 4-1.42bit
	m_Parity = 0;
	m_SumParity = 0;

	m_mode = 0;
	m_DataCount = 0;

	m_inv = 0;
	m_sq = 0;
	m_SQLevel = 600.0;
	m_Tap = 72;
	m_FilWidth = GetFilWidth(m_Tap);

	m_wp = 0;
	m_rp = 0;
	m_BufCount = 0;

	SetBaudRate(45.45);

	m_MarkFreq = 2125;
	m_SpaceFreq = 2295;
	SetMarkFreq(2125);
	SetSpaceFreq(2295);

	m_dMark = 0;
	m_dSpace = 0;

	m_Scope = 0;
	m_majority = 1;
	m_ignoreFream = 0;
	m_XYScope = 0;
	SetIIR(60.0);
	m_Limit = 1;
	m_LimitGain = 10.0;

	m_deff = 0;
	m_avgdeff = 0;

	m_sqcount = 0;
//	SmoozSQ.SetCount(4);
	SmoozSQ.SetCount(8);

	m_LimitOverSampling = 0;

	m_Tick = 0;
	m_atcPLL.m_Max = 12;

	m_LimitAGC = 1;
	m_limitMax = 1;
	m_limitMin = -1;
	m_d = 0;
}

void CFSKDEM::SetIIR(double bw)
{
	m_iirfw = bw;
	m_iirm.SetFreq(m_AFCMarkFreq, DemSamp, m_iirfw);
	m_iirs.SetFreq(m_AFCSpaceFreq, DemSamp, m_iirfw);
	m_pll.SetFreeFreq(m_AFCMarkFreq, m_AFCSpaceFreq);
}

void CFSKDEM::SetBaudRate(double b)
{
	if( b >= 1.0 ){
		m_BaudRate = b;
		m_Count = m_ReCount = int(DemSamp/b + 0.5);
	}
}

void CFSKDEM::SetSmoozFreq(double f)
{
	m_SmoozFreq = f;
	m_Smooz = int(DemSamp / f + 0.5);

	avgMark.SetCount(m_Smooz);
	avgSpace.SetCount(m_Smooz);
}

void CFSKDEM::SetLPFFreq(double f)
{
	m_lpffreq = f;
	LpfMark.MakeIIR(f, DemSamp, m_lpfOrder, 0, 1.0);
	LpfSpace.MakeIIR(f, DemSamp, m_lpfOrder, 0, 1.0);
}

void CFSKDEM::SetMarkFreq(double d)
{
	if ((m_AFCSpaceFreq - d) < 4000) {    //1.70D prevent divide by zero in M_SlideFFT.Create
		m_SetMarkFreq = m_AFCMarkFreq = m_MarkFreq = d;
		MakeFilter(HMark, m_Tap, ffBPF, DemSamp, m_MarkFreq-m_FilWidth, m_MarkFreq+m_FilWidth, 60, 1.0);
		SetIIR(m_iirfw);
		m_Phase.SetCarrierFreq(m_AFCMarkFreq);
		if( m_AFCSpaceFreq > m_AFCMarkFreq ) m_Phase.SetShift(m_AFCSpaceFreq - m_AFCMarkFreq);
		m_AA6YQ.SetMarkFreq(m_AFCMarkFreq);
	}
}

void CFSKDEM::SetSpaceFreq(double d)
{
	if ((d - m_AFCMarkFreq) < 4000) {    //1.70D prevent divide by zero in M_SlideFFT.Create
		m_SetSpaceFreq = m_AFCSpaceFreq = m_SpaceFreq = d;
		MakeFilter(HSpace, m_Tap, ffBPF, DemSamp, m_SpaceFreq-m_FilWidth, m_SpaceFreq+m_FilWidth, 60, 1.0);
		SetIIR(m_iirfw);
		m_Phase.SetCarrierFreq(m_AFCMarkFreq);
		if( m_AFCSpaceFreq > m_AFCMarkFreq ) m_Phase.SetShift(m_AFCSpaceFreq - m_AFCMarkFreq);
		m_AA6YQ.SetSpaceFreq(m_AFCSpaceFreq);
	}
}

void CFSKDEM::AFCMarkFreq(double d)
{
	if( d != m_MarkFreq ){
		m_MarkFreq = d;
		if( fabs(d - m_AFCMarkFreq) >= (m_type ? 10.0 : 5.0) ){
			m_AFCMarkFreq = d;
			MakeFilter(HMark, m_Tap, ffBPF, DemSamp, m_MarkFreq-m_FilWidth, m_MarkFreq+m_FilWidth, 60, 1.0);
			SetIIR(m_iirfw);
		    m_Phase.SetCarrierFreq(m_AFCMarkFreq);
		}
	    m_AA6YQ.SetMarkFreqByAFC(d);
	}
}

void CFSKDEM::AFCSpaceFreq(double d)
{
	if( d != m_SpaceFreq ){
		m_SpaceFreq = d;
		if( fabs(d - m_AFCSpaceFreq) >= (m_type ? 10.0 : 5.0) ){
			m_AFCSpaceFreq = d;
			MakeFilter(HSpace, m_Tap, ffBPF, DemSamp, m_SpaceFreq-m_FilWidth, m_SpaceFreq+m_FilWidth, 60, 1.0);
			SetIIR(m_iirfw);
		}
	    m_AA6YQ.SetSpaceFreqByAFC(d);
	}
}

double CFSKDEM::GetFilWidth(int tap)
{
	double Width;

	if( tap >= 256 ){
		Width = 20;
	}
	else if( tap >= 192 ){
		Width = 30;
	}
	else {
		Width = 40;
	}
	return Width;
}

void CFSKDEM::SetFilterTap(int tap)
{
	if( tap != m_Tap ){
		m_Tap = tap;
		m_FilWidth = GetFilWidth(tap);
		MakeFilter(HSpace, m_Tap, ffBPF, DemSamp, m_SpaceFreq-m_FilWidth, m_SpaceFreq+m_FilWidth, 60, 1.0);
		MakeFilter(HMark, m_Tap, ffBPF, DemSamp, m_MarkFreq-m_FilWidth, m_MarkFreq+m_FilWidth, 60, 1.0);
	}
}

void CFSKDEM::WriteData(BYTE d)
{
	if( m_BufCount < DEMBUFMAX ){
		m_Buff[m_wp] = d;
		m_wp++;
		m_BufCount++;
		if( m_wp >= DEMBUFMAX ) m_wp = 0;
	}
}

void CFSKDEM::DoFSK(void)
{
	int b;

	if( m_Tick ){
		Tick.Write(m_dMark);
		return;
	}
	if( m_dMark >= m_dSpace ){	// マークの時
		b = m_inv ? 0 : 1;
	}
	else {						// スペースの時
		b = m_inv ? 1 : 0;
	}
	double deff = fabs(m_dMark - m_dSpace);

// Adjustment for the level meter, added by JE3HHT (Ver 1.68A)
	if( m_AA6YQ.m_fEnabled && (m_type != 3) ){
		deff *= 4;
       	deff -= 30000;
    }
// till end

	if( m_deff < deff ) m_deff = deff;
	if( !m_sqcount ){
		m_avgdeff = SmoozSQ.Avg(m_deff);
		m_deff = 0;
		m_sqcount = DemSamp / 10;
	}
	m_sqcount--;

	if( m_sq ){
		if( !m_mode ){
			if( m_Limit  ){		// 受信時
				if( (m_SQLevel * 10.0) > m_avgdeff ){
					b = 1;
				}
			}
			else {				// 送信時
				if( m_SQLevel > m_avgdeff ){
					b = 1;
				}
			}
		}
	}
//	m_Bit = b;
	if( m_Scope ){
		m_ScopeSync.WriteData(m_Count ? 0 : 8192.0);
		m_ScopeBit.WriteData(b ? 8192.0 : 0);
	}
	switch(m_mode){
		case 0:			// スタートビット検出待ち
			if( !b ){
				m_Count = m_ReCount/2;
				if( m_majority ){
					m_mark = m_space = 0;
					m_mode = 256;	// 多数決ロジック
				}
				else {
					m_mode++;
				}
#if BITDEBUG
				m_bitCountA = m_bitCount;
				m_bitCount = 0;
#endif
			}
			else {
				m_Count = m_ReCount;
			}
			break;
// 通常ロジックによる判定
		case 1:			// スタートビット検出待ち(Half)
			if( b ){
				m_mode = 0;
			}
			else if( !m_Count ){
				if( m_Scope ){
					m_ScopeSync.UpdateData(-8192.0);	// スタートビット位置
				}
				m_Count = m_ReCount;
				m_Data = 0;
				m_DataCount = m_BitLen;
				m_SumParity = 0;
				m_mode++;
			}
			break;
		case 2:			// 符号記録中
			if( !m_Count ){
				m_Count = m_ReCount;
				m_Data = BYTE(m_Data << 1);
				m_Data |= BYTE(b);
				m_DataCount--;
				if( !m_DataCount ){
					if( m_Parity ){
						m_mode++;
					}
					else {
						m_mode += 2;
					}
				}
				if( m_Parity ){
					m_SumParity += b;
				}
			}
			break;
		case 3:			// パリティビット
			if( !m_Count ){
				m_Count = m_ReCount;
				m_mode++;
				switch(m_Parity){
					case 1:			// Even
						if( (!(m_SumParity & 1) ^ b) & 1 ) m_mode = 0;		// パリティエラー
						break;
					case 2:			// Odd
						if( ((m_SumParity & 1) ^ b) & 1 ) m_mode = 0;		// パリティエラー
						break;
					case 3:
						if( !b ) m_mode = 0;
						break;
					case 4:
						if( b ) m_mode = 0;
						break;
					default:
						break;
				}
				if( !m_mode ){	// パリティエラー
					m_Count = m_ReCount;
					m_mode = 7;
				}
			}
			break;
		case 4:			// ストップビットの確認
			if( !m_Count ){
				if( m_Scope ){
					m_ScopeSync.UpdateData(-4096.0);	// Stopbit位置
				}
				if( b  || m_ignoreFream ){
					if( m_BufCount < DEMBUFMAX ){
						m_Buff[m_wp] = m_Data;
						m_wp++;
						m_BufCount++;
						if( m_wp >= DEMBUFMAX ) m_wp = 0;
					}
					switch(m_StopLen){
						case 2:		// 2bit
							m_Count = (m_ReCount * 11/8);		// 23/16bit時間のウエイト
							break;
						case 1:		// 1.5bit
							m_Count = (m_ReCount * 7/8);		// 15/16bit時間のウエイト
							break;
						case 4:		// 1.42bit
							m_Count = (m_ReCount * 4/5);		// 0.82bit時間のウエイト
							break;
						case 3:
						default:	// 1bit
							m_Count = (m_ReCount * 3/8);		// 7/16bit時間のウエイト
							break;
					}
					m_mode++;
				}
				if( !b ){			// フレーミングエラー
					m_mode = 8;
				}
			}
			break;
		case 5:			// ストップビット終了待ちタイマー
			if( !m_Count ){
				if( m_Scope ){
					m_ScopeSync.UpdateData(-4096.0);	// Stopbits終了位置
				}
				m_mode++;
			}
			break;
		case 6:			// ストップ終了待ち
			if( b ){
				m_Count = m_ReCount;
			}
			else {		// 次のスタートビット
				m_Count = m_ReCount/2;
				m_mode = 0;
			}
			break;
		case 7:			// パリティエラー時
			if( !m_Count ){
				m_ScopeSync.UpdateData(-4096.0);	// Stopbit位置
				m_mode--;
			}
			break;
		case 8:			// フレーミングエラー時
			m_Count = m_ReCount;
			if( b ){
				m_mode = 0;
			}
			break;
// 多数決ロジックによるデータ取りこみ
		case 256:		// スタートビット検出待ち(Half)
			if( b ){m_mark++;} else {m_space++;}
			if( !m_Count ){
				b = (m_mark >= m_space) ? 1 : 0;
				if( b ){
					m_mode = 0;
				}
				else {
					if( m_Scope ){
						m_ScopeSync.UpdateData(-8192.0);	// スタートビット位置
					}
					m_Count = m_ReCount/2;
					m_mode++;
				}
			}
			break;
		case 257:		// スタートビット終了待ち
			if( !m_Count ){
				if( m_Scope ){
					m_ScopeSync.UpdateData(-8192.0);	// スタートビット位置
				}
				m_Count = m_ReCount;
				m_mark = m_space = 0;
				m_Data = 0;
				m_DataCount = m_BitLen;
				m_SumParity = 0;
				m_mode++;
			}
			break;
		case 258:			// 符号記録中
			if( b ){m_mark++;} else {m_space++;}
			if( !m_Count ){
				b = (m_mark >= m_space) ? 1 : 0;
				m_mark = m_space = 0;
				m_Count = m_ReCount;
				m_Data = BYTE(m_Data << 1);
				m_Data |= BYTE(b);
				m_DataCount--;
				if( !m_DataCount ){
					if( m_Parity ){
						m_mode++;
					}
					else {
						switch(m_StopLen){
							case 2:		// 2bit
							case 1:		// 1.5bit
							case 4:		// 1.42bit
								break;
							case 3:
							default:	// 1bit
								m_Count = m_ReCount * 7 / 8;
								break;
						}
						m_mode += 2;
					}
				}
				if( m_Parity ){
					m_SumParity += b;
				}
			}
			break;
		case 259:			// パリティビット
			if( b ){
				m_mark++;
			}
			else {
				m_space++;
			}
			if( !m_Count ){
				b = (m_mark >= m_space) ? 1 : 0;
				m_mark = m_space = 0;
				m_Count = m_ReCount;
				m_mode++;
				switch(m_Parity){
					case 1:			// Even
						if( (!(m_SumParity & 1) ^ b) & 1 ) m_mode = 0;		// パリティエラー
						break;
					case 2:			// Odd
						if( ((m_SumParity & 1) ^ b) & 1 ) m_mode = 0;		// パリティエラー
						break;
					case 3:
						if( !b ) m_mode = 0;
						break;
					case 4:
						if( b ) m_mode = 0;
						break;
					default:
						break;
				}
				if( !m_mode ){	// パリティエラー
					m_Count = m_ReCount;
					m_mode = 7;
				}
				else {
					switch(m_StopLen){
						case 2:		// 2bit
						case 1:		// 1.5bit
						case 4:		// 1.42bit
							break;
						case 3:
						default:	// 1bit
							m_Count = m_ReCount * 7 / 8;
							break;
					}
				}
			}
			break;
		case 260:			// ストップビットの確認
			if( b ){
				m_mark++;
			}
			else {
				m_space++;
			}
			if( !m_Count ){
				b = (m_mark >= m_space) ? 1 : 0;
				m_mark = m_space = 0;
				if( m_Scope ){
					m_ScopeSync.UpdateData(-4096.0);	// Stopbit位置
				}
				if( b || m_ignoreFream ){
					if( m_BufCount < DEMBUFMAX ){
						m_Buff[m_wp] = m_Data;
						m_wp++;
						m_BufCount++;
						if( m_wp >= DEMBUFMAX ) m_wp = 0;
					}
					switch(m_StopLen){
						case 2:		// 2bit
							m_Count = (m_ReCount * 7/8);
							break;
						case 1:		// 1.5bit
							m_Count = (m_ReCount * 3/8);
							break;
						case 4:		// 1.42bit
							m_Count = (m_ReCount * 2/5);
							break;
						case 3:
						default:	// 1bit
							m_Count = m_ReCount;
							if( m_Scope ){
								m_ScopeSync.UpdateData(-4096.0);	// Stopbits終了位置
							}
							m_mode = -1;
							break;
					}
					m_mode++;
				}
				if( !b ){		// フレーミングエラー
					m_mode = 8;
				}
			}
			break;
		case 261:			// ストップビット終了待ちタイマー
			if( !m_Count ){
				if( m_Scope ){
					m_ScopeSync.UpdateData(-4096.0);	// Stopbits終了位置
				}
				m_mode++;
			}
			break;
		case 262:			// ストップ終了待ち
			if( b ){
				m_Count = m_ReCount;
			}
			else {		// 次のスタートビット
				m_Count = m_ReCount/2;
				m_mode = 0;
			}
			break;
	};
	m_Count--;
#if BITDEBUG
	m_bitCount++;
#endif
}

void CFSKDEM::Do(double d)
{
	if( m_AA6YQ.m_fEnabled ) d = m_AA6YQ.Do(d);
	double ds = d;

	if( (d > 24578.0) || (d < -24578.0) ){
		m_OverFlow = 1;
	}
	if( m_Limit ){
		if( m_LimitAGC ){
			if( m_limitMax < d ) m_limitMax = d;
			if( m_limitMin > d ) m_limitMin = d;
			if( (d >= 0) && (m_d < 0) ){
				m_limitagc = (m_limitMax - m_limitMin);
				if( m_limitagc ){
					m_limitagc = (64.0 * 16384.0)/ m_limitagc;
					if( m_limitagc >= 4096.0 ) m_limitagc = 4096;
				}
				else {
					m_limitagc = 200;
				}
				m_limitMax = 1;
				m_limitMin = -1;
			}
			m_d = d;
			if( m_LimitOverSampling ){
				d = OverLimit.Do(d, m_limitagc);
			}
			else {
				d *= m_limitagc;
				if( d > 16384.0 ) d = 16384.0;
				if( d < -16384.0 ) d = -16384.0;
			}
		}
		else {
			if( m_LimitOverSampling ){
				d = OverLimit.Do(d, m_LimitGain);
			}
			else {
				d *= m_LimitGain;
				if( d > 16384.0 ) d = 16384.0;
				if( d < -16384.0 ) d = -16384.0;
			}
		}
//		if( m_AA6YQ.m_fEnabled ) d *= 0.3333;	// Delete by JE3HHT (Ver1.68A)
	}
	if( (Count & 1) || (!DemOver) ){
		if( DemOver ) d = DECM2.Do(d, i2);		// 1/2デシメータ

		switch(m_type){
			case 2:			// PLL
			{
				double dm = m_iirm.Do(d);
				double ds = m_iirs.Do(d);
				d = m_pll.Do(d);
				if( m_XYScope ){	// For XY-Scope
					m_XYScopeMark.WriteData(dm);
					m_XYScopeSpace.WriteData(ds);
				}
				if( m_Scope  ){	// For Scope
					m_ScopeMark[0].WriteData(dm);
					m_ScopeSpace[0].WriteData(ds);
				}
				if( m_Scope ){
					m_ScopeMark[1].WriteData(m_pll.GetOut());
					m_ScopeSpace[1].WriteData(d);
				}
				d = m_atcPLL.Do(d + 8192.0) - 8192.0;
				if( d >= 0 ){
					m_dMark = d;
					m_dSpace = 0;
				}
				else {
					m_dMark = 0;
					m_dSpace = -d;
				}
				// 平滑
				if( m_lpf ){
					m_dMark = LpfMark.Do(m_dMark);
					m_dSpace = LpfSpace.Do(m_dSpace);
				}
				else {
					m_dMark = avgMark.Avg(m_dMark);
					m_dSpace = avgSpace.Avg(m_dSpace);
				}
				if( m_Scope ){
					m_ScopeMark[2].WriteData(m_dMark);
					m_ScopeSpace[2].WriteData(m_dSpace);
				}
				if( m_atc ){
					m_dMark = m_atcMark.Do(m_dMark);
					m_dSpace = m_atcSpace.Do(m_dSpace);
					if( m_Scope ){
						m_ScopeMark[3].WriteData(m_dMark);
						m_ScopeSpace[3].WriteData(m_dSpace);
					}
				}
				break;
            }
			case 3:			// FFT
            {
				m_Phase.DoFSK(ds);
                m_dMark = m_Phase.m_dm;
                m_dSpace = m_Phase.m_ds;

				if( m_XYScope || m_Scope ){
					double dm = m_iirm.Do(d);
					double ds = m_iirs.Do(d);

					if( m_XYScope ){	// For XY-Scope
						m_XYScopeMark.WriteData(dm);
						m_XYScopeSpace.WriteData(ds);
					}
					if( m_Scope  ){		// For Scope
						m_ScopeMark[0].WriteData(dm);
						m_ScopeSpace[0].WriteData(ds);
					}
                }
				// 検波
//				m_dMark = fabs(m_dMark);
//				m_dSpace = fabs(m_dSpace);
				if( m_dMark < 0.0 ) m_dMark = -m_dMark;
				if( m_dSpace < 0.0 ) m_dSpace = -m_dSpace;
				if( m_Scope ){
					m_ScopeMark[1].WriteData(m_dMark);
					m_ScopeSpace[1].WriteData(m_dSpace);
				}
				// 平滑
				if( m_lpf ){
					m_dMark = LpfMark.Do(m_dMark);
					m_dSpace = LpfSpace.Do(m_dSpace);
				}
				else {
					m_dMark = avgMark.Avg(m_dMark);
					m_dSpace = avgSpace.Avg(m_dSpace);
				}
				if( m_Scope ){
					m_ScopeMark[2].WriteData(m_dMark);
					m_ScopeSpace[2].WriteData(m_dSpace);
				}
				if( m_atc ){
					m_dMark = m_atcMark.Do(m_dMark);
					m_dSpace = m_atcSpace.Do(m_dSpace);
					if( m_Scope ){
						m_ScopeMark[3].WriteData(m_dMark);
						m_ScopeSpace[3].WriteData(m_dSpace);
					}
				}
#if 1			// 高速計算
				m_dMark /= (64*32768.0);
                m_dSpace /= (64*32768.0);
#else
				m_dMark = m_dMark > 0.0 ? sqrt(m_dMark) : 0.0;
                m_dSpace = m_dSpace > 0.0 ? sqrt(m_dSpace) : 0.0;
				m_dMark *= (1.0/16.0);
                m_dSpace *= (1.0/16.0);
#endif
				break;
            }
			default:
            {
				if( m_type ){			// FIR
					m_dMark = DoFIR(HMark, ZMark, d, m_Tap);
					m_dSpace = DoFIR(HSpace, ZSpace, d, m_Tap);
				}
				else {					// IIR
					m_dMark = m_iirm.Do(d);
					m_dSpace = m_iirs.Do(d);
				}
				if( m_XYScope ){	// For XY-Scope
					m_XYScopeMark.WriteData(m_dMark);
					m_XYScopeSpace.WriteData(m_dSpace);
				}
				if( m_Scope  ){		// For Scope
					m_ScopeMark[0].WriteData(m_dMark);
					m_ScopeSpace[0].WriteData(m_dSpace);
				}
				// 検波
//				m_dMark = fabs(m_dMark);
//				m_dSpace = fabs(m_dSpace);
				if( m_dMark < 0 ) m_dMark = -m_dMark;
				if( m_dSpace < 0 ) m_dSpace = -m_dSpace;
				if( m_Scope ){
					m_ScopeMark[1].WriteData(m_dMark);
					m_ScopeSpace[1].WriteData(m_dSpace);
				}
				// 平滑
				if( m_lpf ){
					m_dMark = LpfMark.Do(m_dMark);
					m_dSpace = LpfSpace.Do(m_dSpace);
				}
				else {
					m_dMark = avgMark.Avg(m_dMark);
					m_dSpace = avgSpace.Avg(m_dSpace);
				}
				if( m_Scope ){
					m_ScopeMark[2].WriteData(m_dMark);
					m_ScopeSpace[2].WriteData(m_dSpace);
				}
				if( m_atc ){
					m_dMark = m_atcMark.Do(m_dMark);
					m_dSpace = m_atcSpace.Do(m_dSpace);
					if( m_Scope ){
						m_ScopeMark[3].WriteData(m_dMark);
						m_ScopeSpace[3].WriteData(m_dSpace);
					}
				}
				break;
            }
        }
		DoFSK();
	}
	else {
		i2 = d;
	}
	Count++;
}

int CFSKDEM::GetData(void)
{
	int r;
	if( m_BufCount ){
		r = m_Buff[m_rp];
		m_BufCount--;
		m_rp++;
		if( m_rp >= DEMBUFMAX ) m_rp = 0;
	}
	else {
		r = -1;
	}
	return r;
}


// ===== Rtty.cpp, righe 1462-1474 =====
const char _LTR[32]={
	0x00, 'T', 0x0d, 'O', ' ', 'H', 'N', 'M',
	0x0a, 'L', 'R', 'G', 'I', 'P', 'C', 'V',
	'E', 'Z', 'D', 'B', 'S', 'Y', 'F', 'X',
	'A', 'W', 'J', 0x00, 'U', 'Q', 'K', 0x00,
};
const char _FIG[32]={
	0x00, '5', 0x0d, '9', ' ', 'h', ',', '.',
	0x0a, ')', '4', '&', '8', '0', ':', ';',
	'3', '"', '$', '?', 's', '6', '!', '/',
	'-', '2', 0x27, 0x00, '7', '1', '(', 0x00,
};

// ===== Rtty.cpp, righe 1651-1700 =====

//--------------------------------------------------------
// CScopeクラス
CScope::CScope()
{
	m_ScopeSize = SCOPESIZE;
	m_DataFlag = 0;

	pScopeData = new double[m_ScopeSize];
	memset(pScopeData, 0, sizeof(double)*m_ScopeSize);
}

CScope::~CScope()
{
	delete[] pScopeData;
}

void CScope::WriteData(double d)
{
	if( !m_DataFlag ){
		if( m_wp < m_ScopeSize ){
			pScopeData[m_wp] = d;
			m_wp++;
			if( m_wp >= m_ScopeSize ){
				m_DataFlag = 1;
			}
		}
	}
}

void CScope::UpdateData(double d)
{
	if( !m_DataFlag ){
		if( m_wp ){
			pScopeData[m_wp-1] = d;
		}
	}
}

void CScope::Collect(int size)
{
	m_DataFlag = 1;
	m_ScopeSize = size;
	m_wp = 0;
	m_DataFlag = 0;
}



//--------------------------------------------------------
// ===== Rtty.cpp, righe 1725-2624 =====
//--------------------------------------------------------
// CSamplePeakクラス
CSamplePeak::CSamplePeak()
{
	memset(Strage, 0, sizeof(Strage));
	m_CurPeak = 0.0;
	m_Peak = 0.0;
	m_Strage = 8 - 1;
	SetBaudRate(45.45);
}

void CSamplePeak::SetBaudRate(double b)
{
	if( b >= 1.0 ){
		m_ReCount = m_Count = int(DemSamp/b + 0.5);
	}
}

void CSamplePeak::Sync(int Delay)
{
	m_Count = m_ReCount - Delay;
	while( m_Count < 0 ) m_Count += m_ReCount;
	memcpy(Strage, &Strage[1], sizeof(double)*m_Strage);
	Strage[m_Strage] = m_CurPeak;
	m_Peak = 0.0;
	int i;
	for( i = 0; i <= m_Strage; i++ ){
		if( m_Peak < Strage[i] ) m_Peak = Strage[i];
	}
	m_CurPeak = 0.0;
}

int CSamplePeak::Sample(double d)
{
	int r = 0;

	if( m_CurPeak < d ){
		m_CurPeak = d;
		if( m_Peak < d ){
			m_Peak = d;
			r = 1;
		}
	}
	if( !m_Count ){
		Sync(0);
		r = 1;
	}
	m_Count--;
	return r;
}

//--------------------------------------------------------
// CAGCクラス
CAGC::CAGC()
{
	m_MaxGain = 2048.0;
	m_StepGain = 3.0;
	m_MarkGain = 1.0;
	m_SpaceGain = 1.0;
	m_DeffGain = 12.0;
	m_Sync = 1;
}

void CAGC::Sync(int Delay)
{
	if( m_Sync ){
		Mark.Sync(Delay);
		Space.Sync(Delay);
	}
}

double CAGC::SampleMark(double d)
{
	if( Mark.Sample(d) ){
		double gain = Mark.GetPeak();
		if( gain ){
			gain = 8192.0 / gain;
			double ugain = m_MarkGain * m_StepGain;
			double dgain = m_MarkGain / m_StepGain;
			if( ugain < gain ){
				m_MarkGain = ugain;
			}
			else if( dgain > gain ){
				m_MarkGain = dgain;
			}
			else {
				m_MarkGain = gain;
			}
			if( m_MarkGain > m_MaxGain ) m_MarkGain = m_MaxGain;
		}
		else {
			m_MarkGain = 1.0;
		}
	}
	return d * m_MarkGain;
}

double CAGC::SampleSpace(double d)
{
	if( Space.Sample(d) ){
		double gain = Space.GetPeak();
		if( gain ){
			gain = 8192.0 / gain;
			double ugain = m_SpaceGain * m_StepGain;
			double dgain = m_SpaceGain / m_StepGain;
			if( ugain < gain ){
				m_SpaceGain = ugain;
			}
			else if( dgain > gain ){
				m_SpaceGain = dgain;
			}
			else {
				m_SpaceGain = gain;
			}
			if( m_SpaceGain > m_MaxGain ) m_SpaceGain = m_MaxGain;
			if( m_SpaceGain > m_MarkGain ){
				if( m_SpaceGain > m_MarkGain * m_DeffGain ){
					m_SpaceGain = m_MarkGain * m_DeffGain;
				}
			}
			else {
				if( m_SpaceGain * m_DeffGain < m_MarkGain ){
					m_SpaceGain = m_MarkGain / m_DeffGain;
				}
			}
		}
		else {
			m_SpaceGain = 1.0;
		}
	}
	return d * m_SpaceGain;
}

//--------------------------------------------------------
// CATCクラス
CATC::CATC()
{
	m_Low = 0;
	m_High = 16384.0;
	m_CurLow = MAXDOUBLE;
	m_CurHigh = -MAXDOUBLE;
	m_Max = 4;
	m_Cnt = 0;
	int i;
	for( i = 0; i < ATCMAX; i++ ){
		m_LowList[i] = m_Low;
		m_HighList[i] = m_High;
	}
	m_iir.MakeIIR(100, DemSamp, 3, 0, 0);
}

double CATC::Do(double d)
{
	if( m_CurLow > d ) m_CurLow = d;
	if( m_CurHigh < d ) m_CurHigh = d;
//		if( m_Low > d ) m_Low = d;
//        if( m_High < d ) m_High = d;
	if( !m_Cnt ){
		m_Cnt = 64;
		if( m_CurLow > (ATCC-ATCW) ) m_CurLow = (ATCC-ATCW);
		if( m_CurHigh < (ATCC+ATCW) ) m_CurHigh = (ATCC+ATCW);
		if( m_Max ){
			memcpy(m_LowList, &m_LowList[1], (m_Max)*sizeof(double));
			memcpy(m_HighList, &m_HighList[1], (m_Max)*sizeof(double));
		}
		m_LowList[m_Max] = m_CurLow;
		m_HighList[m_Max] = m_CurHigh;
		int i;
		m_Low = m_LowList[0];
		m_High = m_HighList[0];
		for( i = 1; i <= m_Max; i++ ){
			if( m_Low > m_LowList[i] ) m_Low = m_LowList[i];
			if( m_High < m_HighList[i] ) m_High = m_HighList[i];
		}
		m_CurLow = MAXDOUBLE;
		m_CurHigh = -MAXDOUBLE;
	}
	m_Cnt--;
	double th = ((m_High + m_Low)*0.5);
	th = m_iir.Do(th);
	if( m_High > m_Low ){
		d = (d - th) * 1.1 + th;
	}
	d += (ATCC - th);
	return d;
}

//---------------------------------------------------------------------------
// CFIRクラス
__fastcall CFIR::CFIR()
{
	m_pZ = NULL;
    m_pH = NULL;
    m_Tap = 0;
}

//---------------------------------------------------------------------------
__fastcall CFIR::~CFIR()
{
	if( m_pZ ) delete[] m_pZ;
	if( m_pH ) delete[] m_pH;
}
//---------------------------------------------------------------------------
void __fastcall CFIR::Create(int tap, int type, double fs, double fcl, double fch, double att, double gain)
{
	m_Tap = tap;
	if( m_pZ ) delete[] m_pZ;
    if( m_pH ) delete[] m_pH;
	m_pZ = new double[tap+1];
	m_pH = new double[tap+1];
    memset(m_pZ, 0, sizeof(double)*(tap+1));
	::MakeFilter(m_pH, tap, type, fs, fcl, fch, att, gain);
}
//---------------------------------------------------------------------------
double __fastcall CFIR::Do(double d)
{
	return DoFIR(m_pH, m_pZ, d, m_Tap);
}
//---------------------------------------------------------------------------
void __fastcall CFIR::SaveCoef(LPCSTR pName)
{
	FILE *fp;
    if( (fp = fopen(pName, "wt")) != NULL ){
		int i;
		for( i = 0; i <= m_Tap; i++ ){
			fprintf(fp, "H[%u]=%lf\n", i, m_pH[i]);
        }
		fclose(fp);
    }
}

//---------------------------------------------------------------------------
// CFIR2クラス
__fastcall CFIR2::CFIR2()
{
	m_pZ = NULL;
    m_pH = NULL;
    m_pZP = NULL;
    m_W = 0;
    m_Tap = 0;
    m_fs = 0;
}

//---------------------------------------------------------------------------
__fastcall CFIR2::~CFIR2()
{
	if( m_pZ ) delete[] m_pZ;
	if( m_pH ) delete[] m_pH;
}
//---------------------------------------------------------------------------
void __fastcall CFIR2::Delete(void)
{
	if( m_pZ ) delete[] m_pZ;
    if( m_pH ) delete[] m_pH;
	m_pZ = NULL;
    m_pH = NULL;
    m_pZP = NULL;
    m_W = 0;
    m_Tap = 0;
    m_fs = 0;
}
//---------------------------------------------------------------------------
void __fastcall CFIR2::Create(int tap, int type, double fs, double fcl, double fch, double att, double gain)
{
	if( (m_Tap != tap) || !m_pZ || !m_pH ){
		if( m_pZ ) delete[] m_pZ;
		m_pZ = new double[(tap+1)*2];
		memset(m_pZ, 0, sizeof(double)*(tap+1)*2);
		if( m_pH ) delete[] m_pH;
		m_pH = new double[tap+1];
		m_W = 0;
    }
	m_Tap = tap;
    m_TapHalf = tap/2;
    m_fs = fs;
	::MakeFilter(m_pH, tap, type, fs, fcl, fch, att, gain);
}
//---------------------------------------------------------------------------
void __fastcall CFIR2::Create(int tap, double fs, double fcl, double fch)
{
	if( (m_Tap != tap) || !m_pZ || !m_pH ){
		if( m_pZ ) delete[] m_pZ;
		m_pZ = new double[(tap+1)*2];
		memset(m_pZ, 0, sizeof(double)*(tap+1)*2);
		if( m_pH ) delete[] m_pH;
		m_pH = new double[tap+1];
		m_W = 0;
    }
	m_Tap = tap;
    m_TapHalf = tap/2;
    m_fs = fs;
	::MakeHilbert(m_pH, tap, fs, fcl, fch);
}
//---------------------------------------------------------------------------
void __fastcall CFIR2::CreateSamp(int tap, double fs, const double *pSmpFQ)
{
	if( (m_Tap != tap) || !m_pZ || !m_pH ){
		if( m_pZ ) delete[] m_pZ;
		m_pZ = new double[(tap+1)*2];
		memset(m_pZ, 0, sizeof(double)*(tap+1)*2);
		if( m_pH ) delete[] m_pH;
		m_pH = new double[tap+1];
        m_W = 0;
    }

    int htap = tap/2;
    int i, j;
    double *pSamp = new double[tap+1];
    memcpy(pSamp, pSmpFQ, sizeof(double)*(tap/2));
    for( i = 0; i < tap/2; i++ ){
		pSamp[tap-i] = pSamp[i];
    }
    pSamp[tap/2] = pSamp[tap/2 - 1];
    double *pH = new double[tap+1];
    double re, fm;

	for( i = 0; i <= htap; i++ ){
		re = 0.0;
		for( j = 0; j < tap; j++ ){
			fm = 2.0 * PI * double((i*j)%tap)/double(tap);
			re += pSamp[j] * cos(fm);
		}
		pH[i] = re / tap;
    }
#if 0
	fm = 0;
    for( i = 0; i <= htap; i++ ){
		fm += pH[i];
    }
    fm = 0.5 / fm;
    for( i = 0; i <= htap; i++ ){
		pH[i] *= fm;
    }
#endif
	for( i = 0; i <= htap; i++ ) m_pH[htap-i] = pH[i];
	for( i = 0; i < htap; i++ ) m_pH[tap-i] = m_pH[i];
	delete[] pH;
    delete[] pSamp;

	m_Tap = tap;
    m_TapHalf = htap;
    m_fs = fs;
}
//---------------------------------------------------------------------------
void __fastcall CFIR2::Clear(void)
{
	if( m_pZ ) memset(m_pZ, 0, sizeof(double)*(m_Tap+1)*2);
    m_W = 0;
}
//---------------------------------------------------------------------------
double __fastcall CFIR2::Do(double d)
{
	double *dp1 = &m_pZ[m_W+m_Tap+1];
	m_pZP = dp1;
	*dp1 = d;
    m_pZ[m_W] = d;
    d = 0;
    double *hp = m_pH;
    for( int i = 0; i <= m_Tap; i++ ){
		d += (*dp1--) * (*hp++);
    }
    m_W++;
	if( m_W > m_Tap ) m_W = 0;
    return d;
}
//---------------------------------------------------------------------------
double __fastcall CFIR2::Do(double *hp)
{
    double d = 0;
    double *dp = m_pZP;
    for( int i = 0; i <= m_Tap; i++ ){
		d += (*dp--) * (*hp++);
    }
    return d;
}
//---------------------------------------------------------------------------
void __fastcall CFIR2::Do(CLX &z, double d)
{
	double *dp1 = &m_pZ[m_W+m_Tap+1];
	m_pZP = dp1;
	*dp1 = d;
    m_pZ[m_W] = d;
    d = 0;
    double *hp = m_pH;
    for( int i = 0; i <= m_Tap; i++ ){
		d += (*dp1--) * (*hp++);
    }
    z.j = d;
    z.r = m_pZ[m_W+m_TapHalf+1];
    m_W++;
	if( m_W > m_Tap ) m_W = 0;
}


//---------------------------------------------------------------------------
// CFIRXクラス
__fastcall CFIRX::CFIRX()
{
	m_pZ = NULL;
    m_pH = NULL;
    m_pZP = NULL;
    m_W = 0;
    m_Tap = 0;
    m_fs = 0;
}

//---------------------------------------------------------------------------
__fastcall CFIRX::~CFIRX()
{
	if( m_pZ ) delete[] m_pZ;
	if( m_pH ) delete[] m_pH;
}
//---------------------------------------------------------------------------
void __fastcall CFIRX::Create(int tap, int type, double fs, double fcl, double fch, double att, double gain)
{
	if( (m_Tap != tap) || !m_pZ || !m_pH ){
		if( m_pZ ) delete[] m_pZ;
		m_pZ = new CLX[(tap+1)*2];
		memset(m_pZ, 0, sizeof(CLX)*(tap+1)*2);
	    if( m_pH ) delete[] m_pH;
		m_pH = new double[tap+1];
		m_W = 0;
    }
	m_Tap = tap;
    m_TapHalf = tap/2;
    m_fs = fs;
	::MakeFilter(m_pH, tap, type, fs, fcl, fch, att, gain);
}
//---------------------------------------------------------------------------
void __fastcall CFIRX::Clear(void)
{
	if( m_pZ ) memset(m_pZ, 0, sizeof(CLX)*(m_Tap+1)*2);
    m_W = 0;
}
//---------------------------------------------------------------------------
void __fastcall CFIRX::Do(CLX &d)
{
	CLX *dp1 = &m_pZ[m_W+m_Tap+1];
	m_pZP = dp1;
	*dp1 = d;
    m_pZ[m_W] = d;
	CLX z = 0;
    double *hp = m_pH;
    for( int i = 0; i <= m_Tap; i++, dp1-- ){
		z.r += dp1->r * (*hp);
		z.j += dp1->j * (*hp++);
    }
    m_W++;
	if( m_W > m_Tap ) m_W = 0;
    d = z;
}

/*=============================================================================
  CSlideFFTクラス  スライディング　FFT
=============================================================================*/
#define	SLIDE_WINDOW_COEFF		0.9999
__fastcall CSlideFFT::CSlideFFT(void)
{
	m_Length = 0;
	m_Base = 0;
	m_Tones = 0;
	m_kWindow = 0;
	m_pBase = NULL;
    m_pCur = m_pEnd = NULL;
}

//--------------------------------------------------------------------------
__fastcall CSlideFFT::~CSlideFFT()
{
	if( m_pBase ) delete[] m_pBase;
}

//--------------------------------------------------------------------------
void __fastcall CSlideFFT::Create(int len, int base, int tones)
{
#if LOGFFT
	FILE *fp = fopen("SLIDERFFT.txt", "wt");
	fprintf(fp, "len=%d, base=%d, tones=%d\n", len, base, tones);
	fclose(fp);
#endif
	if( !m_pBase || (len != m_Length) ){
		if( m_pBase ) delete[] m_pBase;
		m_pBase = new CLX[len];
	}

    memset(m_pBase, 0, sizeof(CLX)*len);
    memset(m_tWindow, 0, sizeof(m_tWindow));
    memset(m_tData, 0, sizeof(m_tData));

	m_Length = len;
	m_Base = base;
    m_Tones = tones;

	double k = 2.0 * PI / double(len);
	for(int i = 0; i < tones; i++){
		m_tWindow[i].r = cos((i+base) * k) * SLIDE_WINDOW_COEFF;
		m_tWindow[i].j = sin((i+base) * k) * SLIDE_WINDOW_COEFF;
	}
	m_kWindow = pow(SLIDE_WINDOW_COEFF, len);
    m_pCur = m_pBase;
    m_pEnd = &m_pBase[m_Length];
}

//--------------------------------------------------------------------------
CLX* __fastcall CSlideFFT::Do(const CLX &zIn)
{
	CLX z;

	if( m_pCur >= m_pEnd ) m_pCur = m_pBase;
	z = *m_pCur;
	*m_pCur = zIn;
    m_pCur++;
	z *= m_kWindow;

	CLX *pData = m_tData;
	CLX *pWindow = m_tWindow;
	for( int i = 0; i < m_Tones; i++, pData++ ){
		*pData -= z;
		*pData += zIn;
		*pData *= *pWindow++;
	}
	return m_tData;
}

/*=============================================================================
  CPHASEクラス
=============================================================================*/
__fastcall CPHASE::CPHASE()
{
	m_TONES = 4;
	m_SHIFT = 170.0;
	m_SampleFreq = 11025.0*0.5;
    m_CarrierFreq = 1750;
    m_MixerFreq = 0;
    SetSampleFreq(m_SampleFreq);
}
//--------------------------------------------------------------------------
void __fastcall CPHASE::ShowPara(void)
{
/*
	if( Application->MainForm ){
		char bf[256];
	    sprintf(bf, "Car=%.lf, Shift=%.lf", m_CarrierFreq, m_SHIFT);
		Application->MainForm->Caption = bf;
    }
*/
}
//--------------------------------------------------------------------------
void __fastcall CPHASE::SetSampleFreq(double f)
{
	m_SampleFreq = f;
    Create();
}
//--------------------------------------------------------------------------
void __fastcall CPHASE::Create(void)
{
//	m_fftSHIFT = m_SHIFT * (m_TONES + 1) / m_TONES;
	m_fftSHIFT = m_SHIFT * m_TONES / (m_TONES - 1);
	m_SymbolLen = m_TONES * m_SampleFreq / m_fftSHIFT;
    m_BASEPOINT = int(CPHASE_BASEFREQ * m_TONES / m_fftSHIFT);

	m_MixerFreq = double(m_BASEPOINT) * m_SampleFreq / m_SymbolLen;
	m_VCO.SetSampleFreq(m_SampleFreq);
	m_Hilbert.Create(20, m_SampleFreq, 25.0, m_SampleFreq*0.5 - 25.0);
    m_Hilbert.Clear();
    SetCarrierFreq(m_CarrierFreq);
    m_SlideFFT.Create(int(m_SymbolLen + 0.5), m_BASEPOINT, m_TONES);
	m_AGC.SetSampleFreq(m_SampleFreq);
	m_AGC.SetCarrierFreq(m_CarrierFreq);
//    m_LPF.Create(192, ffBEF, m_SampleFreq, 85.0 - 20, 85.0 + 20, 60, 1.0);
#if LOGFFT
    m_fp = fopen("FFT.txt", "wt");
#endif
//	ShowPara();
}
//--------------------------------------------------------------------------
void __fastcall CPHASE::SetShift(double f)
{
	m_SHIFT = f;
    Create();
}
//--------------------------------------------------------------------------
void __fastcall CPHASE::SetCarrierFreq(double f)
{
	m_CarrierFreq = f;
    m_VCO.SetFreeFreq(m_CarrierFreq - m_MixerFreq);
	m_AGC.SetCarrierFreq(m_CarrierFreq);
//	ShowPara();
}
//--------------------------------------------------------------------------
CLX* __fastcall CPHASE::Do(double d)
{
	m_Hilbert.Do(m_sig, d);					// 複素数化

    CLX z;
	z.r = m_VCO.Do();
    z.j = m_VCO.DoCos();
	z *= m_sig;								// 周波数変換

//	m_LPF.Do(z);
	return m_SlideFFT.Do(z);
}
//--------------------------------------------------------------------------
void __fastcall CPHASE::DoFSK(double d)
{
//	d = m_AGC.Do(d);
	CLX *pFFT = Do(d);
    m_dm = pFFT[0].vAbs();
    m_ds = pFFT[m_TONES-1].vAbs();

#if LOGFFT
    for( int i = 0; i < m_TONES; i++ ){
		if( i ) fprintf(m_fp, ",");
		fprintf(m_fp, "%.0lf", pFFT[i].vAbs());
    }
    fprintf(m_fp, "\n");
#endif
}
//--------------------------------------------------------------------------
void __fastcall DoAvg(double &av, double in, double factor)
{
	av = av * (1.0 - factor) + (in * factor);
}
//---------------------------------------------------------------------------
__fastcall CFAVG::CFAVG()
{
	Create(16);
}
//---------------------------------------------------------------------------
void __fastcall CFAVG::Reset(void)
{
	m_Cnt = 0;
    m_Sum = 0;
    m_Avg = 0;
}
//---------------------------------------------------------------------------
void __fastcall CFAVG::Reset(double d)
{
	m_Cnt = m_Max;
    m_Sum = d * m_Max;
    m_Avg = d;
}
//---------------------------------------------------------------------------
void __fastcall CFAVG::Create(int max)
{
	m_Max = max;
    m_Mul = 1.0 / m_Max;
    Reset();
}
//---------------------------------------------------------------------------
double __fastcall CFAVG::DoZ(double d)
{
	m_Sum += d;
    if( m_Cnt < m_Max ){
		m_Cnt++;
    }
    else {
		m_Sum -= m_Avg;
    }
    m_Avg = m_Sum * m_Mul;
    return m_Avg;
}
//---------------------------------------------------------------------------
double __fastcall CFAVG::Do(double d)
{
	m_Sum += d;
    if( m_Cnt < m_Max ){
		m_Cnt++;
	    m_Avg = m_Sum / m_Cnt;
    }
    else {
		m_Sum -= m_Avg;
	    m_Avg = m_Sum * m_Mul;
    }
    return m_Avg;
}

//--------------------------------------------------------
// CAGCクラス
__fastcall CAGCX::CAGCX()
{
	m_fc = 1000.0;
	m_MonitorFreq = SampFreq/2048;
    m_SampleFreq = SampFreq;
	m_CarrierFreq = 1750.0;
    m_LimitGain = 0.005;
    m_AvgOver.Create(4);
	Create();
}
//--------------------------------------------------------
void __fastcall CAGCX::Create(void)
{
	m_Count = 0;
	m_Max = -1.0;
	m_Min = 1.0;
	m_d = 0;
	m_agc = 1.0;
//    m_Level.Create(ffLPF, 0.3, m_MonitorFreq, 3, 0, 0);
	SetCarrierFreq(m_CarrierFreq);
}
//--------------------------------------------------------
void __fastcall CAGCX::SetCarrierFreq(double f)
{
	m_CarrierFreq = f;
	if( m_CarrierFreq >= 1000.0 ){
	    m_Gain = (m_CarrierFreq / 1000.0);

/*
500		0.003		333
1000	0.0035		285		1.0		1.0
1500	0.005		200		1.4		1.5
1800	0.006		166		1.71
2000	0.008		125     2.3		2.0
2100	0.01		100		2.85	2.1
2200	0.012       83      3.4		2.2
2300	0.015		66      4.3     2.3
2400	0.025       40      7.1		2.4
2500	0.028		36      7.9		2.5
2600	0.030       33      8.6		2.6
*/
        if( m_Gain >= 2.35 ){
			m_Gain *= 7.1 / 2.4;
        }
		else if( m_Gain >= 2.25 ){
			m_Gain *= 4.3 / 2.3;
        }
        else if( m_Gain >= 2.15 ){
        	m_Gain *= 3.4 / 2.2;
        }
		else if( m_Gain >= 2.05 ){
			m_Gain *= 2.9 / 2.1;
        }
        else if( m_Gain >= 1.8 ){
			m_Gain *= 2.3 / 2.2;
        }
    }
    else {
		m_Gain = 1.0;
    }
    SetFC(m_fc);
}
//--------------------------------------------------------
void __fastcall CAGCX::SetFC(double fc)
{
	m_fc = fc;
    if( fc > m_CarrierFreq*0.45 ) fc = m_CarrierFreq*0.45;
//    m_LPF.Create(ffLPF, fc, m_CarrierFreq, 1, 0, 0);
    m_LPF.MakeIIR(fc, m_CarrierFreq, 1, 0, 0);
    m_d = 0;
    m_TLimit = m_SampleFreq*0.8/m_CarrierFreq;
    m_AvgOver.Reset(1.0);
}
//--------------------------------------------------------
void __fastcall CAGCX::Reset(void)
{
	m_Max = -1.0;
	m_Min = 1.0;
	m_agc = 1.0;
    m_d = 0;
    m_Count = 0;
    for( int i = 0; i < 12; i++ ){
		m_LPF.Do(1.0);
//    	m_Level.Do(0);
    }
    m_AvgOver.Reset(1.0);
}
//--------------------------------------------------------
double __fastcall CAGCX::Do(double d)
{
	if( m_Max < d ) m_Max = d;
	if( m_Min > d ) m_Min = d;
	if( (d >= 0) && (m_d < 0) && (m_Count >= m_TLimit) ){
		double amp = m_Max - m_Min;
        if( amp > 0.1 ){
		    m_agc = m_LPF.Do(5.0/amp);
    	    if( m_agc >= 1.0 ) m_agc = 1.0;
			m_Max = -32768.0;
			m_Min = 32768.0;
        }
        m_Count = 0;
	}
    m_Count++;
	m_d = d;
	d *= m_agc;
    if( d > 2.5 ){ d = 2.5; } else if( d < -2.5 ){ d = -2.5; }
    return d;
}
//--------------------------------------------------------
BOOL __fastcall CAGCX::GetOver(void)
{
// 5.0 / (65536 * 0.666)
	return (m_AvgOver.Do(m_agc) < 0.0001146);
//	return (m_agc < 0.0001146);
}
/********************************************************
CAMPCONT class by JE3HHT on Sep.2010
*********************************************************/
__fastcall CAMPCONT::CAMPCONT()
{
    m_Max = g_SinTable.m_Size / 4;
    m_iMax = g_SinTable.m_Size / 4;
    m_Cnt = m_Max;
    m_S = 0;
    SetMax(16);
}
//---------------------------------------------------------------------------
void __fastcall CAMPCONT::SetMax(int max)
{
    m_ADD = m_Max / double(max);
}
//---------------------------------------------------------------------------
void __fastcall CAMPCONT::Reset(void)
{
	m_Cnt = m_Max;
    m_S = 0;
}
//---------------------------------------------------------------------------
double __fastcall CAMPCONT::Do(int s)
{
	if( s != m_S ){
		m_Cnt = 0.0;
	    m_S = s;
    }
	int r = m_Cnt;
    if( r >= m_Max ) return s;
    m_Cnt += m_ADD;
    if( s ){
		return g_SinTable.m_tSin[r];
    }
    else {
		return g_SinTable.m_tSin[r+m_iMax];
    }
}
/********************************************************
CAA6YQ class by JE3HHT on Sep.2010
*********************************************************/
__fastcall CAA6YQ::CAA6YQ(void)
{
	m_fEnabled = FALSE;
   	m_bpfTaps = 512;
	m_bpfFW = 35.0;
    m_befTaps = 256;
    m_befFW = 15.0;
    m_afcERR = 5.0;

	m_dblMark = 2125.0;
    m_dblSpace = 2295.0;
	m_dblMarkAFC = m_dblMark;
    m_dblSpaceAFC = m_dblSpace;
	Create();
}
//--------------------------------------------------------
void __fastcall CAA6YQ::Create(void)
{
    m_BPF.Create(m_bpfTaps, ffBPF, SampFreq, m_dblMark-m_bpfFW, m_dblSpace+m_bpfFW, 60.0, 1.0);
	double fc = (m_dblMark + m_dblSpace)/2.0;
    m_BEF.Create(m_befTaps, ffBEF, SampFreq, fc-m_befFW, fc+m_befFW, 10.0, 1.0);
}
//--------------------------------------------------------
void __fastcall CAA6YQ::SetMarkFreq(double f)
{
	m_dblMark = f;
	m_dblMarkAFC = f;
	if( m_fEnabled ) Create();
}
//--------------------------------------------------------
void __fastcall CAA6YQ::SetSpaceFreq(double f)
{
	m_dblSpace = f;
    m_dblSpaceAFC = f;
	if( m_fEnabled ) Create();
}
//--------------------------------------------------------
void __fastcall CAA6YQ::SetMarkFreqByAFC(double f)
{
	m_dblMark = f;
	double df = fabs(f - m_dblMarkAFC);
    if( df >= m_afcERR ){
		m_dblMarkAFC = f;
		if( m_fEnabled ) Create();
    }
}
//--------------------------------------------------------
void __fastcall CAA6YQ::SetSpaceFreqByAFC(double f)
{
	m_dblSpace = f;
	double df = fabs(f - m_dblSpaceAFC);
    if( df >= m_afcERR ){
		m_dblSpaceAFC = f;
		if( m_fEnabled ) Create();
    }
}
//--------------------------------------------------------
double __fastcall CAA6YQ::Do(double d)
{
	return m_BEF.Do(m_BPF.Do(d));
}
//--------------------------------------------------------



