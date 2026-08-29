import type { Conversation, ConversationKind } from '@companion/protocol';

const KEY = 'caddie.pinnedConversations';
const KINDS = new Set<ConversationKind>(['channel', 'dm', 'thread', 'huddle', 'other']);

function isConversation(value: unknown): value is Conversation {
  if (!value || typeof value !== 'object') return false;
  const item = value as Record<string, unknown>;
  return (
    typeof item.name === 'string' &&
    KINDS.has(item.kind as ConversationKind) &&
    typeof item.workspace === 'string'
  );
}

function loadAll(): Record<string, Conversation[]> {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return {};
    const parsed: unknown = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) return {};
    const out: Record<string, Conversation[]> = {};
    for (const [bundle, list] of Object.entries(parsed as Record<string, unknown>)) {
      if (!Array.isArray(list)) continue;
      out[bundle] = list.filter(isConversation).map((item) => ({
        name: item.name,
        kind: item.kind,
        workspace: item.workspace,
        windowIndex: null,
      }));
    }
    return out;
  } catch {
    return {};
  }
}

export function loadPinnedConversations(bundleId: string): Conversation[] {
  if (!bundleId) return [];
  return loadAll()[bundleId] ?? [];
}

export function savePinnedConversations(bundleId: string, pins: Conversation[]): void {
  if (!bundleId) return;
  try {
    const all = loadAll();
    all[bundleId] = pins.map((item) => ({
      name: item.name,
      kind: item.kind,
      workspace: item.workspace,
      windowIndex: null,
    }));
    localStorage.setItem(KEY, JSON.stringify(all));
  } catch {}
}
