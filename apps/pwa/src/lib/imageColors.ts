export type Glow = {
  a: string;
  b: string;
};

const DEFAULT_GLOW: Glow = {
  a: 'rgba(197, 163, 255, 0.5)',
  b: 'rgba(197, 163, 255, 0.35)',
};

export function artworkSrc(url: string): string {
  return `/api/artwork?url=${encodeURIComponent(url)}`;
}

function rgba(r: number, g: number, b: number, a: number): string {
  return `rgba(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)}, ${a})`;
}

function hexToRgb(hex: string): { r: number; g: number; b: number } | null {
  const raw = hex.replace('#', '');
  if (raw.length !== 6) return null;
  const n = Number.parseInt(raw, 16);
  if (Number.isNaN(n)) return null;
  return { r: (n >> 16) & 255, g: (n >> 8) & 255, b: n & 255 };
}

export function accentToGlow(accent: string): Glow {
  const rgb = hexToRgb(accent);
  if (!rgb) return DEFAULT_GLOW;
  return {
    a: rgba(rgb.r, rgb.g, rgb.b, 0.5),
    b: rgba(Math.min(255, rgb.r * 0.7 + 80), Math.min(255, rgb.g * 0.7 + 40), rgb.b, 0.4),
  };
}
