import { describe, expect, it } from 'vitest';
import { CLUSTER_CLEAR_PASSWORD } from '../api/client';
import { draftToConfig } from './ClusterTab';

const server = { name: 'Prova', host: 'cluster.example.org', port: 7300, enabled: true };
const base = { enabled: true, callsign: 'iu3abc', servers: [server], login: 'set/ft8\nsh/dx 5', maxSpots: '500', maxAge: '60', spotVia: 'dxsummit' as const, spotServer: '' };

describe('draftToConfig', () => {
  it('turns the tab values into a valid configuration', () => {
    expect(draftToConfig(base)).toEqual({
      enabled: true,
      callsign: 'IU3ABC',
      servers: [{ ...server, username: '', password: '' }],
      login_commands: ['set/ft8', 'sh/dx 5'],
      max_spots: 500,
      max_age_min: 60,
      spot_via: 'dxsummit',
      spot_server: '',
    });
  });

  it('accepts an empty callsign (the core then uses the one of the engine) and no commands', () => {
    expect(draftToConfig({ ...base, callsign: '', login: '' })?.login_commands).toEqual([]);
  });

  it('refuses a bad callsign, host, port or number', () => {
    expect(draftToConfig({ ...base, callsign: 'AB' })).toBeNull();
    expect(draftToConfig({ ...base, callsign: 'NOCIFRA' })).toBeNull();
    expect(draftToConfig({ ...base, servers: [{ ...server, host: '; rm' }] })).toBeNull();
    expect(draftToConfig({ ...base, servers: [{ ...server, port: 0 }] })).toBeNull();
    expect(draftToConfig({ ...base, servers: [{ ...server, name: ' ' }] })).toBeNull();
    expect(draftToConfig({ ...base, maxSpots: '10' })).toBeNull();
    expect(draftToConfig({ ...base, maxAge: '1' })).toBeNull();
  });

  it('refuses more than 8 servers and commands that would publish something', () => {
    expect(draftToConfig({ ...base, servers: Array.from({ length: 9 }, (_, i) => ({ ...server, port: 7000 + i })) })).toBeNull();
    expect(draftToConfig({ ...base, login: 'dx 14074 K1ABC cq' })).toBeNull();
    expect(draftToConfig({ ...base, login: 'ann/full ciao' })).toBeNull();
    expect(draftToConfig({ ...base, login: Array.from({ length: 11 }, () => 'sh/dx').join('\n') })).toBeNull();
  });

  it('keeps login name and password, never sends back has_password, and checks them', () => {
    const c = draftToConfig({ ...base, spotVia: 'telnet', spotServer: 'cluster.example.org:7300', servers: [{ ...server, username: 'iu3abc-2', password: 'abc', has_password: true }] });
    expect(c?.servers[0]).toEqual({ ...server, username: 'IU3ABC-2', password: 'abc' });
    expect(c?.spot_via).toBe('telnet');
    expect(draftToConfig({ ...base, servers: [{ ...server, password: CLUSTER_CLEAR_PASSWORD }] })?.servers[0]?.password).toBe(CLUSTER_CLEAR_PASSWORD);
    expect(draftToConfig({ ...base, servers: [{ ...server, username: 'NOCALL' }] })).toBeNull();
    expect(draftToConfig({ ...base, servers: [{ ...server, password: 'à' }] })).toBeNull();
  });
});
