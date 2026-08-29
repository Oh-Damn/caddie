export type Preferences = {
  haptics: boolean;
  sound: boolean;
  aliveBackground: boolean;
  motion: boolean;
  optimisticUpdates: boolean;
};

export const PREFS_CHANGE = 'caddie:prefs';

const STORAGE_KEY = 'caddie.prefs';

const DEFAULTS: Preferences = {
  haptics: true,
  sound: true,
  aliveBackground: true,
  motion: true,
  optimisticUpdates: true,
};

export function loadPreferences(): Preferences {
  const raw = localStorage.getItem(STORAGE_KEY);
  if (!raw) return DEFAULTS;
  try {
    return { ...DEFAULTS, ...JSON.parse(raw) };
  } catch {
    return DEFAULTS;
  }
}

export function savePreferences(next: Preferences): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(next));
  window.dispatchEvent(new Event(PREFS_CHANGE));
}

export const PREF_ITEMS: {
  key: keyof Preferences;
  label: string;
  hint: string;
}[] = [
  {
    key: 'haptics',
    label: 'Haptics',
    hint: 'Vibrate on taps',
  },
  {
    key: 'sound',
    label: 'Sound',
    hint: 'Click on taps, chime when an agent needs you',
  },
  {
    key: 'optimisticUpdates',
    label: 'Instant feedback',
    hint: 'Update before the Mac confirms',
  },
  {
    key: 'aliveBackground',
    label: 'Alive background',
    hint: 'Blurred album art and grain on home',
  },
  {
    key: 'motion',
    label: 'Animations',
    hint: 'Reveals and shimmer',
  },
];
