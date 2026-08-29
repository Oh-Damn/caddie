import type { ReactNode } from 'react';

type Props = {
  header: ReactNode;
  dock: ReactNode;
  children: ReactNode;
  split?: boolean;
};

export function HomeLayout({ header, dock, children, split = false }: Props) {
  return (
    <div
      className={`flex min-h-0 w-full min-w-0 flex-1 flex-col gap-1.5 overflow-hidden ${
        split
          ? 'landscape:grid landscape:grid-cols-[minmax(0,3fr)_minmax(0,7fr)] landscape:grid-rows-[auto_minmax(0,1fr)]'
          : ''
      }`}
    >
      <div
        className={`shrink-0 pb-3 landscape:pb-0 ${split ? 'landscape:col-span-2' : ''}`}
      >
        {header}
      </div>
      <div
        className={`min-h-0 min-w-0 flex-1 overflow-y-auto overflow-x-hidden overscroll-y-contain ${
          split ? 'landscape:col-start-2 landscape:row-start-2' : ''
        }`}
      >
        <section className="flex h-full min-h-0 min-w-0 flex-col gap-1.5 pb-1">
          {children}
        </section>
      </div>
      <div
        className={`z-10 flex min-h-0 min-w-0 shrink-0 flex-col gap-1.5 ${
          split
            ? 'landscape:col-start-1 landscape:row-start-2 landscape:h-full landscape:min-h-0 landscape:overflow-hidden'
            : 'landscape:hidden'
        }`}
      >
        {dock}
      </div>
    </div>
  );
}
