import { beforeEach, describe, expect, it } from 'vitest';
import { PERSONAL_KEY, readPersonal, savePersonal } from './personal';

beforeEach(() => {
  localStorage.clear();
});

describe('i dati personali', () => {
  it('leggono quelli che la pagina PSK·RTTY·CW salvava gia (nome, QTH e RST)', () => {
    localStorage.setItem(PERSONAL_KEY, JSON.stringify({ myname: 'Mario', myqth: 'Padova', rsts: '579', rstr: '589' }));
    expect(readPersonal()).toEqual({ myname: 'Mario', mysurname: '', myqth: 'Padova', myemail: '', rsts: '579', rstr: '589' });
  });

  it('cambiando un campo dalle Impostazioni non si perdono gli altri', () => {
    localStorage.setItem(PERSONAL_KEY, JSON.stringify({ myname: 'Mario', myqth: 'Padova', rsts: '579', rstr: '589' }));
    savePersonal({ mysurname: 'Rossi', myemail: 'iu3abc@example.org' });
    expect(JSON.parse(localStorage.getItem(PERSONAL_KEY) ?? '{}')).toEqual({ myname: 'Mario', mysurname: 'Rossi', myqth: 'Padova', myemail: 'iu3abc@example.org', rsts: '579', rstr: '589' });
  });

  it('un valore rovinato torna ai valori di base', () => {
    localStorage.setItem(PERSONAL_KEY, '{rotto');
    expect(readPersonal()).toEqual({ myname: '', mysurname: '', myqth: '', myemail: '', rsts: '599', rstr: '599' });
  });
});
