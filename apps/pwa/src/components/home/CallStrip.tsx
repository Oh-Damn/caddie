import type { CallSession } from '@companion/protocol';
import { Headphones, Mic, MicOff, PhoneOff, Video } from 'lucide-react';
import { useEffect, useState } from 'react';
import { packTabRef } from '../../companion/tabs';
import { IconButton } from '../ui/IconButton';

type Props = {
  call: CallSession;
  onCommand: (action: string, value?: number | null, target?: string | null) => void;
};

export function CallStrip({ call, onCommand }: Props) {
  const [confirmLeave, setConfirmLeave] = useState(false);

  useEffect(() => {
    if (!confirmLeave) return;
    const id = window.setTimeout(() => setConfirmLeave(false), 3000);
    return () => window.clearTimeout(id);
  }, [confirmLeave]);

  const focusCall = () => {
    if (call.app === 'meet' && call.tabWindowIndex && call.tabIndex) {
      onCommand(
        'browser.activate_tab',
        packTabRef(call.tabWindowIndex, call.tabIndex),
        call.bundleId,
      );
      return;
    }
    onCommand('app.focus', null, call.bundleId);
  };

  return (
    <div className="flex items-center gap-2 faceplate px-2.5 py-2">
      <button type="button" className="min-w-0 flex-1 py-1 text-left" onClick={focusCall}>
        <p className="type-secondary truncate">{call.appName}</p>
        <p className="type-meta mt-0.5 truncate">{call.title || 'In a call'}</p>
      </button>
      <IconButton
        label={call.muted ? 'Unmute mic' : 'Mute mic'}
        variant={call.muted ? 'filled' : 'chip'}
        size="md"
        onClick={() => onCommand('call.toggle_mic')}
      >
        {call.muted ? <MicOff className="h-4 w-4" /> : <Mic className="h-4 w-4" />}
      </IconButton>
      {call.hasCamera ? (
        <IconButton
          label="Toggle camera"
          variant="chip"
          size="md"
          onClick={() => onCommand('call.toggle_camera')}
        >
          <Video className="h-4 w-4" />
        </IconButton>
      ) : null}
      {call.hasDeafen ? (
        <IconButton
          label="Deafen"
          variant="chip"
          size="md"
          onClick={() => onCommand('call.deafen')}
        >
          <Headphones className="h-4 w-4" />
        </IconButton>
      ) : null}
      {call.hasLeave ? (
        <IconButton
          label={confirmLeave ? 'Confirm leave' : 'Leave'}
          variant={confirmLeave ? 'filled' : 'chip'}
          size="md"
          onClick={() => {
            if (!confirmLeave) {
              setConfirmLeave(true);
              return;
            }
            setConfirmLeave(false);
            onCommand('call.leave');
          }}
        >
          <PhoneOff className="h-4 w-4" />
        </IconButton>
      ) : null}
    </div>
  );
}
