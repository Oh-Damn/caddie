import { DEFAULT_PORT, LAN_HOST, type DeviceDetails } from '@companion/protocol';
import { loadPreferences, PREFS_CHANGE } from '../lib/preferences';
import { installSound } from '../lib/sound';

const TAP_TARGET = 'button, [role="button"], a, label, input, select, summary';

const SOUND_TARGET = 'button, [role="button"]';

export type HapticKind = 'light' | 'medium' | 'heavy' | 'success' | 'warning';

const PATTERNS: Record<HapticKind, number | number[]> = {
  light: 10,
  medium: 18,
  heavy: 28,
  success: [12, 60, 12],
  warning: [24, 80, 24],
};

const IOS_PULSES: Record<HapticKind, number> = {
  light: 1,
  medium: 1,
  heavy: 1,
  success: 2,
  warning: 2,
};

let iosSwitch: HTMLInputElement | null = null;
let hapticsInstalled = false;
let hapticsEnabled: boolean | null = null;

function enabled(): boolean {
  if (hapticsEnabled === null) {
    try {
      hapticsEnabled = loadPreferences().haptics;
    } catch {
      hapticsEnabled = false;
    }
  }
  return hapticsEnabled;
}

function iosSwitchPulse(count: number): void {
  if (!iosSwitch) {
    iosSwitch = document.createElement('input');
    iosSwitch.type = 'checkbox';
    iosSwitch.setAttribute('switch', '');
    iosSwitch.setAttribute('aria-hidden', 'true');
    iosSwitch.tabIndex = -1;

    iosSwitch.style.cssText =
      'position:fixed;left:0;top:0;width:1px;height:1px;margin:0;opacity:0.002;pointer-events:none;z-index:-1;border:0;padding:0;appearance:none;';
    document.body.appendChild(iosSwitch);
  }
  const el = iosSwitch;
  el.click();
  for (let i = 1; i < count; i += 1) {
    setTimeout(() => el.click(), i * 90);
  }
}

export function haptic(kind: HapticKind = 'light'): void {
  try {
    if (!enabled()) return;
    if (navigator.vibrate?.(PATTERNS[kind])) return;
    iosSwitchPulse(IOS_PULSES[kind]);
  } catch {}
}

export function installHaptics(): void {
  if (hapticsInstalled) return;
  hapticsInstalled = true;
  installSound(SOUND_TARGET);
  window.addEventListener(PREFS_CHANGE, () => {
    hapticsEnabled = null;
  });
  document.addEventListener(
    'pointerdown',
    (event) => {
      const target = event.target;
      if (!(target instanceof Element)) return;
      if (!target.closest(TAP_TARGET)) return;
      haptic();
    },
    { capture: true, passive: true },
  );
}

export function newId(): string {
  const bytes = new Uint8Array(16);
  crypto.getRandomValues(bytes);
  bytes[6] = (bytes[6] & 0x0f) | 0x40;
  bytes[8] = (bytes[8] & 0x3f) | 0x80;
  const hex = [...bytes].map((b) => b.toString(16).padStart(2, '0')).join('');
  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}

const STORAGE_KEY = 'companion.session';

export type SavedSession = {
  host: string;
  deviceId: string;
  deviceName: string;
};

export function loadSession(): SavedSession | null {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as SavedSession;
  } catch {
    return null;
  }
}

export function saveSession(session: SavedSession): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(session));
}

export function clearSession(): void {
  localStorage.removeItem(STORAGE_KEY);
}

const NAME_KEY = 'companion.deviceName';

function uaName(): string {
  const ua = navigator.userAgent;
  if (/iPhone/i.test(ua)) return 'iPhone';
  if (/iPad/i.test(ua)) return 'iPad';
  if (/Android/i.test(ua)) return 'Android';
  return 'Phone';
}

export function deviceName(): string {
  try {
    const saved = localStorage.getItem(NAME_KEY)?.trim();
    if (saved) return saved;
  } catch {}
  return uaName();
}

export function setDeviceName(name: string): void {
  try {
    const trimmed = name.trim().slice(0, 64);
    if (trimmed) localStorage.setItem(NAME_KEY, trimmed);
    else localStorage.removeItem(NAME_KEY);
  } catch {}
}

export function defaultDeviceName(): string {
  return uaName();
}

type HighEntropy = {
  model?: string;
  platform?: string;
  platformVersion?: string;
};

type UaData = {
  platform?: string;
  getHighEntropyValues?: (hints: string[]) => Promise<HighEntropy>;
};

let detailsPromise: Promise<DeviceDetails> | null = null;

export function deviceDetails(): Promise<DeviceDetails> {
  if (!detailsPromise) {
    detailsPromise = resolveDetails();
  }
  return detailsPromise;
}

async function resolveDetails(): Promise<DeviceDetails> {
  const empty = { model: '', platform: '', platformVersion: '' };
  const data = (navigator as Navigator & { userAgentData?: UaData }).userAgentData;
  if (!data) return empty;
  if (!data.getHighEntropyValues) {
    return { ...empty, platform: data.platform ?? '' };
  }
  try {
    const high = await data.getHighEntropyValues([
      'model',
      'platform',
      'platformVersion',
    ]);
    return {
      model: high.model ?? '',
      platform: high.platform ?? data.platform ?? '',
      platformVersion: high.platformVersion ?? '',
    };
  } catch {
    return { ...empty, platform: data.platform ?? '' };
  }
}

export function wsUrlFromHttp(httpUrl: string): string {
  const u = new URL(httpUrl, window.location.origin);
  u.protocol = u.protocol === 'https:' ? 'wss:' : 'ws:';
  u.pathname = '/ws';
  u.search = '';
  u.hash = '';
  return u.toString();
}

export function defaultHost(): string {
  const { host, hostname, port } = window.location;
  if (port === String(DEFAULT_PORT) || hostname.endsWith('.local')) {
    return host;
  }
  return `${LAN_HOST}:${DEFAULT_PORT}`;
}

export function normalizeHost(input: string): string {
  const trimmed = input.trim();
  if (!trimmed) return defaultHost();
  if (/^https?:\/\//i.test(trimmed)) {
    try {
      return new URL(trimmed).host;
    } catch {
      return trimmed;
    }
  }
  if (trimmed.startsWith('[')) {
    if (/\]:\d+$/.test(trimmed)) return trimmed;
    if (trimmed.endsWith(']')) return `${trimmed}:${DEFAULT_PORT}`;
  }
  if (/:\d+$/.test(trimmed)) return trimmed;
  if (isIpv4(trimmed)) return `${trimmed}:${DEFAULT_PORT}`;
  if (!trimmed.includes('.')) return `${trimmed}.local:${DEFAULT_PORT}`;
  return `${trimmed}:${DEFAULT_PORT}`;
}

export type PairingUrl = {
  origin: string;
  host: string;
  secret: string;
};

export function parsePairingUrl(raw: string): PairingUrl | null {
  try {
    const url = new URL(raw.trim());
    if (url.protocol !== 'https:' && url.protocol !== 'http:') return null;
    const secret = url.searchParams.get('s')?.trim();
    if (!secret) return null;
    return { origin: url.origin, host: url.host, secret };
  } catch {
    return null;
  }
}

function isIpv4(value: string): boolean {
  return /^(?:\d{1,3}\.){3}\d{1,3}$/.test(value);
}
