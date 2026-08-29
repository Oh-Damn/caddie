import { PRODUCT_NAME } from '@companion/protocol';
import { Download, Share } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import {
  INSTALL_OPEN,
  isIos,
  isStandalone,
  loadInstallDismissed,
  saveInstallDismissed,
  type BeforeInstallPromptEvent,
} from '../../lib/install';
import { BrandMark } from './BrandMark';
import { Button } from './Button';

export function InstallPopup() {
  const [open, setOpen] = useState(false);
  const deferred = useRef<BeforeInstallPromptEvent | null>(null);
  const [canPrompt, setCanPrompt] = useState(false);

  useEffect(() => {
    if (isStandalone()) return;
    if (!loadInstallDismissed()) setOpen(true);

    const onPrompt = (event: Event) => {
      event.preventDefault();
      deferred.current = event as BeforeInstallPromptEvent;
      setCanPrompt(true);
    };
    const onInstalled = () => setOpen(false);
    const onOpen = () => {
      if (!isStandalone()) setOpen(true);
    };

    window.addEventListener('beforeinstallprompt', onPrompt);
    window.addEventListener('appinstalled', onInstalled);
    window.addEventListener(INSTALL_OPEN, onOpen);
    return () => {
      window.removeEventListener('beforeinstallprompt', onPrompt);
      window.removeEventListener('appinstalled', onInstalled);
      window.removeEventListener(INSTALL_OPEN, onOpen);
    };
  }, []);

  if (!open || isStandalone()) return null;

  const dismiss = () => {
    saveInstallDismissed();
    setOpen(false);
  };

  const install = async () => {
    const event = deferred.current;
    if (!event) {
      dismiss();
      return;
    }
    await event.prompt();
    deferred.current = null;
    setCanPrompt(false);
    saveInstallDismissed();
    setOpen(false);
  };

  const ios = isIos();

  return (
    <div
      className="fixed inset-0 z-20 flex items-center justify-center bg-app-well/80 px-4"
      role="dialog"
      aria-modal="true"
      aria-labelledby="install-title"
      aria-describedby="install-message"
    >
      <div className="faceplate w-full max-w-sm p-4">
        <BrandMark className="h-12 w-12" />
        <h2 id="install-title" className="type-primary mt-3">
          Install {PRODUCT_NAME}
        </h2>
        <p id="install-message" className="type-secondary mt-2 text-app-muted">
          {ios ? (
            <>
              Tap <Share className="mb-0.5 inline h-3.5 w-3.5" aria-label="Share" />{' '}
              Share, then Add to Home Screen.
            </>
          ) : canPrompt ? (
            'Add this app to your home screen for quicker controls.'
          ) : (
            'Install from the browser menu, then open it from your home screen.'
          )}
        </p>
        <div className="mt-4 flex flex-col gap-2">
          {canPrompt ? (
            <>
              <Button onClick={() => void install()}>
                <Download className="h-4 w-4" />
                Install
              </Button>
              <Button variant="ghost" onClick={dismiss}>
                Not now
              </Button>
            </>
          ) : (
            <Button onClick={dismiss}>Got it</Button>
          )}
        </div>
      </div>
    </div>
  );
}
