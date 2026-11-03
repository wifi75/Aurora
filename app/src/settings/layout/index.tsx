import { useState, type CSSProperties, type ReactNode } from 'react';
import { createPortal } from 'react-dom';

/**
 * L'icona «?» con la spiegazione di una voce: passandoci sopra (o con la tastiera) la spiegazione compare subito in una nuvoletta.
 * La nuvoletta e' fuori dal flusso (portale in `body`, posizione fissa): dentro l'etichetta veniva tagliata dal suo `overflow: hidden`.
 * Un clic non tocca il comando accanto (la voce e' spesso una `<label>`).
 */
export function Help({ tip }: { tip?: string }) {
  const [pos, setPos] = useState<{ x: number; y: number } | null>(null);
  if (!tip) return null;
  const show = (el: HTMLElement) => {
    const r = el.getBoundingClientRect();
    setPos({ x: r.left + r.width / 2, y: r.top });
  };
  return (
    <>
      {/* il «?» e' disegnato dallo stile, non e' testo: il nome della voce resta «Distanza in miglia», non «Distanza in miglia?» */}
      <span
        className="sx-help"
        aria-hidden="true"
        tabIndex={-1}
        onMouseEnter={(e) => show(e.currentTarget)}
        onMouseLeave={() => setPos(null)}
        onClick={(e) => {
          e.preventDefault();
          if (pos) setPos(null);
          else show(e.currentTarget);
        }}
      />
      {pos
        ? createPortal(
            <div
              className="sx-tip"
              role="tooltip"
              style={{ left: Math.min(Math.max(pos.x, 180), window.innerWidth - 180), top: pos.y }}
            >
              {tip}
            </div>,
            document.body,
          )
        : null}
    </>
  );
}

/*
 * Sistema di layout comune delle schede delle Impostazioni.
 *
 *   <Page>                          pagina a tutta larghezza: 3 colonne (schermo largo), 2 (medio), 1 (telefono)
 *     <Group title span={2}>        riquadro con titolo e cornice; `span` = quante colonne occupa (1, 2 o 3)
 *       <Cells>                     griglia di celle che si adatta alla larghezza del riquadro
 *         <Field label>…</Field>    cella con etichetta a sinistra e controllo a destra, altezza uguale per tutte
 *       </Cells>
 *     </Group>
 *     <Notes>…</Notes>              note lunghe in un riquadro comprimibile (in fondo a una colonna)
 *   </Page>
 *
 * Per riordinare una scheda basta sostituire le liste verticali con questi componenti: nessuna logica cambia.
 */

/** La pagina di una scheda: griglia a colonne che usa tutta la larghezza disponibile. */
export function Page({ children, className = '' }: { children: ReactNode; className?: string }) {
  return <div className={`sx-page ${className}`.trim()}>{children}</div>;
}

/** Un riquadro con titolo e cornice ben visibile. */
export function Group({
  title,
  span = 1,
  rows,
  children,
  aside,
  className = '',
}: {
  title: ReactNode;
  /** Colonne occupate della pagina (1, 2 o 3). */
  span?: 1 | 2 | 3;
  /** Righe occupate nella griglia della pagina (per affiancare riquadri bassi a uno alto). */
  rows?: 2 | 3;
  children: ReactNode;
  /** Qualcosa a destra del titolo (un tasto, un contatore). */
  aside?: ReactNode;
  className?: string;
}) {
  const style: CSSProperties | undefined = rows ? { gridRow: `span ${rows}` } : undefined;
  return (
    <section className={`sx-group sx-span-${span} ${className}`.trim()} style={style}>
      <h3 className="sx-title">
        <span>{title}</span>
        {aside ? <span className="sx-aside">{aside}</span> : null}
      </h3>
      <div className="sx-body">{children}</div>
    </section>
  );
}

/** Griglia di celle: quante colonne dipende dalla larghezza del riquadro (`min` = larghezza minima di una cella, in rem). */
export function Cells({ children, min = 16, className = '' }: { children: ReactNode; min?: number; className?: string }) {
  return (
    <div className={`sx-cells ${className}`.trim()} style={{ '--sx-min': `${min}rem` } as CSSProperties}>
      {children}
    </div>
  );
}

/**
 * Una cella: etichetta a sinistra (al massimo due righe, il testo intero e' nel suggerimento), controllo a destra.
 * E' una `<label>`: un clic sul testo agisce sul controllo.
 * `stack` mette il controllo sotto l'etichetta a tutta larghezza (aree di testo); `wide` occupa tutta la riga.
 */
export function Field({
  label,
  hint,
  children,
  stack = false,
  wide = false,
  changed = false,
  off = false,
  className = '',
}: {
  label: ReactNode;
  hint?: string;
  children: ReactNode;
  stack?: boolean;
  wide?: boolean;
  changed?: boolean;
  off?: boolean;
  className?: string;
}) {
  const cls = ['sx-field', stack ? 'sx-field--stack' : '', wide ? 'sx-field--wide' : '', changed ? 'sx-field--changed' : '', off ? 'sx-field--off' : '', className].filter(Boolean).join(' ');
  // il testo intero dell'etichetta resta nel suggerimento del browser (puo' essere tagliato a due righe); la spiegazione sta nel «?»
  const title = typeof label === 'string' ? label : undefined;
  return (
    <label className={cls} title={title}>
      <span className="sx-label">
        {label}
        <Help tip={hint} />
      </span>
      <span className="sx-control">{children}</span>
    </label>
  );
}

/** Una cella che non e' un controllo (un valore letto, un gruppo di pulsanti): etichetta e contenuto, senza `<label>`. */
export function Cell({ label, hint, children, wide = false, className = '' }: { label?: ReactNode; hint?: string; children: ReactNode; wide?: boolean; className?: string }) {
  return (
    <div className={`sx-field sx-field--plain${wide ? ' sx-field--wide' : ''} ${className}`.trim()}>
      {label ? (
        <span className="sx-label">
          {label}
          <Help tip={hint} />
        </span>
      ) : null}
      <span className="sx-control">{children}</span>
    </div>
  );
}

/** Pulsanti in fila, in fondo a un riquadro. */
export function Actions({ children }: { children: ReactNode }) {
  return <div className="sx-actions">{children}</div>;
}

/** Messaggio di stato o d'errore dentro un riquadro. */
export function Message({ children, kind = 'note' }: { children: ReactNode; kind?: 'note' | 'alert' | 'muted' }) {
  if (kind === 'alert') return <p className="sx-message notice" role="alert">{children}</p>;
  if (kind === 'muted') return <p className="sx-message muted">{children}</p>;
  return <p className="sx-message live-note" role="status">{children}</p>;
}

/** Note lunghe delle pagine: per scelta del proprietario non si mostrano piu' (le spiegazioni restano nei «?» delle voci). */
export function Notes(_props: { children: ReactNode; title?: ReactNode; span?: 1 | 2 | 3 }) {
  return null;
}
