export const PROTOCOL_VERSION = 1;
export const DEFAULT_PORT = 7842;
export const PRODUCT_NAME = 'Caddie';
export const SERVICE_NAME = 'caddie';
export const LAN_HOST = 'caddie.local';

export type Envelope<T extends string, P> = {
  v: typeof PROTOCOL_VERSION;
  id: string;
  type: T;
  payload: P;
};

export type DeviceDetails = {
  model: string;
  platform: string;
  platformVersion: string;
};

export type HelloPayload = {
  deviceName: string;
  deviceId: string | null;
  details?: DeviceDetails;
};

export type PairPayload = {
  secret: string;
  deviceName: string;
  details?: DeviceDetails;
};

export type CommandPayload = {
  action: string;
  value: number | null;
  target: string | null;
};

export type PresencePayload = {
  visible: boolean;
};

export type QueryPayload = {
  kind: 'apps' | 'clipboard' | 'agents';
};

export type AppWindow = {
  title: string;
  index: number;
};

export type RunningApp = {
  name: string;
  bundleId: string;
  windows: AppWindow[];
};

export type AppsPayload = {
  items: RunningApp[];
};

export type HelloOkPayload = {
  serverName: string;
  trusted: boolean;
  deviceId: string | null;
};

export type PairOkPayload = {
  deviceId: string;
};

export type PairDenyPayload = {
  reason: string;
};

export type RepeatMode = 'off' | 'all' | 'one';

export type BrowserTab = {
  title: string;
  windowIndex: number;
  tabIndex: number;
  active: boolean;
  audible: boolean;
  media?: boolean;
  profile?: string;
};

export type NowPlaying = {
  title: string;
  artist: string;
  artworkUrl: string;
  playing: boolean;
  sourceName: string;
  sourceBundleId: string;
  positionSec: number;
  durationSec: number;
  shuffle: boolean;
  liked: boolean;
  repeatMode: RepeatMode;
};

export type CallApp = 'discord' | 'slack' | 'meet' | 'zoom' | 'teams';

export type CallSession = {
  active: boolean;
  app: CallApp;
  appName: string;
  bundleId: string;
  title: string;
  muted: boolean;
  hasCamera: boolean;
  hasDeafen: boolean;
  hasLeave: boolean;
  tabWindowIndex: number | null;
  tabIndex: number | null;
};

export type ApprovalApp = 'cursor' | 'claude';

export type ApprovalSession = {
  app: ApprovalApp;
  appName: string;
  bundleId: string;
  title: string;
  canAllow: boolean;
  canDeny: boolean;
};

export type ConversationKind = 'channel' | 'dm' | 'thread' | 'huddle' | 'other';

export type Conversation = {
  name: string;
  kind: ConversationKind;
  workspace: string;
  windowIndex: number | null;
};

export type AgentWindow = {
  label: string;
  used: number;

  limit: number | null;
  percent: number;
  resetsAt: string | null;
};

export type AgentSession = {
  id: string;
  project: string;
  branch: string;
  model: string;
  tokens: number;
  weighted: number;
  subagentTokens: number;
  lastActive: string;
  active: boolean;
  waiting: boolean;
  detail: string;
};

export type AgentProvider = {
  id: string;
  name: string;
  available: boolean;
  estimated: boolean;
  active: number;
  waiting: number;
  blocked: string | null;
  session: AgentWindow | null;
  week: AgentWindow | null;
  sessions: AgentSession[];
};

export type AgentsSummary = {
  percent: number;
  waiting: number;
  active: number;
};

export type AgentsPayload = {
  summary: AgentsSummary;
  providers: AgentProvider[];
};

export type StatePayload = {
  appName: string;
  bundleId: string;
  pluginId: string;
  windowTitle: string;
  volume: number;
  muted: boolean;
  nowPlaying: NowPlaying | null;
  browserNowPlaying: NowPlaying | null;
  browserMediaOwned?: boolean;
  tabs: BrowserTab[];
  call: CallSession | null;
  approval?: ApprovalSession | null;
  unread: number | null;
  currentConversation: Conversation | null;
  openConversations: Conversation[];
  recentConversations: Conversation[];
  pinnedConversations: Conversation[];
  agents: AgentsSummary | null;
};

export type ClipboardKind = 'text' | 'image';

export type ClipboardItem = {
  id: string;
  kind: ClipboardKind;
  preview: string;
  text: string;
  width?: number;
  height?: number;
};

export type ClipboardPayload = {
  items: ClipboardItem[];
};

export type Widget =
  | {
      type: 'button';
      id: string;
      title: string;
      action: string;
      target?: string;
      icon?: string;
      kind?: 'toggle';
      valueKey?: string;
    }
  | {
      type: 'slider';
      id: string;
      title: string;
      action: string;
      min: number;
      max: number;
      valueKey: 'volume';
    }
  | { type: 'now_playing'; id: string }
  | { type: 'row'; id: string; children: Widget[] }
  | { type: 'stack'; id: string; children: Widget[] };

export type LayoutPayload = {
  screen: string;
  title: string;
  widgets: Widget[];
};

export type AckPayload = { ok: boolean };

export type ErrorPayload = {
  code: string;
  message: string;
};

export type ClientMessage =
  | Envelope<'hello', HelloPayload>
  | Envelope<'pair', PairPayload>
  | Envelope<'command', CommandPayload>
  | Envelope<'query', QueryPayload>
  | Envelope<'presence', PresencePayload>;

export type ServerMessage =
  | Envelope<'hello_ok', HelloOkPayload>
  | Envelope<'pair_ok', PairOkPayload>
  | Envelope<'pair_deny', PairDenyPayload>
  | Envelope<'layout', LayoutPayload>
  | Envelope<'state', StatePayload>
  | Envelope<'apps', AppsPayload>
  | Envelope<'clipboard', ClipboardPayload>
  | Envelope<'agents', AgentsPayload>
  | Envelope<'ack', AckPayload>
  | Envelope<'ping', AckPayload>
  | Envelope<'error', ErrorPayload>;

export type ConnectedDevice = {
  name: string;
  model: string;
  platform: string;
  platformVersion: string;
  ip: string;
  mac: string;
  pairedAt: string;
  lastSeen: string;
  connected: boolean;
};

export type SessionInfo = {
  httpUrl: string;
  fallbackHttpUrl: string;
  wsUrl: string;
  pairingSecret: string;
  fingerprint: string;
  certFingerprint: string;
  port: number;
  clientCount: number;
  live: boolean;
  device: ConnectedDevice | null;
  appName: string;
  pluginId: string;
  onboardingComplete: boolean;
  accessibilityTrusted: boolean;
  uninstallPrompt: boolean;
  bundled: boolean;
};
