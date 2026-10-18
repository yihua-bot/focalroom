import React from 'react';
import ReactDOM from 'react-dom/client';
import { WebI18nProvider } from './i18n';
import SharedApp from './SharedApp';

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode>
    <WebI18nProvider>
      <SharedApp />
    </WebI18nProvider>
  </React.StrictMode>,
);
