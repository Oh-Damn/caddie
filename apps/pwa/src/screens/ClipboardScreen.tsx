import { useEffect, useMemo, useState } from 'react';
import type { ClipboardItem } from '@companion/protocol';
import { Search, Trash2 } from 'lucide-react';
import { ClipboardRow } from '../components/clipboard/ClipboardRow';
import { ImageViewer } from '../components/clipboard/ImageViewer';
import { Field } from '../components/ui/Field';
import { IconButton } from '../components/ui/IconButton';
import { useCompanion } from '../companion/useCompanion';
import { useCommand } from '../companion/useCommand';
import { StackLayout } from '../layouts/StackLayout';
import { navigate, ROUTES } from '../routes';

export function ClipboardScreen() {
  const { clipboardItems, deviceId, dropClipboard, clearClipboard } = useCompanion();
  const go = useCommand();
  const [query, setQuery] = useState('');
  const [viewing, setViewing] = useState<ClipboardItem | null>(null);
  const [confirmClear, setConfirmClear] = useState(false);
  const items = clipboardItems ?? [];
  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const list = clipboardItems ?? [];
    if (!needle) return list;
    return list.filter(
      (item) =>
        item.preview.toLowerCase().includes(needle) ||
        item.text.toLowerCase().includes(needle),
    );
  }, [clipboardItems, query]);

  useEffect(() => {
    if (!confirmClear) return;
    const id = window.setTimeout(() => setConfirmClear(false), 3000);
    return () => window.clearTimeout(id);
  }, [confirmClear]);

  const select = (item: ClipboardItem) => {
    if (item.kind === 'image') {
      setViewing(item);
      return;
    }
    go('clipboard.paste', null, item.id);
  };

  const remove = (item: ClipboardItem) => {
    if (viewing?.id === item.id) setViewing(null);
    dropClipboard(item.id);
  };

  return (
    <StackLayout
      title="Clipboard"
      onBack={() => navigate(ROUTES.home)}
      trailing={
        items.length > 0 ? (
          <IconButton
            label={confirmClear ? 'Confirm clear' : 'Clear clipboard'}
            latch={confirmClear}
            active={confirmClear}
            onClick={() => {
              if (!confirmClear) {
                setConfirmClear(true);
                return;
              }
              setConfirmClear(false);
              setViewing(null);
              clearClipboard();
            }}
          >
            <Trash2 className="h-5 w-5 shrink-0" strokeWidth={2.2} />
          </IconButton>
        ) : null
      }
    >
      <div className="flex flex-col gap-3 pb-4">
        <Field
          value={query}
          onChange={setQuery}
          placeholder="Filter copied items"
          icon={<Search className="h-4 w-4" />}
        />
        {items.length === 0 ? (
          <p className="type-meta px-1">
            Copy something on your Mac. Recent text and images show up here.
          </p>
        ) : filtered.length === 0 ? (
          <p className="type-meta px-1">No matches.</p>
        ) : (
          <ul className="faceplate overflow-hidden">
            {filtered.map((item) => (
              <li key={item.id} className="border-b border-app-border last:border-b-0">
                <ClipboardRow
                  item={item}
                  deviceId={deviceId}
                  onSelect={select}
                  onRemove={remove}
                />
              </li>
            ))}
          </ul>
        )}
      </div>
      {viewing ? (
        <ImageViewer
          item={viewing}
          deviceId={deviceId}
          onClose={() => setViewing(null)}
          onPaste={() => {
            go('clipboard.paste', null, viewing.id);
            setViewing(null);
          }}
          onRemove={() => remove(viewing)}
        />
      ) : null}
    </StackLayout>
  );
}
