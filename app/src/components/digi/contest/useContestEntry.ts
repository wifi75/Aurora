import { useQuery } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { getTextContestCheck, type TextContest, type TextContestScore } from '../../../api/contestTesto';
import { getDxcc, getLog4om, getLogQsos, type Log4omQso } from '../../../api/client';
import { autoExchange, callMultipliers, contestSuggestions, previousQsos, HF_BANDS } from '../../../lib/contestEntry';
import { qsoTimeMs } from '../../../lib/logWindow';
import type { Qso } from '../DigiLog';

export interface ContestEntryInput {
  qso: Qso;
  contest: TextContest | null;
  inSet: (q: Log4omQso) => boolean;
  noBand: boolean;
  noMode: boolean;
  /** La banda con cui si registra (vuota = sconosciuta). */
  band: string;
  modeName: string;
  /** Il contest attivo (`null` = nessuno: la riga e' un log normale). */
  contestName: string | null;
  score: TextContestScore | undefined;
  onQso(patch: Partial<Qso>): void;
}

/**
 * Tutto cio' che l'inserimento di un QSO nel contest sa del nominativo scritto: controllo del core (doppio, punti, moltiplicatori nuovi),
 * casella del terzo elemento dello scambio, paese e zone, scambio ricevuto precompilato, suggerimenti, QSO precedenti e moltiplicatori che porta.
 * Sta fuori dalla scheda perche' i moltiplicatori nuovi servono anche alle schede dei moltiplicatori (la casella «da lavorare» pulsa).
 */
export function useContestEntry({ qso, contest, inSet, noBand, noMode, band, modeName, contestName, score, onQso }: ContestEntryInput) {
  const contestOn = contestName !== null;
  // il nominativo si controlla un attimo dopo l'ultimo tasto
  const call = qso.call.trim().toUpperCase();
  const [typed, setTyped] = useState('');
  useEffect(() => {
    const id = window.setTimeout(() => setTyped(call), 220);
    return () => window.clearTimeout(id);
  }, [call]);
  const nrr = (qso.nrr ?? '').trim();
  const check = useQuery({
    queryKey: ['ssb-contest-check', typed, band, nrr],
    queryFn: () => getTextContestCheck(typed, band, modeName, nrr),
    enabled: contestOn && typed.length >= 3 && call === typed && band !== '',
    retry: false,
    staleTime: 0,
    gcTime: 0,
    refetchOnWindowFocus: false,
  });
  // lo stesso log del pannello Log (stessa chiave, quindi la stessa lettura): il doppio si cerca solo nei QSO del contest
  const logQ = useQuery({ queryKey: ['log4om', 500], queryFn: () => getLog4om(500), retry: false, refetchInterval: 10_000 });
  const mine = (logQ.data?.qsos ?? []).find((q) => inSet(q) && q.call.toUpperCase() === call && (noBand || q.band === band) && (noMode || q.mode.toUpperCase() === modeName.toUpperCase()));
  const raw = call === typed && typed.length >= 3 ? check.data : undefined;
  const c = raw ? { ...raw, doppio: mine !== undefined, precedente: mine ? { time_ms: Date.parse(`${mine.date.replace(' ', 'T').slice(0, 19)}Z`), band: mine.band, mode: mine.mode, exch_rcvd: '', exch_sent: '' } : null } : undefined;
  const pending = new Set(c?.nuovi ?? []);
  // la casella del terzo elemento dello scambio (sezione, club, stato) compare solo se il core dice che questo nominativo la da' (stesse regole
  // del punteggio); la risposta si ricorda per il nominativo, cosi' non sparisce mentre si scrive il numero e la richiesta si rifa'
  const [exchFor, setExchFor] = useState<{ call: string; want: boolean }>({ call: '', want: false });
  const exchAtteso = raw ? raw.exch_atteso === true : null;
  useEffect(() => {
    if (exchAtteso !== null) setExchFor({ call: typed, want: exchAtteso });
  }, [exchAtteso, typed]);
  const showExtra = contestOn && exchFor.call === call && exchFor.want;
  const extraNow = qso.extra ?? '';
  const onQsoRef = useRef(onQso);
  onQsoRef.current = onQso;
  useEffect(() => {
    // senza la casella il valore scritto non vale piu' (altro nominativo)
    if (!showExtra && extraNow !== '') onQsoRef.current({ extra: '' });
  }, [showExtra, extraNow]);

  // paese e zone del nominativo (database dei prefissi): suggerimenti, scambio ricevuto precompilato, moltiplicatori che porta
  const known = contestOn && call === typed && typed.length >= 3;
  const dxccQ = useQuery({ queryKey: ['ssb-dxcc', typed], queryFn: () => getDxcc(typed), enabled: known, retry: false, staleTime: 300_000, refetchOnWindowFocus: false });
  const info = known ? dxccQ.data : undefined;
  const suggestions = contest ? contestSuggestions(contest, info) : [];
  const auto = contest ? autoExchange(contest, info) : null;
  // lo scambio ricevuto si precompila solo se e' vuoto o e' ancora quello messo da qui: quello scritto a mano non si tocca
  const autoRef = useRef('');
  const nrrRef = useRef(nrr);
  nrrRef.current = nrr;
  const dxccDone = dxccQ.isSuccess || dxccQ.isError;
  useEffect(() => {
    if (!contestOn) return;
    const cur = nrrRef.current;
    if (call.length < 3) {
      if (autoRef.current !== '' && cur === autoRef.current) onQsoRef.current({ nrr: '' });
      autoRef.current = '';
      return;
    }
    if (call !== typed || !dxccDone || (cur !== '' && cur !== autoRef.current)) return;
    if (auto !== null) {
      if (cur !== auto) onQsoRef.current({ nrr: auto });
      autoRef.current = auto;
    } else {
      if (cur !== '') onQsoRef.current({ nrr: '' });
      autoRef.current = '';
    }
  }, [contestOn, call, typed, auto, dxccDone]);

  // QSO precedenti con il nominativo, una riga per banda: dal log (con gli scambi nel commento), solo quelli del contest in corso
  const logAll = useQuery({ queryKey: ['log-qsos-contest', logQ.data?.total ?? 0], queryFn: getLogQsos, enabled: known, retry: false, staleTime: 5_000, refetchOnWindowFocus: false });
  const prevRows = known
    ? previousQsos(logAll.data?.qsos ?? [], typed, (q) => inSet({ ...q, date: `${new Date(qsoTimeMs(q)).toISOString().slice(0, 19).replace('T', ' ')}Z` }), HF_BANDS)
    : [];
  const callMults = known && score && c ? callMultipliers(score.tipi, info, c.nuovi, score.griglia, band) : [];

  return { contest, call, typed, known, c, pending, showExtra, extraNow, suggestions, prevRows, callMults, logQ };
}

export type ContestEntryState = ReturnType<typeof useContestEntry>;
