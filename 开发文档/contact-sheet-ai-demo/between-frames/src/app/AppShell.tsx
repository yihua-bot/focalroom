import type { ReactNode } from 'react';

export function AppShell({ nav, children, inspector, collapsed = false }: { nav: ReactNode; children: ReactNode; inspector?: ReactNode; collapsed?: boolean }) {
  return <div className={`app-shell ${inspector ? '' : 'without-inspector'} ${collapsed ? 'sidebar-collapsed' : ''}`}>
    {nav}<main className="main">{children}</main>{inspector}
  </div>;
}
