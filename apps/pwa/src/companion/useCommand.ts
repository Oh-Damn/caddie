import { useCallback } from 'react';
import { useCompanion } from './useCompanion';

export function useCommand() {
  const { command } = useCompanion();
  return useCallback(
    (action: string, value: number | null = null, target: string | null = null) => {
      command(action, value, target);
    },
    [command],
  );
}
