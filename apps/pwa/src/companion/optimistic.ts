import type {
  Conversation,
  NowPlaying,
  RepeatMode,
  RunningApp,
  StatePayload,
} from '@companion/protocol';
import { isNativePlayer, normalizeNowPlaying } from './nowPlaying';
import { soloActiveTabs, tabIdentity, unpackTabRef } from './tabs';

const HOLD_MS = 2500;

type Entry = { value: unknown; until: number };

export class OptimisticHold {
  private pending = new Map<string, Entry>();

  patchForCommand(
    state: StatePayload,
    action: string,
    value: number | null,
    target: string | null,
    apps: RunningApp[] | null,
  ): StatePayload {
    const next = patchStateForCommand(state, action, value, target, apps);
    if (next.muted !== state.muted) this.hold('muted', next.muted);
    if (next.volume !== state.volume) this.hold('volume', next.volume);
    if (next.nowPlaying?.playing !== state.nowPlaying?.playing) {
      this.hold('playing', next.nowPlaying?.playing ?? false);
    }
    if (next.browserNowPlaying?.playing !== state.browserNowPlaying?.playing) {
      this.hold('browserPlaying', next.browserNowPlaying?.playing ?? false);
    }
    if (next.nowPlaying?.shuffle !== state.nowPlaying?.shuffle) {
      this.hold('shuffle', next.nowPlaying?.shuffle ?? false);
    }
    if (next.nowPlaying?.liked !== state.nowPlaying?.liked) {
      this.hold('liked', next.nowPlaying?.liked ?? false);
    }
    if (next.nowPlaying?.repeatMode !== state.nowPlaying?.repeatMode) {
      this.hold('repeatMode', next.nowPlaying?.repeatMode ?? 'off');
    }
    if (next.call?.muted !== state.call?.muted) {
      this.hold('callMuted', next.call?.muted ?? false);
    }
    if (action === 'browser.activate_tab' && value != null) {
      const { windowIndex, tabIndex } = unpackTabRef(value);
      const tapped = state.tabs.find(
        (t) => t.windowIndex === windowIndex && t.tabIndex === tabIndex,
      );
      this.hold('activeTab', tapped ? tabIdentity(tapped) : String(value), 4000);
    }
    if (next.pinnedConversations !== state.pinnedConversations) {
      this.hold('pinned', next.pinnedConversations);
    }
    return next;
  }

  merge(server: StatePayload): StatePayload {
    const now = Date.now();
    let result: StatePayload = {
      ...server,
      nowPlaying: normalizeNowPlaying(server.nowPlaying),
      browserNowPlaying: normalizeNowPlaying(server.browserNowPlaying),
      tabs: soloActiveTabs(server.tabs ?? [], server.windowTitle),
      call: server.call ?? null,
      unread: server.unread ?? null,
      currentConversation: server.currentConversation ?? null,
      openConversations: server.openConversations ?? [],
      recentConversations: server.recentConversations ?? [],
      pinnedConversations: server.pinnedConversations ?? [],
    };

    for (const [key, entry] of [...this.pending.entries()]) {
      if (now > entry.until) {
        this.pending.delete(key);
        continue;
      }
      const current = this.read(result, key);
      if (current === entry.value) {
        this.pending.delete(key);
        continue;
      }
      result = this.write(result, key, entry.value);
    }

    return result;
  }

  clear() {
    this.pending.clear();
  }

  private hold(key: string, value: unknown, ms = HOLD_MS) {
    this.pending.set(key, { value, until: Date.now() + ms });
  }

  private read(state: StatePayload, key: string): unknown {
    switch (key) {
      case 'muted':
        return state.muted;
      case 'volume':
        return state.volume;
      case 'playing':
        return state.nowPlaying?.playing;
      case 'browserPlaying':
        return state.browserNowPlaying?.playing;
      case 'shuffle':
        return state.nowPlaying?.shuffle;
      case 'liked':
        return state.nowPlaying?.liked;
      case 'repeatMode':
        return state.nowPlaying?.repeatMode;
      case 'callMuted':
        return state.call?.muted;
      case 'activeTab': {
        const actives = state.tabs.filter((t) => t.active);
        const active = actives.length === 1 ? actives[0] : undefined;
        return active ? tabIdentity(active) : undefined;
      }
      case 'pinned':
        return state.pinnedConversations;
      default:
        return undefined;
    }
  }

  private write(state: StatePayload, key: string, value: unknown): StatePayload {
    switch (key) {
      case 'muted':
        return { ...state, muted: value as boolean };
      case 'volume':
        return { ...state, volume: value as number };
      case 'playing':
        if (!state.nowPlaying) return state;
        return {
          ...state,
          nowPlaying: { ...state.nowPlaying, playing: value as boolean },
        };
      case 'browserPlaying':
        if (!state.browserNowPlaying) return state;
        return {
          ...state,
          browserNowPlaying: {
            ...state.browserNowPlaying,
            playing: value as boolean,
          },
        };
      case 'shuffle':
      case 'liked':
        if (!state.nowPlaying) return state;
        return {
          ...state,
          nowPlaying: { ...state.nowPlaying, [key]: value as boolean },
        };
      case 'repeatMode':
        if (!state.nowPlaying) return state;
        return {
          ...state,
          nowPlaying: { ...state.nowPlaying, repeatMode: value as RepeatMode },
        };
      case 'callMuted':
        if (!state.call) return state;
        return { ...state, call: { ...state.call, muted: value as boolean } };
      case 'activeTab': {
        const id = value as string;
        return {
          ...state,
          tabs: state.tabs.map((tab) => ({
            ...tab,
            active: tabIdentity(tab) === id,
          })),
        };
      }
      case 'pinned':
        return { ...state, pinnedConversations: value as Conversation[] };
      default:
        return state;
    }
  }
}

