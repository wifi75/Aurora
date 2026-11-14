import { useQuery } from '@tanstack/react-query';
import { getContest, getContestLog } from '../api/contest';

/**
 * Il contest attivo e il suo log, per le pagine dei modi testuali: lo stato si rilegge ogni 10 secondi, il log solo quando cambia il numero
 * dei QSO (cioe' a ogni QSO salvato o corretto), non a ogni giro.
 */
export function useContestLog() {
  const stato = useQuery({ queryKey: ['contest'], queryFn: getContest, refetchInterval: 10_000, retry: false, refetchOnWindowFocus: false }).data;
  const attivo = stato?.attivo ?? null;
  const nome = attivo ? (stato?.regole?.contest.nome ?? stato?.contest.find((c) => c.id === attivo)?.nome ?? attivo) : null;
  const t = stato?.totali;
  const nQso = t ? t.qso + t.doppi + t.incerti : 0;
  const log = useQuery({ queryKey: ['contest-log', attivo, nQso], queryFn: getContestLog, enabled: !!attivo, retry: false, refetchOnWindowFocus: false, staleTime: 60_000 }).data;
  return { attivo, nome, nQso, log: attivo ? (log?.qso ?? null) : null };
}
