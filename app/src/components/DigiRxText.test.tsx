import { Profiler } from 'react';
import { act, render } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { DigiRxText } from './DigiRxText';

describe('DigiRxText', () => {
  it('underlines the sent lines, shows the uncertain characters apart and hides the sent lines on request', () => {
    const text = 'CQ DE IU3ABC\n» IU3ABC DE K1ABC\nUR 599';
    const at = text.indexOf(' 5');
    const weak = `${'0'.repeat(at)}11${'0'.repeat(text.length - at - 2)}`;
    const { container, rerender } = render(<pre><DigiRxText text={text} weak={weak} underlineSent /></pre>);
    expect(container.textContent).toBe(text);
    expect(container.querySelector('u.digi-sent')?.textContent).toBe('» IU3ABC DE K1ABC\n');
    expect(container.querySelector('.digi-weak')?.textContent).toBe(' 5');
    rerender(<pre><DigiRxText text={text} weak={weak} underlineSent={false} showSent={false} /></pre>);
    expect(container.querySelector('u')).toBeNull();
    expect(container.textContent).toBe('CQ DE IU3ABC\nUR 599');
  });

  it('with a new character keeps the lines in the middle as they are, also when the start of the text is cut', () => {
    let text = Array.from({ length: 300 }, (_, i) => `RIGA ${i}`).join('\n');
    const view = render(<Profiler id="rx" onRender={() => undefined}><pre><DigiRxText text={text} weak={'0'.repeat(text.length)} underlineSent base={0} /></pre></Profiler>);
    const middle = view.container.querySelectorAll('pre > span')[150];
    const lastBefore = view.container.querySelector('pre > span:last-child')?.textContent;
    // un carattere in piu' in fondo, uno in meno all'inizio (il testo resta della stessa lunghezza): la base avanza di uno
    text = `${text.slice(1)}X`;
    act(() => view.rerender(<Profiler id="rx" onRender={() => undefined}><pre><DigiRxText text={text} weak={'0'.repeat(text.length)} underlineSent base={1} /></pre></Profiler>));
    expect(view.container.querySelector('pre > span:last-child')?.textContent).toBe(`${lastBefore}X`);
    expect(view.container.querySelector('pre > span')?.textContent).toBe('IGA 0\n');
    expect(view.container.querySelectorAll('pre > span')).toHaveLength(300);
    // le righe di mezzo sono gli stessi elementi di prima, con lo stesso testo
    expect(view.container.querySelectorAll('pre > span')[150]).toBe(middle);
    expect(middle?.textContent).toBe('RIGA 150\n');
  });
});
