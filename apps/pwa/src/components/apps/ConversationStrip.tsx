import type { Conversation, ConversationKind } from '@companion/protocol';
import { Bookmark, Plus, X } from 'lucide-react';
import { useState, type ReactNode } from 'react';

type Props = {
  recent: Conversation[];
  pinned: Conversation[];
  onOpen: (conversation: Conversation) => void;
  onTogglePin: (conversation: Conversation) => void;
  onAdd?: (name: string) => void;
};

const NO_BAR = '[scrollbar-width:none] [&::-webkit-scrollbar]:hidden';

export function ConversationStrip({ recent, pinned, onOpen, onTogglePin, onAdd }: Props) {
  if (recent.length === 0 && pinned.length === 0 && !onAdd) {
    return null;
  }

  return (
    <div className="flex flex-col gap-2">
      {pinned.length > 0 || onAdd ? (
        <Section label="Favorites">
          <div className={`flex gap-1.5 overflow-x-auto pb-0.5 ${NO_BAR}`}>
            {pinned.map((item) => (
              <Chip
                key={`fav-${item.kind}-${item.name}`}
                item={item}
                pinned
                onOpen={onOpen}
                onTogglePin={onTogglePin}
              />
            ))}
            {onAdd ? <AddPerson onAdd={onAdd} /> : null}
          </div>
        </Section>
      ) : null}

      {recent.length > 0 ? (
        <Section label="Recents">
          <div className={`flex gap-1.5 overflow-x-auto pb-0.5 ${NO_BAR}`}>
            {recent.map((item) => (
              <Chip
                key={`recent-${item.kind}-${item.name}`}
                item={item}
                pinned={false}
                onOpen={onOpen}
                onTogglePin={onTogglePin}
              />
            ))}
          </div>
        </Section>
      ) : null}
    </div>
  );
}

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="min-w-0">
      <p className="type-micro px-0.5 pb-1">{label}</p>
      {children}
    </div>
  );
}

function Chip({
  item,
  pinned,
  onOpen,
  onTogglePin,
}: {
  item: Conversation;
  pinned: boolean;
  onOpen: (conversation: Conversation) => void;
  onTogglePin: (conversation: Conversation) => void;
}) {
  return (
    <div className="key flex h-touch shrink-0 items-center pl-2.5">
      <button
        type="button"
        className="flex items-center gap-1.5"
        onClick={() => onOpen(item)}
      >
        <KindMark kind={item.kind} name={item.name} compact />
        <span className="max-w-28 truncate text-xs">{item.name}</span>
      </button>
      <PinButton
        pinned={pinned}
        name={item.name}
        compact
        onClick={() => onTogglePin(item)}
      />
    </div>
  );
}

function AddPerson({ onAdd }: { onAdd: (name: string) => void }) {
  const [open, setOpen] = useState(false);
  const [draft, setDraft] = useState('');

  if (!open) {
    return (
      <button
        type="button"
        aria-label="Add a person"
        onClick={() => setOpen(true)}
        className="key flex h-touch w-touch shrink-0 items-center justify-center border-dashed text-app-muted"
      >
        <Plus className="h-4 w-4" strokeWidth={2.2} />
      </button>
    );
  }

  return (
    <form
      className="key flex h-touch shrink-0 items-center gap-1 border-accent pl-3 pr-1"
      onSubmit={(e) => {
        e.preventDefault();
        const name = draft.trim();
        if (!name) return;
        onAdd(name);
        setDraft('');
        setOpen(false);
      }}
    >
      <input
        autoFocus
        value={draft}
        onChange={(e) => setDraft(e.target.value)}
        placeholder="Name"
        aria-label="Person name"
        className="w-28 min-w-0 bg-transparent text-base outline-none placeholder:text-app-muted"
      />
      <button
        type="button"
        aria-label="Cancel"
        onClick={() => {
          setDraft('');
          setOpen(false);
        }}
        className="flex h-7 w-7 shrink-0 items-center justify-center text-app-muted"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </form>
  );
}

function PinButton({
  pinned,
  name,
  onClick,
  compact = false,
}: {
  pinned: boolean;
  name: string;
  onClick: () => void;
  compact?: boolean;
}) {
  return (
    <button
      type="button"
      aria-label={pinned ? `Remove bookmark ${name}` : `Bookmark ${name}`}
      className={`flex shrink-0 items-center justify-center ${compact ? 'h-9 w-9' : 'h-8 w-8'}`}
      onClick={onClick}
    >
      <Bookmark
        className={`h-3.5 w-3.5 ${pinned ? 'fill-accent text-accent' : 'text-app-muted'}`}
      />
    </button>
  );
}

function KindMark({
  kind,
  name,
  compact = false,
}: {
  kind: ConversationKind;
  name: string;
  compact?: boolean;
}) {
  const size = compact ? 'h-6 w-6 text-[9px]' : 'h-8 w-8 text-[10px]';
  return (
    <span
      className={`flex shrink-0 items-center justify-center rounded-app bg-app-well text-app-muted ${size}`}
    >
      {kind === 'channel' ? '#' : kind === 'thread' ? 'T' : initials(name)}
    </span>
  );
}

function initials(name: string): string {
  const cleaned = name.replace(/^[@#]/, '').trim();
  const parts = cleaned.split(/\s+/).filter(Boolean);
  if (parts.length >= 2) {
    return `${parts[0][0] ?? ''}${parts[1][0] ?? ''}`.toUpperCase();
  }
  return cleaned.slice(0, 2).toUpperCase() || '?';
}
