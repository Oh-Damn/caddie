import type { ReactNode } from 'react';

type Props = {
  label?: string;
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  icon?: ReactNode;
};

export function Field({ label, value, onChange, placeholder, icon }: Props) {
  const input = (
    <span className="relative block">
      {icon ? (
        <span className="pointer-events-none absolute inset-y-0 left-3 flex items-center text-app-muted">
          {icon}
        </span>
      ) : null}
      <input
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        aria-label={label ?? placeholder}
        className={`readout readout-flat h-touch w-full text-base text-lcd outline-none ${
          icon ? 'pl-10 pr-3' : 'px-3'
        }`}
      />
    </span>
  );
  if (!label) {
    return <label className="block">{input}</label>;
  }
  return (
    <label className="flex flex-col gap-1">
      <span className="type-micro px-0.5">{label}</span>
      {input}
    </label>
  );
}
