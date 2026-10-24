import { describe, expect, it } from 'vitest';
import { FULL_VIEW, MIN_W, clampView, panBy, project, zoomAt } from './mapView';

describe('map view', () => {
  it('projects longitude and latitude to the 360 x 180 map', () => {
    expect(project(0, 0)).toEqual({ x: 180, y: 90 });
    expect(project(-180, 90)).toEqual({ x: 0, y: 0 });
    expect(project(180, -90)).toEqual({ x: 360, y: 180 });
  });

  it('keeps the point under the pointer fixed while zooming', () => {
    const v = zoomAt(FULL_VIEW, 0.5, 0.25, 0.5);
    expect(v.w).toBe(180);
    // il punto a un quarto della larghezza e a meta' altezza e' ancora li'
    expect(v.x + 0.25 * v.w).toBeCloseTo(90, 6);
    expect(v.y + 0.5 * (v.w / 2)).toBeCloseTo(90, 6);
  });

  it('does not zoom out past the world or in past the minimum', () => {
    expect(zoomAt(FULL_VIEW, 2, 0.5, 0.5)).toEqual(FULL_VIEW);
    expect(zoomAt({ x: 100, y: 50, w: 20 }, 0.0001, 0.5, 0.5).w).toBe(MIN_W);
  });

  it('keeps the view inside the world when panning', () => {
    expect(panBy({ x: 10, y: 10, w: 90 }, 5, 5)).toMatchObject({ x: 0, y: 0 });
    expect(panBy({ x: 200, y: 100, w: 90 }, -5, -5)).toMatchObject({ x: 270, y: 135 });
    expect(clampView({ x: -50, y: -50, w: 400 })).toEqual(FULL_VIEW);
  });

  it('moves with the drag: dragging right shows what is to the left', () => {
    const v = panBy({ x: 100, y: 40, w: 90 }, 0.1, 0);
    expect(v.x).toBeCloseTo(91, 6);
  });
});
