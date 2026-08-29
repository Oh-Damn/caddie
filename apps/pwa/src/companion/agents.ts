import type { AgentWindow } from '@companion/protocol';

export type Tone = 'idle' | 'ready' | 'wait' | 'error';

export function toneFor(percent: number): Tone {
  if (percent >= 90) return 'error';
  if (percent >= 65) return 'wait';
  if (percent > 0) return 'ready';
  return 'idle';
}

export const TONE_VAR: Record<Tone, string> = {
  idle: 'var(--status-idle)',
  ready: 'var(--status-ready)',
  wait: 'var(--status-wait)',
  error: 'var(--status-error)',
};

export function formatTokens(value: number): string {
  if (value >= 1_000_000)
    return `${(value / 1_000_000).toFixed(value >= 10_000_000 ? 0 : 1)}M`;
  if (value >= 1_000) return `${Math.round(value / 1_000)}K`;
  return String(value);
}

export function resetLabel(window: AgentWindow | null, now = Date.now()): string {
  if (!window?.resetsAt) return window?.label ?? '';
  const at = Date.parse(window.resetsAt);
  if (Number.isNaN(at)) return window.label;
  const mins = Math.round((at - now) / 60_000);
  if (mins <= 0) return 'Resetting';
  if (mins < 60) return `Resets in ${mins} min`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `Resets in ${hours}h ${mins % 60}m`;
  return `Resets in ${Math.round(hours / 24)}d`;
}

export function agoLabel(iso: string, now = Date.now()): string {
  const at = Date.parse(iso);
  if (Number.isNaN(at)) return '';
  const secs = Math.max(0, Math.round((now - at) / 1000));
  if (secs < 60) return 'now';
  const mins = Math.round(secs / 60);
  if (mins < 60) return `${mins}m`;
  const hours = Math.round(mins / 60);
  if (hours < 24) return `${hours}h`;
  return `${Math.round(hours / 24)}d`;
}
