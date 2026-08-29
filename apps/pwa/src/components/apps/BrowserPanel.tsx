import type { BrowserTab, NowPlaying } from '@companion/protocol';
import { Search, X } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { soloActiveTabs, tabIdentity } from '../../companion/tabs';
import {
  loadRecentTabs,
  pushRecentTab,
  saveRecentTabs,
} from '../../companion/recentTabs';
import { parseTabTitle } from '../../lib/tabTitle';
import { TabRow } from './TabRow';

type Props = {
  tabs: BrowserTab[];
  windowTitle?: string;
  nowPlaying: NowPlaying | null;
  onActivate: (tab: BrowserTab) => void;
  onToggleMedia: (tab: BrowserTab) => void;
};

const ALL = ' all';
const NO_BAR = '[scrollbar-width:none] [&::-webkit-scrollbar]:hidden';

export function BrowserPanel({
  tabs: rawTabs,
  windowTitle,
  nowPlaying,
  onActivate,
  onToggleMedia,
}: Props) {
  const [query, setQuery] = useState('');
  const [filtering, setFiltering] = useState(false);
  const [profile, setProfile] = useState<string>(ALL);
  const [recent, setRecent] = useState<string[]>(loadRecentTabs);
  const seenRef = useRef('');
  const filterRef = useRef<HTMLInputElement>(null);
  const popRef = useRef<HTMLDivElement>(null);
  const tabs = soloActiveTabs(rawTabs, windowTitle);

  const active = tabs.find((t) => t.active);
  const activeId = active ? tabIdentity(active) : '';
  useEffect(() => {
    if (!activeId || seenRef.current === activeId) return;
    seenRef.current = activeId;
    setRecent((list) => {
      const next = pushRecentTab(list, activeId);
      saveRecentTabs(next);
      return next;
    });
  }, [activeId]);

  const profiles = useMemo(() => {
    const counts = new Map<string, number>();
    for (const tab of tabs) {
      const key = tab.profile?.trim() || 'Chrome';
      counts.set(key, (counts.get(key) ?? 0) + 1);
    }
    return [...counts.entries()].map(([name, count]) => ({ name, count }));
  }, [tabs]);

  useEffect(() => {
    if (profile !== ALL && !profiles.some((p) => p.name === profile)) setProfile(ALL);
  }, [profile, profiles]);

  useEffect(() => {
    if (filtering) filterRef.current?.focus();
  }, [filtering]);

  useEffect(() => {
    if (!filtering) return;
    const onPointer = (e: PointerEvent) => {
      if (popRef.current && !popRef.current.contains(e.target as Node)) {
        setQuery('');
        setFiltering(false);
      }
    };
    document.addEventListener('pointerdown', onPointer);
    return () => document.removeEventListener('pointerdown', onPointer);
  }, [filtering]);

  const needle = query.trim().toLowerCase();
  const filtered = useMemo(
    () =>
      tabs.filter((tab) => {
        const key = tab.profile?.trim() || 'Chrome';
        if (profile !== ALL && key !== profile) return false;
        return !needle || tab.title.toLowerCase().includes(needle);
      }),
    [needle, profile, tabs],
  );

  const recentTabs = useMemo(() => {
    if (needle) return [];
    const byId = new Map(tabs.map((t) => [tabIdentity(t), t]));
    const out: BrowserTab[] = [];
    for (const id of recent) {
      const tab = byId.get(id);
      if (tab && !tab.active) out.push(tab);
      if (out.length === 6) break;
    }
    return out;
  }, [needle, recent, tabs]);

  const closeFilter = () => {
    setQuery('');
    setFiltering(false);
  };

  if (tabs.length === 0) {
    return <p className="type-secondary pt-2 text-app-muted">No open tabs.</p>;
  }

  return (
    <div className="flex min-w-0 flex-col gap-2">
      <div className="flex items-center gap-1.5">
        {profiles.length > 1 ? (
          <div className={`flex min-w-0 flex-1 gap-1.5 overflow-x-auto ${NO_BAR}`}>
            <ProfilePill
              label="All"
              count={tabs.length}
              selected={profile === ALL}
              onSelect={() => setProfile(ALL)}
            />
            {profiles.map((p) => (
              <ProfilePill
                key={p.name}
                label={p.name}
                count={p.count}
                selected={profile === p.name}
                onSelect={() => setProfile(p.name)}
              />
            ))}
          </div>
        ) : (
          <div className="min-w-0 flex-1" />
        )}
        <div ref={popRef} className="relative shrink-0">
          <button
            type="button"
            aria-label="Filter tabs"
            aria-expanded={filtering}
            className="key flex h-touch w-touch items-center justify-center text-app-muted"
            onClick={() => setFiltering((open) => !open)}
          >
            <Search className="h-4 w-4" strokeWidth={2.2} />
          </button>
          {filtering ? (
            <div className="absolute top-full right-0 z-10 mt-1.5 flex w-[min(18rem,calc(100vw-2.5rem))] items-center gap-2 faceplate px-2.5">
              <Search className="h-3.5 w-3.5 shrink-0 text-app-muted" aria-hidden />
              <input
                ref={filterRef}
                value={query}
                onChange={(e) => setQuery(e.target.value)}
                placeholder="Filter tabs"
                aria-label="Filter tabs"
                className="min-w-0 flex-1 bg-transparent py-2.5 text-base outline-none placeholder:text-app-muted"
              />
              <button
                type="button"
                aria-label="Close filter"
                onClick={closeFilter}
                className="flex h-touch w-touch shrink-0 items-center justify-center text-app-muted"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            </div>
          ) : null}
        </div>
      </div>

      {recentTabs.length > 0 ? (
        <div className="min-w-0">
          <p className="type-micro px-0.5 pb-1">Recent</p>
          <div className={`flex gap-1.5 overflow-x-auto pb-0.5 ${NO_BAR}`}>
            {recentTabs.map((tab) => {
              const parsed = parseTabTitle(tab.title);
              return (
                <button
                  key={tabIdentity(tab)}
                  type="button"
                  onClick={() => onActivate(tab)}
                  className="key flex w-36 shrink-0 flex-col justify-center gap-0.5 px-2.5 py-2 text-left"
                >
                  <span className="truncate text-xs font-semibold tracking-wide">
                    {parsed.page}
                  </span>
                  <span className="type-meta truncate">{parsed.site || 'Tab'}</span>
                </button>
              );
            })}
          </div>
        </div>
      ) : null}

      <div className="faceplate overflow-hidden">
        {filtered.length === 0 ? (
          <p className="type-meta px-3 py-3">No tabs match.</p>
        ) : (
          <ul className="landscape:grid landscape:grid-cols-2">
            {filtered.map((tab) => (
              <li
                key={tabIdentity(tab)}
                className="border-b border-app-border last:border-b-0 landscape:odd:border-r"
              >
                <TabRow
                  tab={tab}
                  playing={tab.audible || (tab.active && Boolean(nowPlaying?.playing))}
                  onActivate={onActivate}
                  onToggleMedia={onToggleMedia}
                />
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

function ProfilePill({
  label,
  count,
  selected,
  onSelect,
}: {
  label: string;
  count: number;
  selected: boolean;
  onSelect: () => void;
}) {
  return (
    <button
      type="button"
      aria-pressed={selected}
      onClick={onSelect}
      className={`key flex shrink-0 items-center gap-1.5 px-3 py-1.5 text-[11px] font-semibold tracking-wide ${
        selected ? 'key-lit' : 'text-app-muted'
      }`}
    >
      <span className="max-w-[7rem] truncate">{label}</span>
      <span className="opacity-70">{count}</span>
    </button>
  );
}
