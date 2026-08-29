import { useEffect, useMemo, useState } from 'react';
import { ConversationStrip } from '../components/apps/ConversationStrip';
import { ContextCard } from '../components/apps/ContextCard';
import { BrowserPanel } from '../components/apps/BrowserPanel';
import { CallStrip } from '../components/home/CallStrip';
import { HomeHeader } from '../components/home/HomeHeader';
import {
  HomeSkeleton,
  ShortcutGridSkeleton,
  TabListSkeleton,
} from '../components/home/HomeSkeleton';
import { ShortcutGrid } from '../components/home/ShortcutGrid';
import { MediaPanel } from '../components/media/MediaPanel';
import { MediaVolumeBar } from '../components/media/MediaVolumeBar';
import {
  browserNowPlaying,
  mediaSources,
  nativeNowPlaying,
} from '../companion/nowPlaying';
import {
  loadPinnedConversations,
  savePinnedConversations,
} from '../companion/pinnedConversations';
import { recentRunningApps } from '../companion/recentApps';
import { packTabRef, packTabTarget, soloActiveTabs } from '../companion/tabs';
import { useCommand } from '../companion/useCommand';
import { useCompanion } from '../companion/useCompanion';
import type { Conversation, NowPlaying } from '@companion/protocol';
import { genericLayout } from '../lib/genericLayout';
import { formatLocation, parseTabTitle, parseWindowTitle } from '../lib/tabTitle';
import { HomeLayout } from '../layouts/HomeLayout';

const BROWSER_BUNDLES = new Set([
  'com.google.Chrome',
  'com.google.Chrome.canary',
  'com.google.Chrome.beta',
  'com.google.Chrome.dev',
  'com.brave.Browser',
  'com.apple.Safari',
  'company.thebrowser.Browser',
]);

