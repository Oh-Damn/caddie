import type { ClipboardItem } from '@companion/protocol';
import { Trash2, X } from 'lucide-react';
import { Button } from '../ui/Button';
import { IconButton } from '../ui/IconButton';
import { clipImageSrc } from '../../lib/clipboardImage';

type Props = {
  item: ClipboardItem;
  deviceId: string;
  onClose: () => void;
  onPaste: () => void;
  onRemove: () => void;
};

export function ImageViewer({ item, deviceId, onClose, onPaste, onRemove }: Props) {
  return (
    <div className="fixed inset-0 z-50 flex flex-col gap-3 bg-app px-4 pb-6 pt-3">
      <div className="flex shrink-0 items-center justify-between gap-3">
        <span className="type-meta truncate">{item.preview}</span>
        <div className="flex shrink-0 items-center gap-3">
          <IconButton label="Remove" onClick={onRemove}>
            <Trash2 className="h-5 w-5 shrink-0" strokeWidth={2.2} />
          </IconButton>
          <IconButton label="Close" onClick={onClose}>
            <X className="h-5 w-5 shrink-0" strokeWidth={2.2} />
          </IconButton>
        </div>
      </div>
      <div className="flex min-h-0 flex-1 items-center justify-center overflow-auto">
        <img
          src={clipImageSrc(item.id, deviceId, true)}
          alt={item.preview}
          className="max-h-full max-w-full rounded-app object-contain"
        />
      </div>
      <Button onClick={onPaste}>Paste on Mac</Button>
    </div>
  );
}
