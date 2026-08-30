type Props = {
  variant?: 'startup' | 'footer';
  version?: string;
};

export function PoweredBy({ variant = 'footer', version }: Props) {
  const startup = variant === 'startup';
  return (
    <div
      className={
        startup
          ? 'flex flex-col items-center gap-4'
          : 'flex shrink-0 items-center justify-center gap-s2 pb-1 opacity-55'
      }
    >
      <img
        src="/oh-damn-logo.svg"
        alt=""
        width={1162}
        height={500}
        className={startup ? 'w-40 max-w-[60%]' : 'w-14'}
      />
      <p className="type-micro text-app-muted">
        POWERED BY OHDAMN!{version ? ` \u00b7 v${version}` : ''}
      </p>
    </div>
  );
}
