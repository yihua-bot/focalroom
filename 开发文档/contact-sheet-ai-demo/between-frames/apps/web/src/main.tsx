import React from 'react';
import ReactDOM from 'react-dom/client';
import { WebI18nProvider } from './i18n';
import SharedApp from './SharedApp';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
const queryClient = new QueryClient();

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <QueryClientProvider client={queryClient}><WebI18nProvider>
      <SharedApp />
    </WebI18nProvider></QueryClientProvider>
  </React.StrictMode>,
);
