import type { RunningApp } from '@companion/protocol';
import { AppSwitcherSkeleton } from '../home/HomeSkeleton';
import { AppIcon } from './AppIcon';

type Props = {
  apps: RunningApp[] | null;
  currentBundle: string;
  onFocus: (bundleId: string, windowIndex?: number | null) => void;
};

type GridItem = {
  key: string;
  app: RunningApp;
  windowIndex: number | null;
  label: string;
  active: boolean;
};

export function AppSwitcher({ apps, currentBundle, onFocus }: Props) {
  return (
    <div className="min-h-0 flex-1 overflow-y-auto">
      {apps === null ? (
        <AppSwitcherSkeleton />
      ) : apps.length === 0 ? (
        <p className="type-secondary pt-6 text-app-muted">No open apps.</p>
      ) : (
        <ul className="grid grid-cols-4 gap-x-2 gap-y-4 pb-4 landscape:grid-cols-8">
          {gridItems(apps, currentBundle).map((item) => (
            <li key={item.key}>
              <button
                type="button"
                className="flex w-full flex-col items-center gap-1.5 text-center"
                onClick={() => onFocus(item.app.bundleId, item.windowIndex)}
              >
                <span
                  className={`key h-14 w-14 shrink-0 overflow-hidden ${
                    item.active ? 'key-lit' : ''
                  }`}
                >
                  <AppIcon
                    bundleId={item.app.bundleId}
                    name={item.app.name}
                    className="h-full w-full object-cover"
                  />
                </span>
                <span
                  className={`type-micro max-w-full truncate px-0.5 ${
                    item.active ? 'text-accent' : 'text-app-muted'
                  }`}
                >
                  {item.label}
                </span>
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function gridItems(apps: RunningApp[], currentBundle: string): GridItem[] {
  const out: GridItem[] = [];
  for (const app of apps) {
    const active = app.bundleId === currentBundle;
    if (app.windows.length <= 1) {
      out.push({
        key: app.bundleId,
        app,
        windowIndex: null,
        label: app.name,
        active,
      });
      continue;
    }
    for (const w of app.windows) {
      out.push({
        key: `${app.bundleId}-${w.index}`,
        app,
        windowIndex: w.index,
        label: w.title,
        active,
      });
    }
  }
  return out;
}
