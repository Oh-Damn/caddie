type Props = {
  children: React.ReactNode;
  onClick?: () => void;
  disabled?: boolean;
  variant?: 'primary' | 'ghost' | 'danger';
};

export function Button({ children, onClick, disabled, variant = 'primary' }: Props) {
  const look =
    variant === 'primary'
      ? 'key key-lit'
      : variant === 'danger'
        ? 'key key-latch'
        : 'key text-app-text';
  return (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`h-10 min-w-24 px-4 type-secondary disabled:opacity-50 ${look}`}
    >
      {children}
    </button>
  );
}
