export const INSTALL_OPEN = 'caddie:install';

const DISMISS_KEY = 'caddie.installDismissed';

export type BeforeInstallPromptEvent = Event & {
  prompt: () => Promise<void>;
  userChoice: Promise<{ outcome: 'accepted' | 'dismissed' }>;
};

export function isStandalone(): boolean {
  return (
    window.matchMedia('(display-mode: standalone)').matches ||
    window.matchMedia('(display-mode: fullscreen)').matches ||
    ('standalone' in navigator &&
      (navigator as { standalone?: boolean }).standalone === true)
  );
}

export function isIos(): boolean {
  return (
    /iPad|iPhone|iPod/.test(navigator.userAgent) ||
    (navigator.platform === 'MacIntel' && navigator.maxTouchPoints > 1)
  );
}

export function loadInstallDismissed(): boolean {
  return localStorage.getItem(DISMISS_KEY) === '1';
}

export function saveInstallDismissed(): void {
  localStorage.setItem(DISMISS_KEY, '1');
}

export function openInstallPopup(): void {
  window.dispatchEvent(new Event(INSTALL_OPEN));
}
