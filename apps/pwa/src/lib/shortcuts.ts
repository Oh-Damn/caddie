export function shortcutHint(action: string): string | null {
  if (action.startsWith('shortcut.send:')) {
    return chordHint(action.slice('shortcut.send:'.length));
  }
  switch (action) {
    case 'shortcut.undo':
      return '⌘Z';
    case 'shortcut.redo':
      return '⇧⌘Z';
    case 'shortcut.cut':
      return '⌘X';
    case 'shortcut.copy':
      return '⌘C';
    case 'shortcut.paste':
      return '⌘V';
    case 'shortcut.select_all':
      return '⌘A';
    case 'shortcut.save':
      return '⌘S';
    case 'shortcut.find':
      return '⌘F';
    default:
      return null;
  }
}

function chordHint(spec: string): string {
  const parts = spec
    .split('+')
    .map((p) => p.trim().toLowerCase())
    .filter(Boolean);
  if (parts.length === 0) return spec;
  const key = parts[parts.length - 1] ?? '';
  const mods = parts.slice(0, -1);
  const glyphs: string[] = [];
  if (mods.includes('control') || mods.includes('ctrl')) glyphs.push('⌃');
  if (mods.includes('alt') || mods.includes('option')) glyphs.push('⌥');
  if (mods.includes('shift')) glyphs.push('⇧');
  if (mods.includes('meta') || mods.includes('cmd') || mods.includes('command')) {
    glyphs.push('⌘');
  }
  glyphs.push(keyLabel(key));
  return glyphs.join('');
}

function keyLabel(key: string): string {
  if (key === 'grave' || key === 'backtick' || key === '`') return '`';
  if (key === 'slash' || key === '/') return '/';
  if (key === 'space') return 'Space';
  if (key === 'enter' || key === 'return') return '↩';
  if (key === 'escape' || key === 'esc') return 'Esc';
  if (key === 'tab') return 'Tab';
  if (key === 'backspace' || key === 'delete') return '⌫';
  if (key.length === 1) return key.toUpperCase();
  if (key.startsWith('f') && key.length <= 3) return key.toUpperCase();
  return key;
}
