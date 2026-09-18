export const LIMITS = { session: 10, ipDay: 100, ipMinute: 6, global: 10_000 };
export type Counter = { count: number; reset: number };
export function quotaPlan(session: string, ip: string, now: number, current: Map<string, Counter>) {
  const dayEnd = (Math.floor(now / 86400000) + 1) * 86400000;
  const minuteEnd = (Math.floor(now / 60000) + 1) * 60000;
  const buckets = [
    { key: `s:${session}`, max: LIMITS.session, reset: dayEnd },
    { key: `d:${ip}`, max: LIMITS.ipDay, reset: dayEnd },
    { key: `m:${ip}`, max: LIMITS.ipMinute, reset: minuteEnd },
    { key: 'global', max: LIMITS.global, reset: dayEnd },
  ];
  let retryAfter = 0;
  const updates: Record<string, Counter> = {};
  for (const b of buckets) {
    const old = current.get(b.key); const count = old && old.reset > now ? old.count : 0;
    if (count >= b.max) retryAfter = Math.max(retryAfter, Math.ceil((b.reset - now) / 1000));
    updates[b.key] = { count: count + 1, reset: b.reset };
  }
  return { allowed: retryAfter === 0, retryAfter, remaining: retryAfter ? 0 : LIMITS.session - updates[`s:${session}`].count, updates };
}
