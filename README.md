<div align="center">

<img src="apps/pwa/public/oh-damn-logo.svg" alt="OhDamn!" width="220">

# Caddie

**A caddie for your Mac, on the phone you already have.**

Powered by [OhDamn!](https://www.oh-damn.com)

[![License: MIT](https://img.shields.io/badge/License-MIT-EF4A4C.svg)](LICENSE)
![Platform](https://img.shields.io/badge/platform-macOS%2012%2B-EF4A4C)

</div>

---

Caddie turns your phone into a control surface that follows what you are doing
on your Mac. Open Spotify and you get transport. Open a browser and you get its
tabs. Leave a coding agent waiting on a prompt and your phone tells you, from
the other side of the room.

The desktop app lives in the menu bar and serves a PWA over your own Wi-Fi.
Nothing goes to a server. There is no account.

## What it does

**Follows the front app.** Media controls for Spotify and Music, tab lists for
Chrome, Safari, Brave and Arc, a shortcut pad for everything else. No profile
switching.

**Watches your coding agents.** Claude Code, Cursor, Claude Desktop and Codex.
When one is waiting on you, the phone chimes and the agents chip lights up. Plan
usage is read from Claude's own local cache, so the percentages are the real
ones rather than an estimate, and no credential is involved.

**Stays on your network.** The Mac serves the PWA over plain HTTP on your own
Wi-Fi, so there is no certificate to accept and nothing to re-trust when your
router hands out a new address. Pairing is a QR code and a six character code.
Trusted devices are remembered.

**Reads only what you allow.** Every agent provider has a switch in the desktop
app, and off is a real stop: no file is opened, no Accessibility tree is walked,
no card appears.

## Requirements

- macOS 12 or later
- Node 22+ and pnpm
- Rust (stable), with `~/.cargo/bin` on PATH

## Run it

```bash
pnpm install
pnpm build:pwa
pnpm dev
```

First launch walks through Wi-Fi, Accessibility and Automation, then shows a QR
code. After that it stays in the menu bar; click the icon for the pairing
screen.

On your phone, join the same Wi-Fi and open the URL from that screen. Use the IP
fallback if the `.local` name does not resolve. The server listens on port
`7842`.

## Build a release

```bash
pnpm build:app
```

Produces a signed universal bundle and a `.dmg`. macOS ties permission grants to
an app's code signature, so an unsigned rebuild loses its Accessibility and
Automation grants. For grants that survive rebuilds, make an identity once:

```bash
sh scripts/signing-identity.sh
CADDIE_SIGNING_IDENTITY="Caddie Dev" pnpm build:app
```

After re-signing, clear the stale grants:

```bash
tccutil reset Accessibility dev.caddie.desktop
tccutil reset AppleEvents dev.caddie.desktop
```

## Permissions

| Permission | Needed for |
| --- | --- |
| Accessibility | Keyboard shortcuts, media keys, window titles |
| Automation | Frontmost app, Music, Spotify, browser tabs |
| Local Network | Letting the phone reach this Mac |

## Start over

```bash
pnpm purge
```

Removes build output and local state, so the next `pnpm dev` behaves like a
first run. Add `--deep` to drop the Rust `target` directory, `--force` to quit a
running app first, `--dry-run` to preview.

If the phone still shows a stale build, use **Clear cache and reload** in the
app's settings.

Because the app is served over HTTP rather than HTTPS, the browser will not give
it a secure context. In-app QR scanning needs the camera and is therefore
unavailable; scan the pairing QR with the phone's own Camera app, or type the
address and code by hand. Service workers are also unavailable, so the app does
not install as an offline PWA. It needs the Mac on the same network to do
anything, so there is nothing useful to run offline.

## Layout

```
apps/desktop      Tauri shell, Rust server, menu bar app
apps/pwa          The phone UI
packages/protocol Shared message types
packages/theme    Design tokens
```

## Licence

MIT. See [LICENSE](LICENSE).

<div align="center">
<br>
<img src="apps/pwa/public/oh-damn-logo.svg" alt="OhDamn!" width="120">
<br>
<sub><b>POWERED BY OhDamn!</b></sub>
</div>
