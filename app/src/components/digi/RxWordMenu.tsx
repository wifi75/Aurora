import { useEffect, useRef } from 'react';
import { useT } from '../../i18n';
import { MenuList } from './MenuBar';

/** Dove mettere la parola scelta nella finestra di ricezione: un campo del log del QSO in corso, negli appunti (il testo selezionato o la
 * parola), oppure si svuota la finestra. */
export type WordTarget = 'call' | 'name' | 'qth' | 'grid' | 'rst' | 'copy' | 'clear';

export interface WordAt {
  /** La parola cliccata; `null` fuori dalle parole (allora solo Copia e Cancella tutto). */
  word: string | null;
  x: number;
  y: number;
}

/** La parola sotto il puntatore (lettere, cifre, `/` e `-`), o `null` se il clic e' fuori dalle parole (spazio, margine, punteggiatura). */
export function wordAtPoint(x: number, y: number, box: HTMLElement): string | null {
  type CaretDoc = Document & {
    caretPositionFromPoint?(x: number, y: number): { offsetNode: Node; offset: number } | null;
    caretRangeFromPoint?(x: number, y: number): Range | null;
  };
  const doc = document as CaretDoc;
  let node: Node | null = null;
  let offset = 0;
  if (doc.caretPositionFromPoint) {
    const pos = doc.caretPositionFromPoint(x, y);
    if (pos) ({ offsetNode: node, offset } = pos);
  } else if (doc.caretRangeFromPoint) {
    const range = doc.caretRangeFromPoint(x, y);
    if (range) {
      node = range.startContainer;
      offset = range.startOffset;
    }
  }
  if (!node || node.nodeType !== Node.TEXT_NODE || !box.contains(node)) return null;
  const text = node.textContent ?? '';
  const isWord = (c: string | undefined) => c !== undefined && /[\p{L}\p{N}/-]/u.test(c);
  // il punto del clic puo' cadere subito dopo l'ultima lettera: si guarda anche il carattere prima
  let at = offset;
  if (!isWord(text[at]) && isWord(text[at - 1])) at -= 1;
  if (!isWord(text[at])) return null;
  // il carattere deve stare davvero sotto il puntatore (oltre la fine della riga caretPositionFromPoint da' l'ultimo carattere)
  const r = document.createRange();
  r.setStart(node, at);
  r.setEnd(node, at + 1);
  const rect = r.getBoundingClientRect();
  if (rect.width > 0 && (x < rect.left - 2 || x > rect.right + 2 || y < rect.top - 2 || y > rect.bottom + 2)) return null;
  let start = at;
  let end = at + 1;
  while (start > 0 && isWord(text[start - 1])) start--;
  while (end < text.length && isWord(text[end])) end++;
  const word = text.slice(start, end).replace(/^[-/]+|[-/]+$/g, '');
  return word === '' ? null : word;
}

/** Il menu del tasto destro della finestra di ricezione: sulla parola la mette nel campo scelto del log (nominativo, nome, QTH, locatore,
 * rapporto ricevuto); sempre Copia (il testo selezionato o la parola) e Cancella tutto. Si chiude con un clic fuori, Esc o lo scorrimento. */
export function RxWordMenu({ at, onPick, onClose }: { at: WordAt; onPick(target: WordTarget, word: string): void; onClose(): void }) {
  const t = useT();
  const box = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const close = (e: MouseEvent) => {
      if (!box.current?.contains(e.target as Node)) onClose();
    };
    const esc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    const scroll = () => onClose();
    document.addEventListener('mousedown', close);
    document.addEventListener('keydown', esc);
    window.addEventListener('blur', scroll);
    document.addEventListener('scroll', scroll, true);
    return () => {
      document.removeEventListener('mousedown', close);
      document.removeEventListener('keydown', esc);
      window.removeEventListener('blur', scroll);
      document.removeEventListener('scroll', scroll, true);
    };
  }, [onClose]);

  // il menu resta dentro la finestra anche vicino ai bordi
  const left = Math.min(at.x, window.innerWidth - 260);
  const top = Math.min(at.y, window.innerHeight - 230);
  const word = at.word ?? '';
  const item = (target: WordTarget, label: string) => ({ label, onSelect: () => onPick(target, word) });
  const always = [item('copy', t('digi.word.copy')), item('clear', t('digi.mi.clearRx'))];
  return (
    <div
      className="digi-menu"
      style={{ position: 'fixed', left: Math.max(0, left), top: Math.max(0, top), zIndex: 60 }}
      ref={box}
      onContextMenu={(e) => e.preventDefault()}
      aria-label={at.word ? t('digi.word.menu', { word: at.word }) : t('digi.rx.title')}
    >
      <MenuList
        items={
          at.word
            ? [item('call', t('digi.word.call')), item('name', t('digi.word.name')), item('qth', t('digi.word.qth')), item('grid', t('digi.word.grid')), item('rst', t('digi.word.rst')), 'sep', ...always]
            : always
        }
        onDone={onClose}
      />
    </div>
  );
}
