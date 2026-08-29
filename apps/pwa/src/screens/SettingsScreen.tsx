import { useState } from 'react';
import { Download, RefreshCw, Unplug } from 'lucide-react';
import { Button } from '../components/ui/Button';
import { Field } from '../components/ui/Field';
import { Switch } from '../components/ui/Switch';
import {
  defaultDeviceName,
  deviceName,
  haptic,
  setDeviceName,
} from '../companion/session';
import { useCompanion } from '../companion/useCompanion';
import { isStandalone, openInstallPopup } from '../lib/install';
import { clearCaches } from '../lib/swUpdate';
import { PREF_ITEMS } from '../lib/preferences';
import { usePreferences } from '../lib/usePreferences';
import { StackLayout } from '../layouts/StackLayout';
import { navigate, ROUTES } from '../routes';

export function SettingsScreen() {
  const { unpair } = useCompanion();
  const { prefs, update } = usePreferences();
  const [name, setName] = useState(deviceName);

  return (
    <StackLayout title="Settings" onBack={() => navigate(ROUTES.home)}>
      <div className="flex flex-col gap-5 pb-4">
        <div className="flex flex-col gap-2">
          <p className="type-micro px-0.5">This device</p>
          <Field
            label="Name"
            value={name}
            onChange={(value) => {
              setName(value);
              setDeviceName(value);
            }}
            placeholder={defaultDeviceName()}
          />
          <p className="type-meta px-1">
            Shown on the Mac. Takes effect the next time this phone connects.
          </p>
        </div>
        <div className="flex flex-col gap-2">
          <p className="type-micro px-0.5">Controls</p>
          <div className="faceplate overflow-hidden">
            {PREF_ITEMS.map((item) => (
              <label
                key={item.key}
                className="flex items-center justify-between gap-3 border-b border-app-border px-3 py-3 last:border-b-0"
              >
                <span>
                  <span className="type-secondary block">{item.label}</span>
                  <span className="type-meta mt-0.5 block">{item.hint}</span>
                </span>
                <Switch
                  checked={prefs[item.key]}
                  label={item.label}
                  onChange={(checked) => {
                    update({ [item.key]: checked });
                    if (item.key === 'haptics' && checked) haptic();
                  }}
                />
              </label>
            ))}
          </div>
        </div>

        <div className="flex flex-col gap-2">
          <p className="type-micro px-0.5">Device</p>
          {isStandalone() ? null : (
            <Button variant="ghost" onClick={openInstallPopup}>
              <Download className="h-4 w-4" />
              Install app
            </Button>
          )}
          <Button variant="ghost" onClick={() => void clearCaches()}>
            <RefreshCw className="h-4 w-4" />
            Clear cache and reload
          </Button>
          <Button variant="danger" onClick={unpair}>
            <Unplug className="h-4 w-4" />
            Unpair
          </Button>
        </div>
      </div>
    </StackLayout>
  );
}
