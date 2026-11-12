import { describe, expect, it } from 'vitest';
import { destinationError, parseDestinations, type UdpDestination } from './IntegrationsPanel';

const base: UdpDestination = { name: 'GridTracker', host: '127.0.0.1', port: 2237, enabled: true, wsjtx: true, loggedAdif: false, adif: false, n1mm: false, multicast: false, ttl: 1, iface: '', resend: false };

describe('the UDP destinations of the Integrations box', () => {
  it('reads the saved list and fills the missing fields', () => {
    const list = parseDestinations('[{"host":"239.255.0.1","port":2237,"multicast":true,"wsjtx":true}]');
    expect(list).toHaveLength(1);
    expect(list[0]).toMatchObject({ host: '239.255.0.1', multicast: true, ttl: 1, enabled: true, adif: false });
    expect(parseDestinations('non json')).toEqual([]);
    expect(parseDestinations(undefined)).toEqual([]);
  });

  it('checks addresses, ports and the multicast options like the engine', () => {
    expect(destinationError(base)).toBeNull();
    expect(destinationError({ ...base, host: 'gridtracker' })).toBe('int.err.host');
    expect(destinationError({ ...base, port: 0 })).toBe('int.err.port');
    expect(destinationError({ ...base, host: '239.255.0.1' })).toBe('int.err.unicast');
    expect(destinationError({ ...base, multicast: true })).toBe('int.err.group');
    expect(destinationError({ ...base, host: '239.255.0.1', multicast: true, ttl: 0 })).toBe('int.err.ttl');
    expect(destinationError({ ...base, host: '239.255.0.1', multicast: true, iface: 'eth0' })).toBe('int.err.iface');
    expect(destinationError({ ...base, host: '224.0.0.1', multicast: true, iface: '127.0.0.1' })).toBeNull();
  });
});
