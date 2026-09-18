import { createFileRoute } from '@tanstack/react-router';
import Generator from '../components/Generator';
import catalog from '../../data/catalog.json';
import { useWorkflow } from '../lib/workflow-context';
export const Route = createFileRoute('/')({ component: Home });
function Home() { const { options, setOptions } = useWorkflow(); return <Generator workflow={catalog} options={options} onChange={setOptions} />; }
