import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App';
import { installHaptics } from './companion/session';
import { installUpdater } from './lib/swUpdate';
import '@fontsource/barlow-condensed/400.css';
import '@fontsource/barlow-condensed/600.css';
import '@fontsource/barlow-condensed/700.css';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import '@companion/theme/tailwind.css';
import './motion.css';

installHaptics();
installUpdater();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
