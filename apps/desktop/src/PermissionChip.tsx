type Props = {
  ok: boolean;
  label: string;
};

export function PermissionChip({ ok, label }: Props) {
  return (
    <p
      className={`faceplate flex items-center gap-2 px-3 py-2 type-secondary ${ok ? '' : 'text-danger'}`}
    >
      <span
        className="led"
        style={{
          color: ok ? 'var(--status-ready)' : 'var(--status-error)',
          background: ok ? 'var(--status-ready)' : 'var(--status-error)',
        }}
        aria-hidden
      />
      {label}: {ok ? 'on' : 'off'}
    </p>
  );
}
