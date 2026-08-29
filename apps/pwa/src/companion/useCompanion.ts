import {
  PRODUCT_NAME,
  type AgentsPayload,
  type ClipboardItem,
  type LayoutPayload,
  type RunningApp,
  type ServerMessage,
  type StatePayload,
} from '@companion/protocol';
import {
  createContext,
  createElement,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react';
import { currentRoute, navigate, ROUTES } from '../routes';
import { usePreferencesRef } from '../lib/usePreferences';
import { useRoute } from '../useRoute';
import { CommandGate, outboundArgs, type CommandArgs } from './commandGate';
import { normalizeNowPlaying } from './nowPlaying';
import { OptimisticHold } from './optimistic';
import { envelope, parseServer } from './protocol';
import { pushRecentBundle } from './recentApps';
import { soloActiveTabs } from './tabs';
import {
  clearSession,
  defaultHost,
  deviceDetails,
  deviceName,
  haptic,
  loadSession,
  normalizeHost,
  parsePairingUrl,
  saveSession,
  wsUrlFromHttp,
  type SavedSession,
} from './session';

export type Status = 'idle' | 'connecting' | 'pairing' | 'ready' | 'error';

const PING_WATCH_MS = 25000;
const QUEUE_CAP = 8;
const CLIP_HOLD_MS = 2500;

function trySend(ws: WebSocket | null, msg: ReturnType<typeof envelope>): boolean {
  if (!ws || ws.readyState !== WebSocket.OPEN) return false;
  ws.send(JSON.stringify(msg));
  return true;
}

export type CompanionValue = {
  status: Status;
  error: string | null;
  setError: (value: string | null) => void;
  host: string;
  setHost: (value: string) => void;
  secret: string;
  setSecret: (value: string) => void;
  layout: LayoutPayload | null;
  appState: StatePayload | null;
  pendingFocusBundle: string | null;
  apps: RunningApp[] | null;
  recentBundles: string[];
  clipboardItems: ClipboardItem[] | null;
  agents: AgentsPayload | null;
  deviceId: string;
  connectManual: () => void;
  connectFromPairingUrl: (url: string) => void;
  command: (action: string, value?: number | null, target?: string | null) => void;
  focusApp: (bundleId: string, windowIndex?: number | null) => void;
  queryApps: () => void;
  queryClipboard: () => void;
  queryAgents: () => void;
  dropClipboard: (id: string) => void;
  clearClipboard: () => void;
  unpair: () => void;
};

const CompanionContext = createContext<CompanionValue | null>(null);

export function useCompanion(): CompanionValue {
  const value = useContext(CompanionContext);
  if (!value) {
    throw new Error('useCompanion requires CompanionProvider');
  }
  return value;
}

function hasSavedDevice(session: SavedSession | null): boolean {
  return Boolean(session?.host && session.deviceId);
}

function useCompanionState(): CompanionValue {
  const wsRef = useRef<WebSocket | null>(null);
  const genRef = useRef(0);
  const savedRef = useRef<SavedSession | null>(loadSession());
  const [status, setStatus] = useState<Status>(() =>
    hasSavedDevice(savedRef.current) ? 'connecting' : 'idle',
  );
  const [error, setError] = useState<string | null>(null);
  const [host, setHost] = useState(defaultHost);
  const [secret, setSecret] = useState('');
  const [layout, setLayout] = useState<LayoutPayload | null>(null);
  const [appState, setAppState] = useState<StatePayload | null>(null);
  const [apps, setApps] = useState<RunningApp[] | null>(null);
  const [clipboardItems, setClipboardItems] = useState<ClipboardItem[] | null>(null);
  const [agents, setAgents] = useState<AgentsPayload | null>(null);
  const appsRef = useRef<RunningApp[] | null>(null);
  appsRef.current = apps;
  const [deviceId, setDeviceId] = useState(() => savedRef.current?.deviceId ?? '');
  const retryRef = useRef(0);
  const retryTimerRef = useRef<number | null>(null);
  const stopRef = useRef(false);
  const statusRef = useRef<Status>(
    hasSavedDevice(savedRef.current) ? 'connecting' : 'idle',
  );
  const optimisticRef = useRef(new OptimisticHold());
  const commandGateRef = useRef(new CommandGate());
  const prefsRef = usePreferencesRef();
  const appStateRef = useRef<StatePayload | null>(null);
  appStateRef.current = appState;
  const pendingFocusRef = useRef<string | null>(null);
  const focusTimeoutRef = useRef<number | null>(null);
  const [pendingFocusBundle, setPendingFocusBundle] = useState<string | null>(null);
  const [recentBundles, setRecentBundles] = useState<string[]>([]);
  const prevBundleRef = useRef<string | null>(null);
  const lastRxRef = useRef(Date.now());
  const outboundQueueRef = useRef<CommandArgs[]>([]);
  const droppedClipboardRef = useRef(new Map<string, number>());
  const clipboardClearedUntilRef = useRef(0);
  const connectRef = useRef<
    (
      httpBase: string,
      pairingSecret: string | null,
      existing?: SavedSession | null,
    ) => void
  >(() => undefined);

  statusRef.current = status;

  const clearRetry = useCallback(() => {
    if (retryTimerRef.current !== null) {
      window.clearTimeout(retryTimerRef.current);
      retryTimerRef.current = null;
    }
  }, []);

  const socketIsHealthy = useCallback(() => {
    const ws = wsRef.current;
    if (!ws) return false;
    if (ws.readyState === WebSocket.CONNECTING) return true;
    return (
      ws.readyState === WebSocket.OPEN && Date.now() - lastRxRef.current <= PING_WATCH_MS
    );
  }, []);

  const clearPendingFocus = useCallback(() => {
    pendingFocusRef.current = null;
    setPendingFocusBundle(null);
    if (focusTimeoutRef.current !== null) {
      window.clearTimeout(focusTimeoutRef.current);
      focusTimeoutRef.current = null;
    }
  }, []);

  const markPendingFocus = useCallback(
    (bundleId: string) => {
      pendingFocusRef.current = bundleId;
      setPendingFocusBundle(bundleId);
      if (focusTimeoutRef.current !== null) {
        window.clearTimeout(focusTimeoutRef.current);
      }
      focusTimeoutRef.current = window.setTimeout(() => {
        if (pendingFocusRef.current === bundleId) {
          clearPendingFocus();
        }
      }, 3000);
    },
    [clearPendingFocus],
  );

  const persistSession = useCallback((deviceId: string) => {
    const prev = savedRef.current;
    const next: SavedSession = {
      host: prev?.host ?? window.location.host,
      deviceId,
      deviceName: prev?.deviceName ?? deviceName(),
    };
    savedRef.current = next;
    saveSession(next);
    setDeviceId(deviceId);
  }, []);

  const enqueue = useCallback((args: CommandArgs) => {
    const queue = outboundQueueRef.current;
    const next =
      args.action === 'app.focus'
        ? queue.filter((item) => item.action !== 'app.focus')
        : queue.slice();
    next.push(args);
    outboundQueueRef.current = next.slice(-QUEUE_CAP);
  }, []);

  const flushQueue = useCallback(() => {
    const pending = outboundQueueRef.current;
    outboundQueueRef.current = [];
    for (const args of pending) {
      if (!trySend(wsRef.current, envelope('command', args))) {
        enqueue(args);
      }
    }
  }, [enqueue]);

  const applyServer = useCallback(
    (msg: ServerMessage) => {
      lastRxRef.current = Date.now();
      switch (msg.type) {
        case 'hello_ok':
          if (msg.payload.trusted && msg.payload.deviceId) {
            persistSession(msg.payload.deviceId);
            setStatus('ready');
            if (currentRoute() === ROUTES.connect) {
              navigate(ROUTES.home, true);
            }
            trySend(
              wsRef.current,
              envelope('presence', {
                visible: document.visibilityState === 'visible',
              }),
            );
            flushQueue();
          } else {
            setStatus('pairing');
          }
          break;
        case 'pair_ok':
          persistSession(msg.payload.deviceId);
          setStatus('ready');
          haptic('success');
          navigate(ROUTES.home, true);
          trySend(
            wsRef.current,
            envelope('presence', {
              visible: document.visibilityState === 'visible',
            }),
          );
          flushQueue();
          break;
        case 'pair_deny':
          setStatus('error');
          setError(msg.payload.reason);
          navigate(ROUTES.connect, true);
          break;
        case 'layout':
          setLayout(msg.payload);
          break;
        case 'state': {
          if (
            pendingFocusRef.current &&
            msg.payload.bundleId === pendingFocusRef.current
          ) {
            clearPendingFocus();
          }
          const next = prefsRef.current.optimisticUpdates
            ? optimisticRef.current.merge(msg.payload)
            : {
                ...msg.payload,
                nowPlaying: normalizeNowPlaying(msg.payload.nowPlaying),
                browserNowPlaying: normalizeNowPlaying(msg.payload.browserNowPlaying),
                tabs: soloActiveTabs(msg.payload.tabs ?? [], msg.payload.windowTitle),
                call: msg.payload.call ?? null,
              };
          setAppState(next);
          break;
        }
        case 'ping':
          break;
        case 'apps':
          setApps(msg.payload.items);
          break;
        case 'clipboard': {
          const now = Date.now();
          if (now < clipboardClearedUntilRef.current) {
            setClipboardItems([]);
            break;
          }
          setClipboardItems(
            msg.payload.items.filter((item) => {
              const until = droppedClipboardRef.current.get(item.id);
              if (until && now < until) return false;
              droppedClipboardRef.current.delete(item.id);
              return true;
            }),
          );
          break;
        }
        case 'agents':
          setAgents(msg.payload);
          break;
        case 'error':
          if (msg.payload.code === 'replaced') {
            stopRef.current = true;
            setStatus('error');
            setError(`${PRODUCT_NAME} moved to another device. Connect to take it back.`);
            break;
          }
          setError(msg.payload.message);
          break;
        default:
          break;
      }
    },
    [clearPendingFocus, flushQueue, persistSession, prefsRef],
  );

  const connect = useCallback(
    (httpBase: string, pairingSecret: string | null, existing?: SavedSession | null) => {
      if (socketIsHealthy()) return;
      const gen = ++genRef.current;
      stopRef.current = false;
      clearRetry();
      optimisticRef.current.clear();
      setError(null);
      setStatus('connecting');
      wsRef.current?.close();
      const url = wsUrlFromHttp(httpBase);
      let ws: WebSocket;
      try {
        ws = new WebSocket(url);
      } catch (e) {
        setStatus('error');
        setError(e instanceof Error ? e.message : 'WebSocket failed');
        return;
      }
      wsRef.current = ws;
      const name = existing?.deviceName ?? deviceName();
      const hostOnly = new URL(httpBase, window.location.origin).host;
      savedRef.current = {
        host: hostOnly,
        deviceId: existing?.deviceId ?? '',
        deviceName: name,
      };
      const autoPair = Boolean(pairingSecret) && !existing?.deviceId;

      ws.onopen = () => {
        if (gen !== genRef.current) return;
        retryRef.current = 0;
        lastRxRef.current = Date.now();

        void deviceDetails().then((details) => {
          if (gen !== genRef.current) return;
          try {
            trySend(
              ws,
              envelope('hello', {
                deviceName: name,
                deviceId: existing?.deviceId || null,
                details,
              }),
            );
          } catch (e) {
            setStatus('error');
            setError(e instanceof Error ? e.message : 'hello failed');
          }
        });
      };

      ws.onmessage = (ev) => {
        if (gen !== genRef.current) return;
        const msg = parseServer(String(ev.data));
        if (!msg) return;
        applyServer(msg);
        if (
          msg.type === 'hello_ok' &&
          !msg.payload.trusted &&
          autoPair &&
          pairingSecret
        ) {
          void deviceDetails().then((details) => {
            if (gen !== genRef.current) return;
            trySend(
              ws,
              envelope('pair', { secret: pairingSecret, deviceName: name, details }),
            );
          });
        }
        if (msg.type === 'pair_deny') {
          stopRef.current = true;
        }
      };

      ws.onclose = () => {
        if (gen !== genRef.current || stopRef.current) return;
        const s = savedRef.current;
        if (!s?.deviceId) {
          setStatus('idle');
          navigate(ROUTES.connect, true);
          return;
        }
        setStatus('connecting');
        clearRetry();
        const delay = Math.min(8000, 400 * 2 ** retryRef.current);
        retryRef.current += 1;
        retryTimerRef.current = window.setTimeout(() => {
          retryTimerRef.current = null;
          if (gen !== genRef.current) return;
          const latest = savedRef.current;
          if (!latest) return;
          connectRef.current(`https://${latest.host}`, null, latest);
        }, delay);
      };

      ws.onerror = () => {
        if (gen !== genRef.current) return;
        if (!savedRef.current?.deviceId) {
          setError(`Connection failed (${url})`);
        }
      };
    },
    [applyServer, clearRetry, socketIsHealthy],
  );

  connectRef.current = connect;

  useEffect(() => {
    const params = new URLSearchParams(window.location.search);
    const s = params.get('s');
    if (s) {
      setSecret(s);
      setHost(window.location.host);
      const saved = loadSession();
      connectRef.current(window.location.origin, saved?.deviceId ? null : s, saved);
    } else {
      const saved = loadSession();
      if (saved?.host && saved.deviceId) {
        setHost(saved.host);
        connectRef.current(`https://${saved.host}`, null, saved);
      } else if (currentRoute() !== ROUTES.connect) {
        navigate(ROUTES.connect, true);
      }
    }
    return () => {
      genRef.current += 1;
      clearRetry();
      wsRef.current?.close();
    };
  }, [clearRetry]);

  const sendPresence = useCallback((visible: boolean) => {
    trySend(wsRef.current, envelope('presence', { visible }));
  }, []);

  useEffect(() => {
    const onChange = () => sendPresence(document.visibilityState === 'visible');
    const onHide = () => sendPresence(false);
    document.addEventListener('visibilitychange', onChange);
    window.addEventListener('pagehide', onHide);
    return () => {
      document.removeEventListener('visibilitychange', onChange);
      window.removeEventListener('pagehide', onHide);
    };
  }, [sendPresence]);

  useEffect(() => {
    const resume = () => {
      if (document.visibilityState === 'hidden') return;
      const saved = savedRef.current ?? loadSession();
      if (!saved?.host || !saved.deviceId) return;
      if (socketIsHealthy()) return;
      connectRef.current(`https://${saved.host}`, null, saved);
    };
    const onVisible = () => {
      if (document.visibilityState === 'visible') resume();
    };
    document.addEventListener('visibilitychange', onVisible);
    window.addEventListener('pageshow', resume);
    window.addEventListener('online', resume);
    return () => {
      document.removeEventListener('visibilitychange', onVisible);
      window.removeEventListener('pageshow', resume);
      window.removeEventListener('online', resume);
    };
  }, [socketIsHealthy]);

  useEffect(() => {
    const id = window.setInterval(() => {
      if (stopRef.current) return;
      if (statusRef.current === 'pairing' || statusRef.current === 'idle') {
        return;
      }
      const ws = wsRef.current;
      if (!ws || ws.readyState !== WebSocket.OPEN) return;
      if (Date.now() - lastRxRef.current > PING_WATCH_MS) {
        ws.close();
      }
    }, 5000);
    return () => window.clearInterval(id);
  }, []);

  useEffect(() => {
    const saved = savedRef.current ?? loadSession();
    if (status === 'error' || (status === 'idle' && !hasSavedDevice(saved))) {
      if (currentRoute() !== ROUTES.connect) {
        navigate(ROUTES.connect, true);
      }
    } else if (status === 'ready' && currentRoute() === ROUTES.connect) {
      navigate(ROUTES.home, true);
    }
  }, [status]);

  const connectManual = useCallback(() => {
    const resolved = normalizeHost(host);
    setHost(resolved);
    const saved = loadSession();
    connect(`https://${resolved}`, saved?.deviceId ? null : secret.trim() || null, saved);
  }, [connect, host, secret]);

  const connectFromPairingUrl = useCallback(
    (url: string) => {
      const parsed = parsePairingUrl(url);
      if (!parsed) {
        setError('Not a Caddie pairing code');
        return;
      }
      setHost(parsed.host);
      setSecret(parsed.secret);
      const saved = loadSession();
      connect(parsed.origin, saved?.deviceId ? null : parsed.secret, saved);
    },
    [connect],
  );

  const command = useCallback(
    (action: string, value: number | null = null, target: string | null = null) => {
      if (commandGateRef.current.shouldBlockLeading(action)) return;

      const prev = appStateRef.current;
      if (action === 'app.focus') {
        const bundle = target?.trim() || prev?.bundleId || '';
        if (bundle && bundle !== prev?.bundleId) {
          markPendingFocus(bundle);
        }
      } else if (!prev) {
        const outbound = outboundArgs(action, value, target, 0);
        commandGateRef.current.dispatch(outbound, (args) => {
          if (!trySend(wsRef.current, envelope('command', args))) {
            enqueue(args);
          }
          commandGateRef.current.markLeading(action);
        });
        return;
      }

      const volume = prev?.volume ?? 0;
      const outbound = outboundArgs(action, value, target, volume);
      commandGateRef.current.dispatch(outbound, (args) => {
        const sent = trySend(wsRef.current, envelope('command', args));
        commandGateRef.current.markLeading(action);
        if (!sent) {
          enqueue(args);
          return;
        }
        if (prefsRef.current.optimisticUpdates && prev) {
          setAppState(
            optimisticRef.current.patchForCommand(
              prev,
              action,
              value,
              target,
              appsRef.current,
            ),
          );
        }
      });
    },
    [enqueue, markPendingFocus, prefsRef],
  );

  const queryApps = useCallback(() => {
    trySend(wsRef.current, envelope('query', { kind: 'apps' }));
  }, []);

  const queryAgents = useCallback(() => {
    trySend(wsRef.current, envelope('query', { kind: 'agents' }));
  }, []);

  const queryClipboard = useCallback(() => {
    trySend(wsRef.current, envelope('query', { kind: 'clipboard' }));
  }, []);

  const dropClipboard = useCallback(
    (id: string) => {
      droppedClipboardRef.current.set(id, Date.now() + CLIP_HOLD_MS);
      setClipboardItems((list) => list?.filter((item) => item.id !== id) ?? []);
      command('clipboard.remove', null, id);
      queryClipboard();
    },
    [command, queryClipboard],
  );

  const clearClipboard = useCallback(() => {
    clipboardClearedUntilRef.current = Date.now() + CLIP_HOLD_MS;
    droppedClipboardRef.current.clear();
    setClipboardItems([]);
    command('clipboard.clear');
    queryClipboard();
  }, [command, queryClipboard]);

  const { route } = useRoute();
  useEffect(() => {
    if (status !== 'ready') return;
    if (route === ROUTES.home || route === ROUTES.apps) {
      queryApps();
    }
  }, [queryApps, route, status]);

  useEffect(() => {
    if (status !== 'ready') return;
    queryClipboard();
  }, [queryClipboard, status]);

  useEffect(() => {
    if (status !== 'ready' || route !== ROUTES.clipboard) return;
    const id = window.setInterval(queryClipboard, 2000);
    return () => window.clearInterval(id);
  }, [queryClipboard, route, status]);

  useEffect(() => {
    if (status !== 'ready' || route !== ROUTES.ai) return;
    queryAgents();
    const id = window.setInterval(queryAgents, 5000);
    return () => window.clearInterval(id);
  }, [queryAgents, route, status]);

  useEffect(() => {
    const bundle = appState?.bundleId;
    if (!bundle) return;
    const prev = prevBundleRef.current;
    if (prev && prev !== bundle) {
      setRecentBundles((recent) => pushRecentBundle(recent, prev));
    }
    prevBundleRef.current = bundle;
  }, [appState?.bundleId]);

  const focusApp = useCallback(
    (bundleId: string, windowIndex?: number | null) => {
      command('app.focus', windowIndex ?? null, bundleId);
      navigate(ROUTES.home);
    },
    [command],
  );

  const unpair = useCallback(() => {
    stopRef.current = true;
    genRef.current += 1;
    clearRetry();
    wsRef.current?.close();
    clearSession();
    savedRef.current = null;
    optimisticRef.current.clear();
    commandGateRef.current.clear();
    outboundQueueRef.current = [];
    droppedClipboardRef.current.clear();
    clipboardClearedUntilRef.current = 0;
    clearPendingFocus();
    setRecentBundles([]);
    prevBundleRef.current = null;
    setDeviceId('');
    setLayout(null);
    setAppState(null);
    setApps(null);
    setClipboardItems(null);
    setStatus('idle');
    navigate(ROUTES.connect, true);
  }, [clearPendingFocus, clearRetry]);

  return useMemo(
    () => ({
      status,
      error,
      setError,
      host,
      setHost,
      secret,
      setSecret,
      layout,
      appState,
      pendingFocusBundle,
      apps,
      recentBundles,
      clipboardItems,
      agents,
      deviceId,
      connectManual,
      connectFromPairingUrl,
      command,
      focusApp,
      queryApps,
      queryClipboard,
      queryAgents,
      dropClipboard,
      clearClipboard,
      unpair,
    }),
    [
      agents,
      appState,
      apps,
      clipboardItems,
      command,
      connectFromPairingUrl,
      connectManual,
      deviceId,
      error,
      focusApp,
      host,
      layout,
      pendingFocusBundle,
      queryApps,
      queryClipboard,
      queryAgents,
      dropClipboard,
      clearClipboard,
      recentBundles,
      secret,
      status,
      unpair,
    ],
  );
}

export function CompanionProvider({ children }: { children: ReactNode }) {
  const value = useCompanionState();
  return createElement(CompanionContext.Provider, { value }, children);
}
