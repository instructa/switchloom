import { createFileRoute } from '@tanstack/react-router';
import Playground from '../components/Playground';
export const Route = createFileRoute('/playground')({ head: () => ({ meta: [{ title: 'Playground — Switchloom' }] }), component: Playground });
