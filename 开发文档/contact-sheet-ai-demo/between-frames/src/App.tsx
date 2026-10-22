import { useState } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { AppController } from './app/AppController';
import { TauriRepository } from './services/TauriRepository';
import { isDesktopRepository } from './services/TauriRepository';
import { AccountProvider } from './features/cloud-login/AccountProvider';
import type { CloudAuthRepository } from './domain/cloud-auth';
import type { BillingRepository } from './domain/billing';
import type { ProjectRepository } from './domain/models';
import './styles/app.css';

export default function App({ repository, auth, billing }: { repository?: ProjectRepository; auth?: CloudAuthRepository; billing?: BillingRepository }) {
  const [repo] = useState(() => repository ?? new TauriRepository());
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false, staleTime: Infinity } } }));
  const desktop = isDesktopRepository(repo) ? repo : undefined;
  return <QueryClientProvider client={client}><AccountProvider auth={auth ?? desktop?.cloudAuth} billing={billing ?? desktop?.billing}><AppController repository={repo} /></AccountProvider></QueryClientProvider>;
}
