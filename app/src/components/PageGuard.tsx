import { Component, type ReactNode } from 'react';

/**
 * Un errore dentro una pagina non deve svuotare tutta l'interfaccia: la pagina si rimette in piedi al prossimo cambio di pagina
 * (`resetKey`). Nato dalla Stazione tenuta viva e nascosta: un disegno su una tela larga 0 lasciava la finestra vuota (2026-11-09).
 */
export class PageGuard extends Component<{ children: ReactNode; resetKey: string }, { failed: boolean; key: string }> {
  state = { failed: false, key: this.props.resetKey };

  static getDerivedStateFromError(): Partial<{ failed: boolean }> {
    return { failed: true };
  }

  static getDerivedStateFromProps(props: { resetKey: string }, state: { failed: boolean; key: string }) {
    return props.resetKey !== state.key ? { failed: false, key: props.resetKey } : null;
  }

  componentDidCatch(error: unknown): void {
    console.error('pagina in errore', error);
  }

  render(): ReactNode {
    return this.state.failed ? null : this.props.children;
  }
}
