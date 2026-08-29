type Props = {
  bundleId: string;
  name: string;
  className?: string;
};

export function AppMark({ bundleId, name, className }: Props) {
  const letter = (name.trim().charAt(0) || '?').toUpperCase();
  return (
    <span
      className={
        className ??
        'flex h-11 w-11 items-center justify-center rounded-app bg-app-elevated border border-app-border'
      }
      aria-hidden
    >
      {mark(bundleId, Boolean(className?.includes('h-28'))) ?? (
        <span
          className={
            className?.includes('h-28')
              ? 'text-4xl font-semibold text-accent'
              : 'text-sm font-semibold text-accent'
          }
        >
          {letter}
        </span>
      )}
    </span>
  );
}

function mark(bundleId: string, large: boolean) {
  const common = large ? 'h-16 w-16' : 'h-6 w-6';
  switch (bundleId) {
    case 'com.google.Chrome':
    case 'com.google.Chrome.canary':
      return (
        <svg viewBox="0 0 24 24" className={common}>
          <circle
            cx="12"
            cy="12"
            r="9"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <circle cx="12" cy="12" r="3.5" fill="var(--accent)" />
        </svg>
      );
    case 'com.apple.Safari':
      return (
        <svg viewBox="0 0 24 24" className={common}>
          <circle
            cx="12"
            cy="12"
            r="9"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <path d="M12 5 L15 15 L12 13 L9 15 Z" fill="var(--accent)" />
        </svg>
      );
    case 'com.spotify.client':
      return (
        <svg viewBox="0 0 24 24" className={common}>
          <circle
            cx="12"
            cy="12"
            r="9"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <path
            d="M7 10c3-1.5 7-1.5 10 0M8 13c2.4-1 5.6-1 8 0M9 16c1.6-.6 3.4-.6 5 0"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="1.6"
            strokeLinecap="round"
          />
        </svg>
      );
    case 'com.apple.Music':
      return (
        <svg viewBox="0 0 24 24" className={common}>
          <path
            d="M9 18V7l10-2v11"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="2"
            strokeLinecap="round"
          />
          <circle cx="7" cy="18" r="2.5" fill="currentColor" />
          <circle cx="17" cy="16" r="2.5" fill="currentColor" />
        </svg>
      );
    case 'com.microsoft.VSCode':
    case 'com.microsoft.VSCodeInsiders':
      return (
        <svg viewBox="0 0 24 24" className={common}>
          <path
            d="M4 8 L10 12 L4 16 M10 12 L20 6 V18 Z"
            fill="none"
            stroke="var(--accent)"
            strokeWidth="2"
            strokeLinejoin="round"
          />
        </svg>
      );
    case 'company.thebrowser.Browser':
    case 'com.brave.Browser':
      return (
        <svg viewBox="0 0 24 24" className={common}>
          <circle
            cx="12"
            cy="12"
            r="9"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
          />
          <rect x="8" y="8" width="8" height="8" rx="1.5" fill="var(--accent)" />
        </svg>
      );
    default:
      return null;
  }
}
