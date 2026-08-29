import type { LayoutPayload, Widget } from '@companion/protocol';

function button(id: string, title: string, action: string): Widget {
  return { type: 'button', id, title, action };
}

export function genericLayout(title = 'Desktop'): LayoutPayload {
  return {
    screen: 'generic',
    title,
    widgets: [
      {
        type: 'stack',
        id: 'root',
        children: [
          {
            type: 'row',
            id: 'edit',
            children: [
              button('undo', 'Undo', 'shortcut.undo'),
              button('redo', 'Redo', 'shortcut.redo'),
              button('save', 'Save', 'shortcut.save'),
            ],
          },
          {
            type: 'row',
            id: 'clip',
            children: [
              button('cut', 'Cut', 'shortcut.cut'),
              button('copy', 'Copy', 'shortcut.copy'),
              button('paste', 'Paste', 'shortcut.paste'),
            ],
          },
          {
            type: 'row',
            id: 'find',
            children: [
              button('select', 'Select all', 'shortcut.select_all'),
              button('find', 'Find', 'shortcut.find'),
            ],
          },
        ],
      },
    ],
  };
}
