import React from 'react';
import ReactDOM from 'react-dom/client';
import { DesktopWorkspace } from '@betweenframes/ui';
import { I18nProvider } from '../../../src/i18n';

ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode><I18nProvider><DesktopWorkspace /></I18nProvider></React.StrictMode>);
