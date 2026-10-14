import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AppController } from './app/AppController';
import { MockRepository } from './services/MockRepository';
import { desktopRuntime, TauriRepository } from './services/TauriRepository';
import type { ProjectRepository } from './domain/models';
import './styles/app.css';

export default function App({ repository }: { repository?: ProjectRepository }) {
  const [repo] = useState(() => repository ?? (desktopRuntime() ? new TauriRepository() : new MockRepository()));
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } }));
  return <QueryClientProvider client={client}><AppController repository={repo} /></QueryClientProvider>;
}
