export function packTabRef(windowIndex: number, tabIndex: number): number {
  return ((windowIndex & 0xffff) << 16) | (tabIndex & 0xffff);
}

export function unpackTabRef(value: number): { windowIndex: number; tabIndex: number } {
  return { windowIndex: value >>> 16, tabIndex: value & 0xffff };
}

type TabKey = {
  title: string;
  tabIndex: number;
  windowIndex?: number;
  profile?: string;
  active?: boolean;
};

export function tabIdentity(tab: TabKey): string {
  return `${tab.profile?.trim() ?? ''}\t${tab.title}\t${tab.tabIndex}`;
}

export function packTabTarget(bundleId: string, tab: TabKey): string {
  return `${bundleId}\t${tab.profile?.trim() ?? ''}\t${tab.title}`;
}

export function soloActiveTabs<T extends TabKey>(tabs: T[], windowTitle?: string): T[] {
  const keep = matchTabForWindow(tabs, windowTitle);
  if (keep != null) {
    const id = tabIdentity(tabs[keep]);
    return tabs.map((tab) => ({ ...tab, active: tabIdentity(tab) === id }));
  }
  const hits = tabs.filter((t) => t.active);
  if (hits.length <= 1) return tabs;
  const picked = hits.reduce((a, b) =>
    (a.windowIndex ?? 0) <= (b.windowIndex ?? 0) ? a : b,
  );
  const id = tabIdentity(picked);
  return tabs.map((tab) => ({ ...tab, active: tabIdentity(tab) === id }));
}

function matchTabForWindow<T extends TabKey>(
  tabs: T[],
  windowTitle?: string,
): number | null {
  const ax = stripWindowMediaPrefix(windowTitle ?? '');
  if (!ax) return null;
  const axLower = ax.toLowerCase();
  let best: { i: number; len: number; profile: boolean; active: boolean } | null = null;
  for (let i = 0; i < tabs.length; i++) {
    const tab = tabs[i];
    const title = tab.title.trim();
    if (!title || !windowTitleHasTab(ax, title)) continue;
    const profile = tab.profile?.trim() ?? '';
    const profileHit = profile.length > 0 && axLower.includes(profile.toLowerCase());
    const cand = {
      i,
      len: title.length,
      profile: profileHit,
      active: Boolean(tab.active),
    };
    if (
      !best ||
      cand.len > best.len ||
      (cand.len === best.len && cand.profile && !best.profile) ||
      (cand.len === best.len &&
        cand.profile === best.profile &&
        cand.active &&
        !best.active)
    ) {
      best = cand;
    }
  }
  return best?.i ?? null;
}

function windowTitleHasTab(windowTitle: string, tabTitle: string): boolean {
  if (windowTitle === tabTitle) return true;
  return (
    windowTitle.startsWith(`${tabTitle} -`) ||
    windowTitle.startsWith(`${tabTitle} \u2013`) ||
    windowTitle.startsWith(`${tabTitle} \u2014`)
  );
}

function stripWindowMediaPrefix(title: string): string {
  return title.replace(/^(?:[\u{1F507}-\u{1F50A}]\uFE0F?|\s)+/u, '');
}
