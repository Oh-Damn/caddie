import type { ReactNode } from 'react';
import { CompanionProvider as Provider } from './useCompanion';

export function CompanionProvider({ children }: { children: ReactNode }) {
  return <Provider>{children}</Provider>;
}
