export type TabTitle = {
  page: string;
  site: string;
  unread: number | null;
};

const SEPARATORS = [' - ', ' | ', ' \u2014 ', ' \u2013 ', ' \u00b7 '];
const MAX_SITE_WORDS = 4;

export function parseTabTitle(raw: string): TabTitle {
  let rest = raw.trim();
  let unread: number | null = null;

  const badge = /^\((\d+)\)\s*/.exec(rest);
  if (badge) {
    unread = Number(badge[1]);
    rest = rest.slice(badge[0].length);
  }

  for (const sep of SEPARATORS) {
    const at = rest.lastIndexOf(sep);
    if (at <= 0) continue;
    const tail = rest.slice(at + sep.length).trim();
    const head = rest.slice(0, at).trim();
    if (!tail || !head) continue;
    if (tail.split(/\s+/).length > MAX_SITE_WORDS) continue;
    return { page: head, site: tail, unread };
  }

  return { page: rest || raw.trim(), site: '', unread };
}

const APP_SUFFIX_SEPS = [' - ', ' \u2014 ', ' \u2013 ', ' | '];
const DIRTY = /^[\u25cf\u2022]\s*/;

export function formatLocation(page: string, site: string): string {
  if (site && site.toLowerCase() === 'claude') return page;
  return [page, site].filter(Boolean).join(' \u00b7 ');
}

export function parseWindowTitle(raw: string, appName: string): string {
  let rest = raw.trim();
  if (!rest) return '';
  const names = [
    appName,
    'Cursor',
    'Code',
    'Visual Studio Code',
    'VS Code',
    'Claude Desktop',
    'Claude',
  ]
    .map((n) => n.trim())
    .filter(Boolean);
  for (const name of names) {
    for (const sep of APP_SUFFIX_SEPS) {
      const suf = `${sep}${name}`;
      if (rest.endsWith(suf) && rest.length > suf.length) {
        rest = rest.slice(0, -suf.length).trim();
      }
    }
  }
  rest = rest.replace(DIRTY, '');
  const { page, site } = parseTabTitle(rest);
  return formatLocation(page, site);
}
