import { defineConfig } from 'vite';
import { tanstackStart } from '@tanstack/react-start/plugin/vite';
import react from '@vitejs/plugin-react';
import tailwindcss from '@tailwindcss/vite';
import { cloudflare } from '@cloudflare/vite-plugin';
import { fileURLToPath } from 'node:url';
export default defineConfig(async ({ mode, command }) => {
  const local = mode === 'developer' && command === 'serve';
  if (process.env.DEVELOPER_MODE === 'true' && !local) throw new Error('DEVELOPER_MODE requires the local development entrypoint.');
  if (local && process.env.DEVELOPER_MODE !== 'true') throw new Error('Set DEVELOPER_MODE=true to start local benchmarks.');
  const localPlugin = local ? (await import('./website/local/plugin.ts')).localPlugin() : null;
  return {
    publicDir: 'website/public', cacheDir: `node_modules/.vite/switchloom-${mode}-${command}`,
    resolve: { alias: { '@': fileURLToPath(new URL('./website/src', import.meta.url)) } },
    server: { host: '127.0.0.1', port: 4173, strictPort: true },
    plugins: [localPlugin, !local && cloudflare({ viteEnvironment: { name: 'ssr' } }), tailwindcss(),
      tanstackStart({ srcDirectory: 'website/src' }), react()],
  };
});
