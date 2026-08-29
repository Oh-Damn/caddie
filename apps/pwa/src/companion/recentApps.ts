import type { RunningApp } from '@companion/protocol';

export function pushRecentBundle(recent: string[], bundleId: string): string[] {
  return [bundleId, ...recent.filter((id) => id !== bundleId)].slice(0, 8);
}

export function recentRunningApps(
  apps: RunningApp[] | null,
  currentBundle: string,
  recentBundles: string[],
): RunningApp[] {
  if (!apps?.length) return [];
  const byId = new Map(apps.map((a) => [a.bundleId, a]));
  const out: RunningApp[] = [];
  for (const id of recentBundles) {
    if (id === currentBundle) continue;
    const app = byId.get(id);
    if (app) out.push(app);
    if (out.length >= 2) break;
  }
  return out;
}
