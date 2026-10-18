import React from 'react';
import ReactDOM from 'react-dom/client';
import { PreviewWorkspace } from '@betweenframes/ui';
import './preview.css';
import SharedApp from './SharedApp';
import { WebI18nProvider } from './i18n';

ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><WebI18nProvider>{location.pathname.startsWith('/shared') || ['/reset-password', '/verify-email'].includes(location.pathname) ? <SharedApp /> : <PreviewWorkspace />}</WebI18nProvider></React.StrictMode>);
