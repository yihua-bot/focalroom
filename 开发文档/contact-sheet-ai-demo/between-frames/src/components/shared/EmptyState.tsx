import { Images } from 'lucide-react';
import type { ReactNode } from 'react';

export function EmptyState({ title, hint, action }: { title: string; hint: string; action?: ReactNode }) {
  return <section className="empty-state"><Images size={32} strokeWidth={1} aria-hidden="true" />
    <h2>{title}</h2><p>{hint}</p>{action}</section>;
}
