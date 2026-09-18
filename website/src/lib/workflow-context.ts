import { createContext, useContext, type Dispatch, type SetStateAction } from 'react';
import type { WorkflowOptions } from './generator';
export const WorkflowContext = createContext<{ options: WorkflowOptions; setOptions: Dispatch<SetStateAction<WorkflowOptions>> } | null>(null);
export function useWorkflow() {
  const value = useContext(WorkflowContext); if (!value) throw new Error('Workflow provider missing'); return value;
}