export function HomeScreen() {
  const { appState, layout, apps, recentBundles, pendingFocusBundle, focusApp, status } =
    useCompanion();
  const go = useCommand();
  const plugin = appState?.pluginId || layout?.screen;
  const nativeNp = nativeNowPlaying(appState);
  const browserNp = browserNowPlaying(appState);
  const goNative = (action: string, value: number | null = null) => {
    go(action, value, nativeNp?.sourceBundleId || null);
  };

  const sources = mediaSources(nativeNp, browserNp);
  const hasMedia = sources.length > 0;
  const goSource = (action: string, value: number | null, target: string) => {
    go(action, value, target || null);
  };
  const appName = appState?.appName || 'Desktop';
  const appBundle = appState?.bundleId || '';
  const isBrowser = plugin === 'browser' || BROWSER_BUNDLES.has(appBundle);
  const isMedia = plugin === 'media' && !isBrowser;
  const switching = pendingFocusBundle !== null;
  const recentApps = recentRunningApps(apps, appBundle, recentBundles);
  const tabs = appState?.tabs ?? [];
  const browserTabs = useMemo(
    () => soloActiveTabs(appState?.tabs ?? [], appState?.windowTitle ?? ''),
    [appState?.tabs, appState?.windowTitle],
  );

  const awaitingTabs = isBrowser && tabs.length === 0;
  const isChat = plugin === 'slack' || plugin === 'discord' || plugin === 'teams';
  const [heardBundle, setHeardBundle] = useState('');
  useEffect(() => {
    if (!appState?.bundleId) return;
    savePinnedConversations(appState.bundleId, appState.pinnedConversations ?? []);
    setHeardBundle(appState.bundleId);
  }, [appState?.bundleId, appState?.pinnedConversations]);
  const pinned =
    heardBundle === appBundle
      ? (appState?.pinnedConversations ?? [])
      : loadPinnedConversations(appBundle);
  const chatHere =
    isChat &&
    appState?.currentConversation &&
    appState.currentConversation.kind !== 'other'
      ? appState.currentConversation
      : null;
  const location = contextLocation({
    isBrowser,
    isChat,
    isMedia,
    appName,
    windowTitle: appState?.windowTitle ?? '',
    chatHere,
    nativeNp,
    browserTabs,
  });
  const recents = appState?.recentConversations ?? [];
  const hasConversations = Boolean(isChat || recents.length > 0 || pinned.length > 0);

  const openConversation = (item: Conversation) => {
    if (item.windowIndex != null) {
      focusApp(appBundle, item.windowIndex);
      return;
    }
    go('conversation.jump', null, item.name);
  };

  const togglePin = (item: Conversation) => {
    go(
      isPinnedName(pinned, item) ? 'conversation.unpin' : 'conversation.pin',
      null,
      item.name,
    );
  };

  const addPerson = (name: string) => {
    go('conversation.pin', null, name);
    go('conversation.jump', null, name);
  };

  const activateTab = (tab: (typeof browserTabs)[number]) => {
    go(
      'browser.activate_tab',
      packTabRef(tab.windowIndex, tab.tabIndex),
      packTabTarget(appBundle, tab),
    );
  };

  const sendHistory = (dir: 'back' | 'forward') => {
    go(
      dir === 'back'
        ? 'shortcut.send:meta+leftbracket'
        : 'shortcut.send:meta+rightbracket',
    );
  };

  if (!layout && !appState) {
    return <HomeSkeleton />;
  }

  const gridLayout =
    layout && layout.screen !== 'media' && layout.screen !== 'browser'
      ? layout
      : genericLayout(appName);

  return (
    <HomeLayout
      header={
        <HomeHeader
          extra={
            hasMedia ? undefined : (
              <MediaVolumeBar
                compact
                sources={sources}
                volume={appState?.volume ?? 0}
                muted={appState?.muted ?? false}
                onCommand={goNative}
                onMediaCommand={goSource}
                onFocusApp={focusApp}
              />
            )
          }
        />
      }
      split={hasMedia}
      dock={
        <MediaVolumeBar
          sources={sources}
          volume={appState?.volume ?? 0}
          muted={appState?.muted ?? false}
          showMediaControls={!(isMedia && nativeNp)}
          hideReadoutFor={
            isMedia && nativeNp ? nativeNp.sourceBundleId || appBundle : undefined
          }
          onCommand={goNative}
          onMediaCommand={goSource}
          onFocusApp={focusApp}
        />
      }
    >
      <ContextCard
        appName={appName}
        bundleId={appBundle}
        windowTitle={location}
        recentApps={recentApps}
        unread={isChat ? (appState?.unread ?? null) : null}
        pinned={chatHere ? isPinnedName(pinned, chatHere) : false}
        showHistory={isChat || isBrowser}
        onFocus={() => {
          if (appBundle) focusApp(appBundle);
        }}
        onFocusApp={(id) => focusApp(id)}
        onHistory={isChat || isBrowser ? sendHistory : undefined}
        onTogglePin={chatHere ? () => togglePin(chatHere) : undefined}
      />
      {hasConversations ? (
        <ConversationStrip
          recent={recents}
          pinned={pinned}
          onOpen={openConversation}
          onTogglePin={togglePin}
          onAdd={isChat ? addPerson : undefined}
        />
      ) : null}
      {appState?.call?.active ? <CallStrip call={appState.call} onCommand={go} /> : null}
      {switching || awaitingTabs ? (
        isBrowser ? (
          <TabListSkeleton />
        ) : (
          <ShortcutGridSkeleton />
        )
      ) : isBrowser ? (
        <BrowserPanel
          tabs={tabs}
          windowTitle={appState?.windowTitle ?? ''}
          nowPlaying={browserNp}
          onActivate={activateTab}
          onToggleMedia={(tab) =>
            go(
              'browser.media_toggle',
              packTabRef(tab.windowIndex, tab.tabIndex),
              packTabTarget(appBundle, tab),
            )
          }
        />
      ) : isMedia && nativeNp ? (
        <MediaPanel
          connected={status === 'ready'}
          nowPlaying={nativeNp}
          sourceName={nativeNp.sourceName || appName}
          sourceBundleId={nativeNp.sourceBundleId || appBundle}
          onFocus={() => {
            const target = nativeNp.sourceBundleId || appBundle;
            if (target) focusApp(target);
          }}
          onCommand={(action) => go(action, null, nativeNp.sourceBundleId)}
        />
      ) : (
        <ShortcutGrid layout={gridLayout} onCommand={go} />
      )}
    </HomeLayout>
  );
}

function isPinnedName(pinned: Conversation[], item: Conversation): boolean {
  return pinned.some(
    (p) => p.kind === item.kind && p.name.toLowerCase() === item.name.toLowerCase(),
  );
}

function contextLocation({
  isBrowser,
  isChat,
  isMedia,
  appName,
  windowTitle,
  chatHere,
  nativeNp,
  browserTabs,
}: {
  isBrowser: boolean;
  isChat: boolean;
  isMedia: boolean;
  appName: string;
  windowTitle: string;
  chatHere: Conversation | null;
  nativeNp: NowPlaying | null;
  browserTabs: ReturnType<typeof soloActiveTabs>;
}): string {
  if (isBrowser) {
    const active = browserTabs.find((t) => t.active);
    if (active) {
      const { page, site } = parseTabTitle(active.title);
      return formatLocation(page, site);
    }
    return parseWindowTitle(windowTitle, appName);
  }
  if (chatHere) {
    return [chatHere.name, chatHere.workspace].filter(Boolean).join(' \u00b7 ');
  }
  if (isChat) return '';
  if (isMedia && nativeNp) {
    return [nativeNp.title, nativeNp.artist].filter(Boolean).join(' \u00b7 ');
  }
  return parseWindowTitle(windowTitle, appName);
}
