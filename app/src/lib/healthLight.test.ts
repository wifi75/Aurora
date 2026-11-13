import { describe, expect, it } from 'vitest';
import type { HealthResources, ResourceSample } from '../api/client';
import type { Issue } from './diagnosis';
import { cpuLevel, decodeLevel, decodeSlowOften, healthRows, issueLevel, overall } from './healthLight';

const samples = (cpus: number[], decode = 0, period = 15000): ResourceSample[] => cpus.map((c, i) => ({ t: i * 5000, cpu_pct: c, decode_ms: decode, period_ms: period }));
const res = (over: Partial<HealthResources> = {}): HealthResources => ({ cpu_pct: 0, mem_mb: 200, cores: 4, decode_threads: 3, last_decode_ms: 900, period_ms: 15000, last_decode_at: 0, history: samples([5, 5, 5]), ...over });
const issue = (id: Issue['id'], severity: Issue['severity']): Issue => ({ id, severity, vars: {} });

describe('spia di salute', () => {
  it('CPU: ambra e rosso solo se durano 10 s', () => {
    expect(cpuLevel(res({ history: samples([10, 60, 60]) }))).toBe('ok');
    expect(cpuLevel(res({ history: samples([10, 60, 60, 60]) }))).toBe('warn');
    expect(cpuLevel(res({ history: samples([90, 90, 90]) }))).toBe('bad');
    expect(cpuLevel(res({ history: samples([90, 90, 30]) }))).toBe('ok');
    expect(cpuLevel(res({ history: [] }))).toBe('none');
  });

  it('decodifica: oltre il 70 % ambra, oltre il periodo rosso', () => {
    expect(decodeLevel(res({ last_decode_ms: 900 }))).toBe('ok');
    expect(decodeLevel(res({ last_decode_ms: 11000 }))).toBe('warn');
    expect(decodeLevel(res({ last_decode_ms: 16000 }))).toBe('bad');
    expect(decodeLevel(res({ last_decode_ms: null }))).toBe('none');
  });

  it('avvisi di audio, CAT e orologio', () => {
    const issues = [issue('audio_loss', 'warning'), issue('port_busy', 'problem')];
    expect(issueLevel(issues, 'audio')).toBe('warn');
    expect(issueLevel(issues, 'cat')).toBe('bad');
    expect(issueLevel(issues, 'clock')).toBe('ok');
  });

  it('il colore complessivo e il peggiore', () => {
    expect(overall(healthRows(res(), []))).toBe('ok');
    expect(overall(healthRows(null, []))).toBe('ok');
    expect(overall(healthRows(res({ last_decode_ms: 11000 }), []))).toBe('warn');
    expect(overall(healthRows(res(), [issue('clock', 'problem')]))).toBe('bad');
  });

  it('consiglia PC lento se la decodifica sfora spesso', () => {
    expect(decodeSlowOften(res({ history: samples(Array<number>(20).fill(5), 12000) }))).toBe(true);
    expect(decodeSlowOften(res({ history: samples(Array<number>(20).fill(5), 2000) }))).toBe(false);
  });
});
