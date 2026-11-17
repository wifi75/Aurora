/*
 * Codici dei paesi: da numerico ISO 3166-1 (l'`id` delle terre di `world-atlas`) ad alpha-2, e il nome nella lingua dell'interfaccia
 * (`Intl.DisplayNames`). Le terre che non hanno un codice (Kosovo, Somaliland...) o che dividono il codice con un altro paese
 * (Ashmore e Cartier, con il 036 dell'Australia) restano con il nome inglese di Natural Earth, scritto per esteso.
 */

/** Numerico (3 cifre) seguito da alpha-2, tutti di fila. */
const RAW =
  '004AF 008AL 010AQ 012DZ 016AS 020AD 024AO 028AG 031AZ 032AR 036AU 040AT 044BS 048BH 050BD 051AM 052BB 056BE 060BM 064BT 068BO 070BA 072BW 074BV 076BR 084BZ 086IO 090SB 092VG 096BN 100BG 104MM 108BI 112BY 116KH 120CM 124CA 132CV 136KY 140CF 144LK 148TD 152CL 156CN 158TW 162CX 166CC 170CO 174KM 175YT 178CG 180CD 184CK 188CR 191HR 192CU 196CY 203CZ 204BJ 208DK 212DM 214DO 218EC 222SV 226GQ 231ET 232ER 233EE 234FO 238FK 239GS 242FJ 246FI 248AX 250FR 254GF 258PF 260TF 262DJ 266GA 268GE 270GM 275PS 276DE 288GH 292GI 296KI 300GR 304GL 308GD 312GP 316GU 320GT 324GN 328GY 332HT 334HM 336VA 340HN 344HK 348HU 352IS 356IN 360ID 364IR 368IQ 372IE 376IL 380IT 384CI 388JM 392JP 398KZ 400JO 404KE 408KP 410KR 414KW 417KG 418LA 422LB 426LS 428LV 430LR 434LY 438LI 440LT 442LU 446MO 450MG 454MW 458MY 462MV 466ML 470MT 474MQ 478MR 480MU 484MX 492MC 496MN 498MD 499ME 500MS 504MA 508MZ 512OM 516NA 520NR 524NP 528NL 531CW 533AW 534SX 535BQ 540NC 548VU 554NZ 558NI 562NE 566NG 570NU 574NF 578NO 580MP 581UM 583FM 584MH 585PW 586PK 591PA 598PG 600PY 604PE 608PH 612PN 616PL 620PT 624GW 626TL 630PR 634QA 638RE 642RO 643RU 646RW 652BL 654SH 659KN 660AI 662LC 663MF 666PM 670VC 674SM 678ST 682SA 686SN 688RS 690SC 694SL 702SG 703SK 704VN 705SI 706SO 710ZA 716ZW 724ES 728SS 729SD 732EH 740SR 744SJ 748SZ 752SE 756CH 760SY 762TJ 764TH 768TG 772TK 776TO 780TT 784AE 788TN 792TR 795TM 796TC 798TV 800UG 804UA 807MK 818EG 826GB 831GG 832JE 833IM 834TZ 840US 850VI 854BF 858UY 860UZ 862VE 876WF 882WS 887YE 894ZM';

const BY_NUMERIC = new Map<string, string>(RAW.split(' ').map((s) => [s.slice(0, 3), s.slice(3)] as [string, string]));

/** Terre senza codice proprio (o con il codice di un altro paese): il codice a mano, se esiste, per nome inglese di Natural Earth. */
const BY_NAME: Record<string, string> = { Kosovo: 'XK' };

/** Il nome inglese per esteso delle terre senza codice (e delle abbreviazioni di Natural Earth che restano come ripiego). */
const ENGLISH: Record<string, string> = {
  'N. Cyprus': 'Northern Cyprus',
  'Indian Ocean Ter.': 'Indian Ocean Territories',
  'Ashmore and Cartier Is.': 'Ashmore and Cartier Islands',
  Somaliland: 'Somaliland',
  'Siachen Glacier': 'Siachen Glacier',
};

/** Le terre che dividono il codice con un altro paese e non devono prendere il suo nome. */
const SHARED_CODE = new Set(['Ashmore and Cartier Is.']);

/** Il codice alpha-2 di una terra: dal numerico ISO, altrimenti dal nome; `undefined` se non ne ha. */
export function alpha2Of(id: string | number | undefined, englishName: string): string | undefined {
  if (SHARED_CODE.has(englishName)) return undefined;
  if (id !== undefined && id !== '') {
    const code = BY_NUMERIC.get(String(id).padStart(3, '0'));
    if (code) return code;
  }
  return BY_NAME[englishName];
}

const namers = new Map<string, Intl.DisplayNames | null>();

function namerFor(lang: string): Intl.DisplayNames | null {
  let n = namers.get(lang);
  if (n === undefined) {
    try {
      n = typeof Intl !== 'undefined' && typeof Intl.DisplayNames === 'function' ? new Intl.DisplayNames([lang], { type: 'region' }) : null;
    } catch {
      n = null;
    }
    namers.set(lang, n);
  }
  return n;
}

/** Il nome di un paese nella lingua dell'interfaccia; se la lingua non lo conosce, il nome inglese (per esteso). Mai vuoto. */
export function countryName(id: string | number | undefined, englishName: string, lang: string): string {
  const fallback = ENGLISH[englishName] ?? englishName;
  const code = alpha2Of(id, englishName);
  if (code) {
    try {
      const name = namerFor(lang)?.of(code);
      if (name && name !== code) return name;
    } catch {
      /* codice non riconosciuto da questa lingua: ripiego sull'inglese */
    }
  }
  return fallback.trim() === '' ? englishName || String(id ?? '') : fallback;
}
