import type { ReactNode } from 'react';

type Props = {
  label: string;
  onClick: () => void;
  children: ReactNode;
  variant?: 'ghost' | 'chip' | 'round' | 'filled';
  size?: 'sm' | 'md' | 'lg';
  active?: boolean;
  accent?: boolean;
  latch?: boolean;
  className?: string;
};

const SIZES = {
  sm: 'h-8 w-8',
  md: 'h-10 w-10',
  lg: 'h-14 w-14',
};

export function IconButton({
  label,
  onClick,
  children,
  variant = 'ghost',
  size = 'md',
  active = false,
  accent = false,
  latch = false,
  className = '',
}: Props) {
  const lit = variant === 'filled' || accent || active;
  const look = latch ? 'key key-latch' : lit ? 'key key-lit' : 'key text-app-muted';

  return (
    <button
      type="button"
      className={`flex shrink-0 items-center justify-center ${SIZES[size]} ${look} ${className}`}
      aria-label={label}
      aria-pressed={active || undefined}
      onClick={onClick}
    >
      {children}
    </button>
  );
}
