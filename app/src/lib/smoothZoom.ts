import * as L from 'leaflet';

/**
 * Zoom fluido per una mappa Leaflet: la rotella e i pulsanti + e - non saltano di mezzo livello alla volta ma scorrono
 * verso il livello voluto, fotogramma per fotogramma, restando fermi sul punto sotto il cursore (o al centro per i pulsanti).
 * L'animazione di Leaflet resta spenta (con `zoomAnimation` attiva i pulsanti e la rotella non cambiavano il livello): qui ogni
 * fotogramma sposta la vista come lo zoom a due dita di Leaflet (`pinch`): le tessere gia' presenti si ingrandiscono o si riducono senza
 * essere scartate e ricaricate (prima, a ogni fotogramma, si vedeva lo sfondo blu finche' non arrivavano quelle nuove); le tessere del
 * livello giusto si caricano una volta sola, alla fine. Con un piccolo avvicinamento esponenziale al livello finale.
 * Restituisce la funzione che toglie tutto.
 */
type Internals = L.Map & {
  _move?: (center: L.LatLng, zoom: number, data?: object) => L.Map;
  _resetView?: (center: L.LatLng, zoom: number) => L.Map;
  _limitCenter?: (center: L.LatLng, zoom: number, bounds?: L.LatLngBoundsExpression) => L.LatLng;
};

/** Il centro che tiene fermo il punto `at` (pixel del contenitore) passando al livello `zoom`, come `Map.setZoomAround`. */
function centerAround(map: L.Map, at: L.Point, zoom: number): L.LatLng {
  const scale = map.getZoomScale(zoom);
  const half = map.getSize().divideBy(2);
  const offset = at.subtract(half).multiplyBy(1 - 1 / scale);
  return map.containerPointToLatLng(half.add(offset));
}
export function enableSmoothZoom(map: L.Map, options: { pxPerLevel?: number; ease?: number } = {}): () => void {
  const pxPerLevel = options.pxPerLevel ?? 160;
  const ease = options.ease ?? 0.2;
  const el = map.getContainer();
  let target = map.getZoom();
  let anchor: L.Point | null = null;
  let raf = 0;

  const clamp = (z: number) => Math.min(map.getMaxZoom(), Math.max(map.getMinZoom(), z));

  const inner = map as Internals;
  const move = (zoom: number, final: boolean) => {
    const at = anchor ?? map.getSize().divideBy(2);
    let center = centerAround(map, at, zoom);
    if (inner._limitCenter) center = inner._limitCenter(center, zoom, map.options.maxBounds);
    if (!inner._move || !inner._resetView) {
      map.setZoomAround(at, zoom, { animate: false });
      return;
    }
    // durante lo scorrimento solo lo spostamento «a pizzico» (nessuna tessera scartata); alla fine si ricarica il livello giusto
    if (final) inner._resetView(center, zoom);
    else inner._move(center, zoom, { pinch: true, round: false });
  };

  const step = () => {
    raf = 0;
    const cur = map.getZoom();
    const goal = clamp(target);
    const diff = goal - cur;
    if (!Number.isFinite(diff) || Math.abs(diff) < 0.004) {
      if (Number.isFinite(goal)) move(goal, true);
      target = map.getZoom();
      return;
    }
    move(cur + diff * ease, false);
    raf = window.requestAnimationFrame(step);
  };

  const start = (z: number, at: L.Point | null) => {
    // se non si sta gia' animando, si riparte dal livello vero (puo' essere cambiato con il ridimensionamento)
    if (!raf) target = map.getZoom();
    target = clamp(z);
    anchor = at;
    if (!raf) raf = window.requestAnimationFrame(step);
  };

  const onWheel = (e: WheelEvent) => {
    e.preventDefault();
    // righe e pagine (alcuni mouse) valgono molto piu' dei pixel
    const unit = e.deltaMode === 1 ? 33 : e.deltaMode === 2 ? 400 : 1;
    const dz = Math.max(-1.5, Math.min(1.5, (-e.deltaY * unit) / pxPerLevel));
    if (dz === 0) return;
    start((raf ? target : map.getZoom()) + dz, map.mouseEventToContainerPoint(e));
  };
  el.addEventListener('wheel', onWheel, { passive: false });

  // pulsanti + e - (e la tastiera, se attiva): stessa scorrevolezza, attorno al centro
  const zoomIn = (delta?: number) => start((raf ? target : map.getZoom()) + (delta ?? map.options.zoomDelta ?? 1), null);
  const zoomOut = (delta?: number) => start((raf ? target : map.getZoom()) - (delta ?? map.options.zoomDelta ?? 1), null);
  const original = { zoomIn: map.zoomIn, zoomOut: map.zoomOut };
  map.zoomIn = ((delta?: number) => (zoomIn(delta), map)) as typeof map.zoomIn;
  map.zoomOut = ((delta?: number) => (zoomOut(delta), map)) as typeof map.zoomOut;

  return () => {
    el.removeEventListener('wheel', onWheel);
    if (raf) window.cancelAnimationFrame(raf);
    raf = 0;
    map.zoomIn = original.zoomIn;
    map.zoomOut = original.zoomOut;
  };
}
