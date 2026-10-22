import { useQuery } from '@tanstack/react-query';
import type { DesktopProjectRepository } from '../../domain/models';
import { Button } from '../../components/shared/Button';
import { taskLabels } from './AIPanel';
export function AgentStatus({ repository, projectId, onOpen }: { repository: DesktopProjectRepository; projectId: string; onOpen: () => void }) {
 const query = useQuery({ queryKey: ['agent', projectId], queryFn: () => repository.agent.query(projectId), refetchInterval: 1000, refetchIntervalInBackground: true, refetchOnWindowFocus: 'always' });
 const task = query.data?.tasks[0];
 if (!task || ['completed','cancelled'].includes(task.state)) return null;
 return <div className="agent-status"><Button size="small" onClick={onOpen}>{taskLabels[task.state]}</Button></div>;
}
