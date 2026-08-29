import type { ReactNode } from 'react';
import { ChevronLeft } from 'lucide-react';
import { IconButton } from '../components/ui/IconButton';
import { PoweredBy } from '../components/ui/PoweredBy';

type Props = {
  title?: string;
  onBack?: () => void;
  children: ReactNode;
  large?: boolean;
  trailing?: ReactNode;

  branded?: boolean;
};

export function StackLayout({
  title,
  onBack,
  children,
  large = false,
  trailing,
  branded = true,
}: Props) {
  return (
    <section className="flex min-h-0 flex-1 flex-col gap-4 overflow-hidden pt-2">
      {onBack || title || trailing ? (
        <div className="flex shrink-0 items-center gap-3">
          {onBack ? (
            <IconButton label="Back" onClick={onBack}>
              <ChevronLeft className="h-5 w-5 shrink-0" strokeWidth={2.2} />
            </IconButton>
          ) : null}
          {title ? (
            <h1
              className={
                large
                  ? 'type-primary min-w-0 flex-1 text-[1.6rem]'
                  : 'type-primary min-w-0 flex-1'
              }
            >
              {title}
            </h1>
          ) : null}
          {trailing ? <div className="ml-auto shrink-0">{trailing}</div> : null}
        </div>
      ) : null}
      <div className="min-h-0 flex-1 overflow-y-auto">
        {children}
        {branded ? <PoweredBy /> : null}
      </div>
    </section>
  );
}
