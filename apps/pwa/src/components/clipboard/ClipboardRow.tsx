import type { ClipboardItem } from '@companion/protocol';
import { X } from 'lucide-react';
import { clipImageSrc } from '../../lib/clipboardImage';

type Props = {
  item: ClipboardItem;
  deviceId: string;
  onSelect: (item: ClipboardItem) => void;
  onRemove: (item: ClipboardItem) => void;
};

export function ClipboardRow({ item, deviceId, onSelect, onRemove }: Props) {
  const isImage = item.kind === 'image';

  return (
    <div className="flex items-center">
      <button
        type="button"
        className="flex min-h-touch min-w-0 flex-1 items-center gap-3 px-3 py-2.5 text-left"
        onClick={() => onSelect(item)}
      >
        {isImage && deviceId ? (
          <img
            src={clipImageSrc(item.id, deviceId)}
            alt=""
            loading="lazy"
            decoding="async"
            className="h-12 w-12 shrink-0 rounded-app border border-app-border object-cover"
          />
        ) : null}
        <span className="flex min-w-0 flex-col">
          <span className="type-secondary line-clamp-2">{item.preview}</span>
          <span className="type-meta">{kindLabel(item)}</span>
        </span>
      </button>
      <button
        type="button"
        aria-label="Remove"
        className="flex h-touch w-touch shrink-0 items-center justify-center text-app-muted"
        onClick={() => onRemove(item)}
      >
        <X className="h-4 w-4 shrink-0" strokeWidth={2.2} />
      </button>
    </div>
  );
}

function kindLabel(item: ClipboardItem): string {
  if (item.kind === 'image') {
    if (item.width && item.height) {
      return `${item.width} \u00d7 ${item.height}`;
    }
    return 'Image';
  }
  return isLink(item.text || item.preview) ? 'Link' : 'Text';
}

function isLink(text: string): boolean {
  return /^https?:\/\//i.test(text.trim());
}