function patchStateForCommand(
  state: StatePayload,
  action: string,
  value: number | null,
  target: string | null,
  apps: RunningApp[] | null,
): StatePayload {
  switch (action) {
    case 'volume.up':
      return { ...state, muted: false, volume: Math.min(100, state.volume + 10) };
    case 'volume.down':
      return { ...state, volume: Math.max(0, state.volume - 10) };
    case 'volume.toggle_mute':
      return { ...state, muted: !state.muted };
    case 'volume.set':
      if (value == null) return state;
      return {
        ...state,
        muted: false,
        volume: Math.round(Math.max(0, Math.min(100, value))),
      };
    case 'media.play_pause':
      return patchNowPlaying(state, target, (np) => ({ ...np, playing: !np.playing }));
    case 'media.seek_back':
      return patchNowPlaying(state, target, (np) => ({
        ...np,
        positionSec: Math.max(0, np.positionSec - 15),
      }));
    case 'media.seek_forward':
      return patchNowPlaying(state, target, (np) => ({
        ...np,
        positionSec: Math.min(np.durationSec, np.positionSec + 15),
      }));
    case 'media.shuffle':
      return patchNowPlaying(state, target, (np) => ({ ...np, shuffle: !np.shuffle }));
    case 'media.like':
      return patchNowPlaying(state, target, (np) => ({ ...np, liked: !np.liked }));
    case 'media.repeat':
      return patchNowPlaying(state, target, (np) => ({
        ...np,
        repeatMode: nextRepeatMode(np.repeatMode, np.sourceBundleId),
      }));
    case 'app.focus': {
      const bundle = target?.trim() || state.bundleId;
      const app = apps?.find((a) => a.bundleId === bundle);
      if (app) {
        return { ...state, bundleId: app.bundleId, appName: app.name };
      }
      if (target?.trim()) {
        return { ...state, bundleId: target.trim() };
      }
      return state;
    }
    case 'browser.activate_tab': {
      if (value == null) return state;
      const { windowIndex, tabIndex } = unpackTabRef(value);
      const tapped = (state.tabs ?? []).find(
        (t) => t.windowIndex === windowIndex && t.tabIndex === tabIndex,
      );
      const id = tapped ? tabIdentity(tapped) : null;
      return {
        ...state,
        tabs: (state.tabs ?? []).map((tab) => ({
          ...tab,
          active: id
            ? tabIdentity(tab) === id
            : tab.windowIndex === windowIndex && tab.tabIndex === tabIndex,
        })),
      };
    }
    case 'call.toggle_mic':
      if (!state.call) return state;
      return { ...state, call: { ...state.call, muted: !state.call.muted } };
    case 'approval.allow':
    case 'approval.deny':
      return { ...state, approval: null };
    case 'conversation.pin':
      return patchPin(state, target, true);
    case 'conversation.unpin':
      return patchPin(state, target, false);
    default:
      return state;
  }
}

function patchPin(
  state: StatePayload,
  target: string | null,
  pin: boolean,
): StatePayload {
  const name = target?.trim();
  if (!name) return state;
  const pinned = state.pinnedConversations ?? [];
  if (!pin) {
    return {
      ...state,
      pinnedConversations: pinned.filter(
        (p) => p.name.toLowerCase() !== name.toLowerCase(),
      ),
    };
  }
  if (pinned.some((p) => p.name.toLowerCase() === name.toLowerCase())) {
    return state;
  }
  const found =
    state.currentConversation?.name.toLowerCase() === name.toLowerCase()
      ? state.currentConversation
      : [...(state.openConversations ?? []), ...(state.recentConversations ?? [])].find(
          (c) => c.name.toLowerCase() === name.toLowerCase(),
        );
  const conv: Conversation = found
    ? { ...found, windowIndex: null }
    : {
        name,
        kind: 'dm',
        workspace: state.currentConversation?.workspace ?? '',
        windowIndex: null,
      };
  return { ...state, pinnedConversations: [conv, ...pinned] };
}

function patchNowPlaying(
  state: StatePayload,
  target: string | null,
  fn: (np: NowPlaying) => NowPlaying,
): StatePayload {
  const id = target?.trim() || '';
  if (id && !isNativePlayer(id) && state.browserNowPlaying) {
    return { ...state, browserNowPlaying: fn(state.browserNowPlaying) };
  }
  if (state.nowPlaying && (!id || state.nowPlaying.sourceBundleId === id)) {
    return { ...state, nowPlaying: fn(state.nowPlaying) };
  }
  if (state.browserNowPlaying && (!id || state.browserNowPlaying.sourceBundleId === id)) {
    return { ...state, browserNowPlaying: fn(state.browserNowPlaying) };
  }
  if (state.nowPlaying) return { ...state, nowPlaying: fn(state.nowPlaying) };
  return state;
}

function nextRepeatMode(current: RepeatMode, bundleId: string): RepeatMode {
  if (bundleId === 'com.apple.Music') {
    if (current === 'off') return 'all';
    if (current === 'all') return 'one';
    return 'off';
  }
  return current === 'off' ? 'all' : 'off';
}
