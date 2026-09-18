import { createRootRoute, HeadContent, Outlet, Scripts, Link } from '@tanstack/react-router';
import { useState } from 'react';
import { WorkflowContext } from '../lib/workflow-context';
import catalog from '../../data/catalog.json';
import { defaultOptions } from '../lib/generator';
import '../styles/global.css';
import '../styles/board.css';
export const Route = createRootRoute({
  head: () => ({ meta: [{ charSet: 'utf-8' }, { name: 'viewport', content: 'width=device-width, initial-scale=1' },
    { title: 'Switchloom — Your workflow.' }, { name: 'description', content: 'Configure GPT models and capability owners. Explore routing with Jev.' }, { name: 'theme-color', content: '#f7f5ef' }],
    links: [{ rel: 'icon', href: '/brand/codex.svg', type: 'image/svg+xml' }] }),
  component: Root,
  errorComponent: () => <main className="shell"><h1>Could not load this page.</h1><button onClick={() => window.location.reload()}>Reload</button></main>,
  notFoundComponent: () => <main className="shell"><h1>Page not found.</h1><Link to="/">Back to workflow</Link></main>,
});
function Root() {
  const [options, setOptions] = useState(() => defaultOptions(catalog));
  return <html lang="en"><head><HeadContent /></head><body><WorkflowContext.Provider value={{ options, setOptions }}>
    <div className="min-h-svh"><header className="shell header"><Link to="/" className="brand">Switchloom</Link><nav aria-label="Main"><Link to="/" className="nav-link" activeOptions={{ exact: true }}>Workflow</Link><Link to="/playground" className="nav-link">Playground</Link><Link to="/benchmarks" className="nav-link">Benchmarks</Link></nav></header><Outlet /></div>
  </WorkflowContext.Provider><Scripts /></body></html>;
}
