import type { BrowserTab } from '@companion/protocol';
import { Volume2 } from 'lucide-react';
import { useRef } from 'react';
import { tabIdentity } from '../../companion/tabs';

type Props = {
  tabs: BrowserTab[];
  onActivate: (tab: BrowserTab) => void;
};

type TabGroup = {
  profile: string;
  tabs: BrowserTab[];
};

export function TabList({ tabs, onActivate }: Props) {
  const profileOrder = useRef<string[]>([]);

  if (tabs.length === 0) {
    return <p className="pt-2 text-sm text-app-muted">No open tabs.</p>;
  }

  const groups = groupTabs(tabs, profileOrder.current);
  const showHeaders = groups.length > 1 || Boolean(groups[0]?.profile);

  return (
    <div className="faceplate overflow-hidden">
      {groups.map((group, gi) => (
        <section
          key={group.profile || 'chrome'}
          className={gi > 0 ? 'border-t border-app-border' : undefined}
        >
          {showHeaders ? (
            <p className="type-micro px-3 pb-0.5 pt-2.5">{group.profile || 'Chrome'}</p>
          ) : null}
          <ul>
            {group.tabs.map((tab) => (
              <li key={tabIdentity(tab)}>
                <button
                  type="button"
                  aria-pressed={tab.active}
                  aria-label={tab.audible ? `${tab.title}, playing` : tab.title}
                  className={`flex min-h-touch w-full items-center gap-2.5 px-3 py-2 text-left ${
                    tab.active ? 'bg-app-well text-app-text' : 'text-app-text'
                  }`}
                  onClick={() => onActivate(tab)}
                >
                  <span
                    className={`h-4 w-0.5 shrink-0 ${
                      tab.active ? 'bg-accent' : 'bg-transparent'
                    }`}
                    aria-hidden
                  />
                  <span className="min-w-0 flex-1 truncate text-xs font-medium">
                    {tab.title}
                  </span>
                  {tab.audible ? (
                    <Volume2 className="h-4 w-4 shrink-0 text-accent" aria-hidden />
                  ) : null}
                </button>
              </li>
            ))}
          </ul>
        </section>
      ))}
    </div>
  );
}

function groupTabs(tabs: BrowserTab[], order: string[]): TabGroup[] {
  const map = new Map<string, BrowserTab[]>();
  for (const tab of tabs) {
    const key = tab.profile?.trim() ?? '';
    if (!map.has(key)) {
      map.set(key, []);
    }
    map.get(key)?.push(tab);
    if (!order.includes(key)) {
      order.push(key);
    }
  }
  for (let i = order.length - 1; i >= 0; i--) {
    if (!map.has(order[i] ?? '')) {
      order.splice(i, 1);
    }
  }
  return order.map((profile) => ({
    profile,
    tabs: map.get(profile) ?? [],
  }));
}
