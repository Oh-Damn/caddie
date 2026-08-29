type Props = {
  variant?: 'page' | 'splash';
};

export function PoweredBy({ variant = 'page' }: Props) {
  const splash = variant === 'splash';
  return (
    <div
      className={
        splash
          ? 'flex flex-col items-center gap-5'
          : 'flex flex-col items-center gap-2 pt-8 pb-3 opacity-55'
      }
    >
      <img
        src="/oh-damn-logo.svg"
        alt=""
        width={1162}
        height={500}
        className={splash ? 'w-52 max-w-[62vw]' : 'w-20'}
      />
      <p className="type-micro text-app-muted">POWERED BY OHDAMN!</p>
    </div>
  );
}
