import type { NowPlaying, RepeatMode, StatePayload } from '@companion/protocol';
import type { MediaSource } from '../components/media/MediaSources';

type RawNowPlaying = NowPlaying & { repeating?: boolean };

const NATIVE_PLAYERS = new Set(['com.spotify.client', 'com.apple.Music']);

export function isNativePlayer(bundleId: string): boolean {
  return NATIVE_PLAYERS.has(bundleId);
}

export function normalizeNowPlaying(
  raw: RawNowPlaying | null | undefined,
): NowPlaying | null {
  if (!raw) return null;
  let repeatMode: RepeatMode = raw.repeatMode ?? 'off';
  if (!raw.repeatMode && raw.repeating) {
    repeatMode = 'all';
  }
  return {
    ...raw,
    shuffle: raw.shuffle ?? false,
    liked: raw.liked ?? false,
    repeatMode,
  };
}

export function nativeNowPlaying(state: StatePayload | null): NowPlaying | null {
  const np = normalizeNowPlaying(state?.nowPlaying ?? null);
  if (np && isNativePlayer(np.sourceBundleId)) return np;
  return null;
}

export function browserNowPlaying(state: StatePayload | null): NowPlaying | null {
  const fromField = normalizeNowPlaying(state?.browserNowPlaying ?? null);
  if (fromField) return fromField;
  const np = normalizeNowPlaying(state?.nowPlaying ?? null);
  if (np && !isNativePlayer(np.sourceBundleId)) return np;
  return null;
}

export function mediaNowPlaying(
  state: StatePayload | null,
  layoutScreen: string | undefined,
): NowPlaying | null {
  const native = nativeNowPlaying(state);
  const browser = browserNowPlaying(state);
  if (native?.playing) return native;
  if (browser?.playing) return browser;
  if (native) return native;
  if (browser) return browser;
  if (layoutScreen !== 'media' || !state) return null;
  const title = state.windowTitle.trim();
  if (!title) return null;
  return {
    title,
    artist: '',
    artworkUrl: '',
    playing: true,
    sourceName: state.appName,
    sourceBundleId: state.bundleId,
    positionSec: 0,
    durationSec: 0,
    shuffle: false,
    liked: false,
    repeatMode: 'off',
  };
}

export function mediaSources(
  native: NowPlaying | null,
  browser: NowPlaying | null = null,
): MediaSource[] {
  const out: MediaSource[] = [];
  if (native?.title) out.push({ ...native, controllable: true });
  if (browser?.title && browser.sourceBundleId !== native?.sourceBundleId) {
    out.push({ ...browser, controllable: true });
  }
  return out;
}
