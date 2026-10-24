/** Nome proposto per il dispositivo, ricavato dal browser: l'utente puo' sempre cambiarlo. */
export function defaultDeviceName(userAgent: string): string {
  if (/iPhone/i.test(userAgent)) return 'iPhone';
  if (/iPad/i.test(userAgent)) return 'iPad';
  if (/Android/i.test(userAgent)) return 'Telefono Android';
  if (/Windows/i.test(userAgent)) return 'PC Windows';
  if (/Macintosh|Mac OS X/i.test(userAgent)) return 'Mac';
  if (/Linux|X11/i.test(userAgent)) return 'PC Linux';
  return 'Browser';
}
