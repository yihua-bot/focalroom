import React from 'react';
import ReactDOM from 'react-dom/client';
import { PreviewWorkspace } from '@betweenframes/ui';
import './preview.css';
import SharedApp from './SharedApp';

ReactDOM.createRoot(document.getElementById('root')!).render(<React.StrictMode>{location.pathname.startsWith('/shared') ? <SharedApp /> : <PreviewWorkspace />}</React.StrictMode>);
