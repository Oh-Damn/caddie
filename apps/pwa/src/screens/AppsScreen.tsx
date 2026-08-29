import { AppSwitcher } from '../components/apps/AppSwitcher';
import { useCompanion } from '../companion/useCompanion';
import { StackLayout } from '../layouts/StackLayout';
import { navigate, ROUTES } from '../routes';

export function AppsScreen() {
  const { apps, appState, focusApp } = useCompanion();

  return (
    <StackLayout title="Apps" onBack={() => navigate(ROUTES.home)}>
      <AppSwitcher
        apps={apps}
        currentBundle={appState?.bundleId || ''}
        onFocus={focusApp}
      />
    </StackLayout>
  );
}
