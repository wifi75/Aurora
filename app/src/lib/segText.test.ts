import { describe, expect, it } from 'vitest';
import { segTextCssUrl, segTextSvg } from './segText';

const STYLE = { on: '#fff', off: '#111', glow: '#bae6fd' };

describe('segText', () => {
  it('draws 14 segments per letter, lit and unlit, and a mark for accented letters', () => {
    const svg = segTextSvg('RÄ', STYLE);
    expect(svg.match(/<line /g)?.length).toBe(28);
    expect(svg).toContain('<circle');
  });

  it('becomes a CSS url and skips unknown characters without breaking', () => {
    expect(segTextCssUrl('RADIO OFF', STYLE)).toMatch(/^url\("data:image\/svg\+xml,/);
    expect(segTextSvg('★', STYLE)).toContain('<svg');
  });
});
