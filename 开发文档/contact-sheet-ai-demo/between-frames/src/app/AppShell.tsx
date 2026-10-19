import type { ReactNode } from 'react';

export function AppShell({ nav, children, inspector, collapsed = false, assistant = false }: { nav: ReactNode; children: ReactNode; inspector?: ReactNode; collapsed?: boolean; assistant?: boolean }) {
  return <div className={`app-shell ${inspector ? '' : 'without-inspector'} ${collapsed ? 'sidebar-collapsed' : ''} ${assistant && inspector ? 'with-assistant' : ''}`}>
    {nav}<main className="main">{children}</main>{inspector}
  </div>;
}
