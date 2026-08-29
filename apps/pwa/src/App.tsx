import { CompanionProvider } from './companion/CompanionProvider';
import { MotionPref } from './components/ui/MotionPref';
import { Splash } from './components/ui/Splash';
import { AppShell } from './layouts/AppShell';
import { Layer, LayerHost } from './layouts/LayerHost';
import { ROUTES } from './routes';
import { AiScreen } from './screens/AiScreen';
import { AppsScreen } from './screens/AppsScreen';
import { ClipboardScreen } from './screens/ClipboardScreen';
import { ConnectScreen } from './screens/ConnectScreen';
import { HomeScreen } from './screens/HomeScreen';
import { SettingsScreen } from './screens/SettingsScreen';
import { useRoute } from './useRoute';

export default function App() {
  return (
    <CompanionProvider>
      <MotionPref />
      <AppShell>
        <Routes />
      </AppShell>
      <Splash />
    </CompanionProvider>
  );
}

function Routes() {
  const { route } = useRoute();
  if (route === ROUTES.connect) return <ConnectScreen />;
  return (
    <LayerHost>
      <Layer active={route === ROUTES.home}>
        <HomeScreen />
      </Layer>
      <Layer active={route === ROUTES.apps}>
        <AppsScreen />
      </Layer>
      <Layer active={route === ROUTES.clipboard}>
        <ClipboardScreen />
      </Layer>
      <Layer active={route === ROUTES.ai}>
        <AiScreen />
      </Layer>
      <Layer active={route === ROUTES.settings}>
        <SettingsScreen />
      </Layer>
    </LayerHost>
  );
}
