import { useCallback, useEffect, useRef, useState } from 'react';
import {
  loadPreferences,
  PREFS_CHANGE,
  savePreferences,
  type Preferences,
} from './preferences';

export function usePreferences() {
  const [prefs, setPrefs] = useState(loadPreferences);

  useEffect(() => {
    const sync = () => setPrefs(loadPreferences());
    window.addEventListener(PREFS_CHANGE, sync);
    return () => window.removeEventListener(PREFS_CHANGE, sync);
  }, []);

  const update = useCallback((patch: Partial<Preferences>) => {
    const next = { ...loadPreferences(), ...patch };
    savePreferences(next);
    setPrefs(next);
  }, []);

  return { prefs, update };
}

export function usePreferencesRef() {
  const ref = useRef(loadPreferences());

  useEffect(() => {
    const sync = () => {
      ref.current = loadPreferences();
    };
    window.addEventListener(PREFS_CHANGE, sync);
    return () => window.removeEventListener(PREFS_CHANGE, sync);
  }, []);

  return ref;
}
