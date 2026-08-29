import type { ReactNode } from 'react';

type LayerProps = {
  active: boolean;
  children: ReactNode;
};

export function Layer({ active, children }: LayerProps) {
  return (
    <div
      className={`screen-layer absolute inset-0 flex min-h-0 flex-col overflow-hidden ${
        active
          ? 'pointer-events-auto z-10 opacity-100'
          : 'pointer-events-none z-0 opacity-0'
      }`}
    >
      {children}
    </div>
  );
}

export function LayerHost({ children }: { children: ReactNode }) {
  return (
    <div className="relative mx-auto min-h-0 w-full max-w-md flex-1 landscape:max-w-4xl">
      {children}
    </div>
  );
}
