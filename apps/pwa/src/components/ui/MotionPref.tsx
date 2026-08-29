import { useEffect } from 'react';
import { usePreferences } from '../../lib/usePreferences';

export function MotionPref() {
  const { prefs } = usePreferences();

  useEffect(() => {
    document.documentElement.classList.toggle('motion-off', !prefs.motion);
    return () => document.documentElement.classList.remove('motion-off');
  }, [prefs.motion]);

  return null;
}
