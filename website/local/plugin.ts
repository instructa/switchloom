import type { Plugin } from 'vite';
import { Readable } from 'node:stream';
import { createLocalServer } from './server.ts';
export function localPlugin(): Plugin {
  return { name: 'switchloom-local', configResolved(config) { if (config.server.host !== '127.0.0.1') throw new Error('Local benchmarks must bind to 127.0.0.1.'); }, async configureServer(server) {
    const local = await createLocalServer();
    server.httpServer?.once('close', () => local.close());
    server.middlewares.use(async (req, res, next) => {
      if (!req.url?.startsWith('/api/')) return next();
      if (!['127.0.0.1', '::1', '::ffff:127.0.0.1'].includes(req.socket.remoteAddress ?? '')) { res.statusCode = 403; res.end('Loopback required.'); return; }
      const controller = new AbortController(); res.once('close', () => controller.abort());
      try {
        const headers = new Headers(); for (const [key, value] of Object.entries(req.headers)) if (value) headers.set(key, Array.isArray(value) ? value.join(',') : value);
        const request = new Request(`http://${req.headers.host}${req.url}`, { method: req.method, headers, signal: controller.signal,
          ...(req.method !== 'GET' && req.method !== 'HEAD' ? { body: Readable.toWeb(req) as ReadableStream<Uint8Array>, duplex: 'half' } : {}) } as RequestInit);
        const response = await local.fetch(request);
        res.statusCode = response.status; response.headers.forEach((v, k) => res.setHeader(k, v));
        if (!response.body) { res.end(); return; }
        const body = Readable.fromWeb(response.body as any); body.on('error', () => res.destroy()); body.pipe(res);
        res.once('close', () => body.destroy());
      } catch { if (!res.headersSent) { res.statusCode = 500; res.end('Local request failed.'); } else res.destroy(); }
    });
  } };
}
