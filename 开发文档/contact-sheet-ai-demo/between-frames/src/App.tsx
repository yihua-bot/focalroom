import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AppController } from './app/AppController';
import { TauriRepository } from './services/TauriRepository';
import type { ProjectRepository } from './domain/models';
import './styles/app.css';

export default function App({ repository }: { repository?: ProjectRepository }) {
  const [repo] = useState(() => repository ?? new TauriRepository());
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } }));
  return <QueryClientProvider client={client}><AppController repository={repo} /></QueryClientProvider>;
}
