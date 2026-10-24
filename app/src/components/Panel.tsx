import type { ReactNode } from 'react';

interface Props {
  title: string;
  className?: string;
  children: ReactNode;
}

/** Riquadro con titolo: contenitore comune dei pannelli laterali. */
export function Panel({ title, className = '', children }: Props) {
  return (
    <section className={`panel ${className}`.trim()}>
      <h2 className="panel-title">{title}</h2>
      {children}
    </section>
  );
}
