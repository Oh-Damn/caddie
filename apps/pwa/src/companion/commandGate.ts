export type CommandArgs = {
  action: string;
  value: number | null;
  target: string | null;
};

type Policy =
  | { kind: 'leading'; ms: number }
  | { kind: 'debounce'; ms: number; key?: string }
  | { kind: 'throttle'; ms: number; key?: string };

const POLICIES: Record<string, Policy> = {
  'volume.toggle_mute': { kind: 'leading', ms: 400 },
  'volume.up': { kind: 'debounce', ms: 120, key: 'volume.set' },
  'volume.down': { kind: 'debounce', ms: 120, key: 'volume.set' },
  'volume.set': { kind: 'debounce', ms: 120 },
  'media.play_pause': { kind: 'leading', ms: 300 },
  'media.prev': { kind: 'throttle', ms: 350 },
  'media.next': { kind: 'throttle', ms: 350 },
  'media.seek_back': { kind: 'throttle', ms: 500 },
  'media.seek_forward': { kind: 'throttle', ms: 500 },
  'media.shuffle': { kind: 'throttle', ms: 500 },
  'media.like': { kind: 'throttle', ms: 500 },
  'media.repeat': { kind: 'throttle', ms: 500 },
  'app.focus': { kind: 'throttle', ms: 120, key: 'app.focus' },
  'browser.activate_tab': { kind: 'throttle', ms: 120, key: 'browser.activate_tab' },
  'call.toggle_mic': { kind: 'leading', ms: 400 },
  'call.toggle_camera': { kind: 'throttle', ms: 500 },
  'call.deafen': { kind: 'leading', ms: 400 },
  'call.leave': { kind: 'leading', ms: 800 },
  'approval.allow': { kind: 'leading', ms: 600 },
  'approval.deny': { kind: 'leading', ms: 600 },
  'clipboard.copy': { kind: 'throttle', ms: 200, key: 'clipboard' },
  'clipboard.paste': { kind: 'throttle', ms: 200, key: 'clipboard' },
  'clipboard.remove': { kind: 'leading', ms: 300 },
  'clipboard.clear': { kind: 'leading', ms: 800 },
  'conversation.jump': { kind: 'throttle', ms: 400, key: 'conversation.jump' },
  'conversation.pin': { kind: 'leading', ms: 300 },
  'conversation.unpin': { kind: 'leading', ms: 300 },
};

function policyFor(action: string): Policy {
  if (POLICIES[action]) return POLICIES[action];
  if (action.startsWith('shortcut.')) {
    return { kind: 'throttle', ms: 200, key: 'shortcut' };
  }
  return { kind: 'throttle', ms: 150 };
}

function gateKey(args: CommandArgs, policy: Policy): string {
  if (policy.kind === 'leading') return args.action;
  return policy.key ?? args.action;
}

export class CommandGate {
  private timers = new Map<string, ReturnType<typeof setTimeout>>();
  private pending = new Map<string, CommandArgs>();
  private lastLeading = new Map<string, number>();
  private lastThrottle = new Map<string, number>();

  clear() {
    for (const timer of this.timers.values()) clearTimeout(timer);
    this.timers.clear();
    this.pending.clear();
    this.lastLeading.clear();
    this.lastThrottle.clear();
  }

  shouldBlockLeading(action: string): boolean {
    const policy = policyFor(action);
    if (policy.kind !== 'leading') return false;
    const last = this.lastLeading.get(action) ?? 0;
    return Date.now() - last < policy.ms;
  }

  markLeading(action: string) {
    const policy = policyFor(action);
    if (policy.kind === 'leading') {
      this.lastLeading.set(action, Date.now());
    }
  }

  dispatch(args: CommandArgs, send: (args: CommandArgs) => void) {
    const policy = policyFor(args.action);
    const key = gateKey(args, policy);

    if (policy.kind === 'debounce') {
      this.pending.set(key, args);
      const prev = this.timers.get(key);
      if (prev) clearTimeout(prev);
      const id = window.setTimeout(() => {
        this.timers.delete(key);
        const latest = this.pending.get(key);
        this.pending.delete(key);
        if (latest) send(latest);
      }, policy.ms);
      this.timers.set(key, id);
      return;
    }

    if (policy.kind === 'throttle') {
      this.pending.set(key, args);
      const now = Date.now();
      const last = this.lastThrottle.get(key) ?? 0;
      const elapsed = now - last;

      if (elapsed >= policy.ms) {
        this.lastThrottle.set(key, now);
        this.pending.delete(key);
        send(args);
        return;
      }

      if (this.timers.has(key)) return;
      const id = window.setTimeout(() => {
        this.timers.delete(key);
        this.lastThrottle.set(key, Date.now());
        const latest = this.pending.get(key);
        this.pending.delete(key);
        if (latest) send(latest);
      }, policy.ms - elapsed);
      this.timers.set(key, id);
      return;
    }

    send(args);
  }
}

export function outboundArgs(
  action: string,
  value: number | null,
  target: string | null,
  nextVolume: number,
): CommandArgs {
  if (action === 'volume.up' || action === 'volume.down') {
    return { action: 'volume.set', value: nextVolume, target: null };
  }
  return { action, value, target };
}
