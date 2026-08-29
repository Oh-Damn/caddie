import { HomeHeader } from './HomeHeader';
import { HomeLayout } from '../../layouts/HomeLayout';
import { Skeleton } from '../ui/Skeleton';

export function HomeSkeleton() {
  return (
    <HomeLayout
      header={<HomeHeader />}
      dock={<Skeleton className="faceplate h-[4.5rem]" />}
    >
      <Skeleton className="faceplate min-h-24 landscape:min-h-40" />
    </HomeLayout>
  );
}

export function ShortcutGridSkeleton() {
  return (
    <div className="grid grid-cols-3 gap-1 landscape:grid-cols-6" aria-hidden>
      <Skeleton className="key col-span-2 min-h-touch-lg" />
      <Skeleton className="key min-h-touch-lg" />
      <Skeleton className="key min-h-touch-lg" />
      <Skeleton className="key min-h-touch-lg" />
      <Skeleton className="key min-h-touch-lg" />
      <Skeleton className="key col-span-2 min-h-touch-lg landscape:col-span-1" />
    </div>
  );
}

export function AppSwitcherSkeleton() {
  return (
    <ul
      className="grid grid-cols-4 gap-x-2 gap-y-4 pb-4 landscape:grid-cols-8"
      aria-hidden
    >
      {Array.from({ length: 16 }).map((_, i) => (
        <li key={i} className="flex flex-col items-center gap-1.5">
          <Skeleton className="key h-14 w-14" />
          <Skeleton className="h-2.5 w-10" />
        </li>
      ))}
    </ul>
  );
}

export function TabListSkeleton() {
  return (
    <div className="faceplate overflow-hidden" aria-hidden>
      {Array.from({ length: 4 }).map((_, i) => (
        <div key={i} className="flex min-h-touch items-center px-3">
          <Skeleton className="h-3 w-3/4" />
        </div>
      ))}
    </div>
  );
}
