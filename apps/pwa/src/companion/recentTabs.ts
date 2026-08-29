const KEY = 'companion.recentTabs';
const CAP = 8;

export function loadRecentTabs(): string[] {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((v) => typeof v === 'string') : [];
  } catch {
    return [];
  }
}

export function saveRecentTabs(list: string[]): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(list));
  } catch {}
}

export function pushRecentTab(list: string[], id: string): string[] {
  if (!id) return list;
  if (list[0] === id) return list;
  return [id, ...list.filter((item) => item !== id)].slice(0, CAP);
}
