import type { LayoutPayload, Widget } from '@companion/protocol';
import { shortcutHint } from '../../lib/shortcuts';

type Props = {
  layout: LayoutPayload;
  onCommand: (action: string, value?: number | null) => void;
};

type GridCell = {
  id: string;
  title: string;
  action: string;
  span: 1 | 2;
  highlight?: boolean;
};

export function ShortcutGrid({ layout, onCommand }: Props) {
  const cells = gridCells(layout.widgets);
  if (cells.length === 0) return null;

  return (
    <div className="grid grid-cols-3 gap-1 landscape:grid-cols-6">
      {cells.map((cell) => {
        const hint = shortcutHint(cell.action);
        return (
          <button
            key={cell.id}
            type="button"
            className={`relative flex min-h-touch-lg flex-col justify-end px-3 py-2.5 text-left ${
              cell.highlight ? 'key key-lit' : 'key'
            } ${cell.span === 2 ? 'col-span-2' : ''}`}
            onClick={() => onCommand(cell.action)}
          >
            {hint ? (
              <span
                className={`absolute right-2 top-1.5 font-mono text-[9px] tracking-widest ${
                  cell.highlight ? 'text-accent-text/70' : 'text-app-muted'
                }`}
              >
                {hint}
              </span>
            ) : null}
            <span className="text-xs font-semibold tracking-wide">{cell.title}</span>
          </button>
        );
      })}
    </div>
  );
}

function gridCells(widgets: Widget[]): GridCell[] {
  const rows = flattenRows(widgets);
  const out: GridCell[] = [];
  for (const row of rows) {
    const buttons = row.filter(
      (w): w is Extract<Widget, { type: 'button' }> => w.type === 'button',
    );
    const wide = buttons.length === 2;
    buttons.forEach((b, i) => {
      out.push({
        id: b.id,
        title: b.title,
        action: b.action,
        span: wide && i === 0 ? 2 : 1,
        highlight: b.action === 'shortcut.save',
      });
    });
  }
  return out;
}

function flattenRows(widgets: Widget[]): Widget[][] {
  const out: Widget[][] = [];
  for (const w of widgets) {
    if (w.type === 'row') out.push(w.children);
    if (w.type === 'stack') out.push(...flattenRows(w.children));
  }
  return out;
}
