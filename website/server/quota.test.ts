import { expect, it } from 'vitest';
import { LIMITS, quotaPlan, type Counter } from './quota';
it('enforces session, IP bursts, daily IP and a shared cap without charging denied requests', () => {
  const counters = new Map<string, Counter>(); const now = Date.UTC(2026, 8, 17, 12);
  function reserve(session: string, ip: string, time = now) {
    const p = quotaPlan(session, ip, time, counters); if (p.allowed) for (const entry of Object.entries(p.updates)) counters.set(...entry); return p;
  }
  for (let i = 0; i < LIMITS.ipMinute; i++) expect(reserve('a', 'ip').allowed).toBe(true);
  const before = new Map(counters);
  expect(reserve('a', 'ip')).toMatchObject({ allowed: false, remaining: LIMITS.session - LIMITS.ipMinute, retryAfter: 60 });
  expect(reserve('new-cookie', 'ip')).toMatchObject({ allowed: false, remaining: LIMITS.session });
  expect(counters).toEqual(before);
  for (let i = LIMITS.ipMinute; i < LIMITS.session; i++) expect(reserve('a', 'ip', now + 60000).allowed).toBe(true);
  expect(reserve('a', 'another-ip', now + 60000)).toMatchObject({ allowed: false, remaining: 0 });
  counters.set('global', { count: LIMITS.global, reset: now + 60000 }); expect(reserve('b', 'b').allowed).toBe(false);
  expect(reserve('a', 'ip', now + 86400000).allowed).toBe(true);
});
