/** L'avviso (stesso stile degli avvisi della riga del contest) per un QSO che il log non ha salvato: resta finche' non si cambia nominativo, si salva o si preme Esc. */
export function SaveFailAlert({ msg, compact = false }: { msg: string; compact?: boolean }) {
  return (
    <p className={`ssbc-alert ssbc-alert--dupe${compact ? ' ssbc-alert--compact' : ''}`} role="alert">
      {msg}
    </p>
  );
}
