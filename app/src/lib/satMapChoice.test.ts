import { describe, expect, it } from 'vitest';
import { SAT_BUILTIN_MAP } from './satGeo';
import { PHOTO_FULL, PREFERRED_MAP, SAT_SIMPLE_MAP, SAT_TERRA_MAP, loadBorders, photoLevel, resolveMap, saveBorders, selectValue, terraUrls } from './satMapChoice';

describe('scelta della mappa', () => {
  const maps = ['Contour HiRes 8k.jpg', 'mia.png'];

  it('la scelta esplicita vince; la mappa vettoriale di prima resta quella semplice', () => {
    expect(SAT_SIMPLE_MAP).toBe(SAT_BUILTIN_MAP);
    expect(resolveMap(SAT_TERRA_MAP, maps, PREFERRED_MAP)).toEqual({ kind: 'terra', variant: 'bathy' });
    expect(resolveMap(SAT_BUILTIN_MAP, maps, PREFERRED_MAP)).toEqual({ kind: 'simple' });
    expect(resolveMap('mia.png', maps, PREFERRED_MAP)).toEqual({ kind: 'file', name: 'mia.png' });
  });

  it('senza scelta: il file preferito se c’e’, altrimenti la Terra (anche se il file scelto e’ sparito)', () => {
    expect(resolveMap('', maps, 'Contour HiRes')).toEqual({ kind: 'file', name: 'Contour HiRes 8k.jpg' });
    expect(PREFERRED_MAP).toBe('');
    expect(resolveMap('', maps, PREFERRED_MAP)).toEqual({ kind: 'terra', variant: 'bathy' });
    expect(resolveMap('', ['mia.png'], PREFERRED_MAP)).toEqual({ kind: 'terra', variant: 'bathy' });
    expect(resolveMap('', [], PREFERRED_MAP)).toEqual({ kind: 'terra', variant: 'bathy' });
    expect(resolveMap('sparita.png', ['mia.png'], PREFERRED_MAP)).toEqual({ kind: 'terra', variant: 'bathy' });
  });

  it('i nomi riservati non sono nomi di file validi per il core (nessuna barra, nessun «..»)', () => {
    for (const n of [SAT_TERRA_MAP, SAT_SIMPLE_MAP]) expect(/[/\\]|\.\./.test(n)).toBe(false);
  });

  it('il valore della tendina: vuoto se la scelta salvata non e’ valida', () => {
    expect(selectValue(SAT_TERRA_MAP, maps)).toBe(SAT_TERRA_MAP);
    expect(selectValue(SAT_SIMPLE_MAP, maps)).toBe(SAT_SIMPLE_MAP);
    expect(selectValue('mia.png', maps)).toBe('mia.png');
    expect(selectValue('', maps)).toBe('');
    expect(selectValue('sparita.png', maps)).toBe('');
  });
});

describe('foto della Terra', () => {
  it('gli indirizzi sono nella cartella maps, relativi alla base dell’interfaccia', () => {
    const u = terraUrls();
    expect(u.day.endsWith('maps/terra-giorno.jpg')).toBe(true);
    expect(u.night.endsWith('maps/terra-notte.jpg')).toBe(true);
    expect(terraUrls('base').day.endsWith('maps/terra-base-giorno.jpg')).toBe(true);
  });

  it('i due nomi riservati scelgono la Terra con la sua variante', () => {
    expect(resolveMap(':terra', [], '')).toEqual({ kind: 'terra', variant: 'bathy' });
    expect(resolveMap(':terra-base', [], '')).toEqual({ kind: 'terra', variant: 'base' });
  });

  it('il livello e’ il piu’ piccolo che basta; oltre l’ultimo, la foto intera', () => {
    expect(photoLevel(500)).toBe(1024);
    expect(photoLevel(1024)).toBe(1024);
    expect(photoLevel(1025)).toBe(2048);
    expect(photoLevel(4096)).toBe(4096);
    expect(photoLevel(4097)).toBe(PHOTO_FULL);
  });

  it('i confini sono spenti di base e la preferenza si ricorda', () => {
    window.localStorage.removeItem('aurora.sat.borders');
    expect(loadBorders()).toBe(false);
    saveBorders(true);
    expect(loadBorders()).toBe(true);
    saveBorders(false);
    expect(loadBorders()).toBe(false);
  });
});
