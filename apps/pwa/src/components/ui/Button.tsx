import type { ReactNode } from 'react';

type Props = {
  children: ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  variant?: 'primary' | 'ghost' | 'danger';
};

export function Button({ children, onClick, disabled, variant = 'primary' }: Props) {
  const look =
    variant === 'danger'
      ? 'key key-latch'
      : variant === 'ghost'
        ? 'key text-app-text'
        : 'key key-lit';

  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`flex h-touch w-full items-center justify-center gap-2 px-4 disabled:opacity-50 ${look}`}
    >
      {children}
    </button>
  );
}
