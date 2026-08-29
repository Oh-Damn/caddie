import type { ApprovalSession } from '@companion/protocol';
import { Check, X } from 'lucide-react';
import { useEffect, useRef } from 'react';
import { haptic } from '../../companion/session';
import { IconButton } from '../ui/IconButton';

type Props = {
  approval: ApprovalSession;
  onCommand: (action: string, value?: number | null, target?: string | null) => void;
};

export function ApprovalStrip({ approval, onCommand }: Props) {
  const seen = useRef('');
  useEffect(() => {
    const id = `${approval.app}:${approval.bundleId}`;
    if (seen.current === id) return;
    seen.current = id;
    haptic('warning');
  }, [approval.app, approval.bundleId]);

  return (
    <div className="flex items-center gap-2 faceplate px-2.5 py-2">
      <button
        type="button"
        className="min-w-0 flex-1 py-1 text-left"
        onClick={() => onCommand('app.focus', null, approval.bundleId)}
      >
        <p className="type-secondary truncate">{approval.appName}</p>
        <p className="type-meta mt-0.5 truncate">
          {approval.title || 'Waiting for approval'}
        </p>
      </button>
      {approval.canAllow ? (
        <IconButton
          label="Allow"
          variant="filled"
          size="md"
          onClick={() => onCommand('approval.allow')}
        >
          <Check className="h-4 w-4" strokeWidth={2.2} />
        </IconButton>
      ) : null}
      {approval.canDeny ? (
        <IconButton
          label="Deny"
          variant="chip"
          size="md"
          onClick={() => onCommand('approval.deny')}
        >
          <X className="h-4 w-4" strokeWidth={2.2} />
        </IconButton>
      ) : null}
    </div>
  );
}
