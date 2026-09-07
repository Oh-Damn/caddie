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

**Stays on your network.** The Mac serves the PWA over HTTPS with a certificate
it signs itself. Pairing is a QR code and a six character code. Trusted devices
are remembered.

**Reads only what you allow.** Every agent provider has a switch in the desktop
app, and off is a real stop: no file is opened, no Accessibility tree is walked,
no card appears.

## Install

Caddie is installed with Homebrew, a tool that downloads and installs Mac apps
from the Terminal. If you have never used the Terminal: hold Command and press
the space bar, type `terminal`, and press Return. A window opens where you paste
in a line and press Return to run it. Run the lines below one at a time and let
each one finish before pasting the next.

If you do not have Homebrew yet, install it first. It asks for your Mac
password, and nothing appears on screen while you type it, which is normal:

```sh
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
```

Then install Caddie:

```sh
brew tap Oh-Damn/caddie
brew trust --cask Oh-Damn/caddie/caddie
brew install --cask caddie
xattr -dr com.apple.quarantine /Applications/Caddie.app
```

| Line | What it does |
| --- | --- |
| `brew tap` | Points Homebrew at the place Caddie is published. |
| `brew trust` | Homebrew will not install from someone's personal tap until you tell it that this one is fine. |
| `brew install` | Downloads Caddie and puts it in your Applications folder. |
| `xattr -dr` | Removes the quarantine mark macOS puts on everything you download. Caddie is not notarised by Apple, so macOS refuses to open it while that mark is there. Homebrew used to clear the mark with `--no-quarantine`, but that option was removed in Homebrew 6. |

Miss the `brew trust` line and the install stops with an error. Miss the `xattr`
line and Caddie will not open: macOS says *"Caddie can't be opened"* or *"Caddie
is damaged"*. Neither one means the download is broken. Run the line you skipped
and try again.

Now open Caddie from your Applications folder. It runs in the **menu bar**
rather than the Dock, so look for its icon along the top right of the screen and
click it for the pairing QR code. The first launch walks you through the
permissions it needs.

### Pairing the phone

Scan the QR on the pairing screen with your phone's camera. It carries the
address and the pairing secret, so there is nothing to type.

If you type the address instead, **include `https://`**. The server speaks TLS
only, so a plain `192.168.1.x:7842` hangs with no useful error. Your phone will
then warn that the connection is not private. That is expected, because the Mac
signs its own certificate. Tap through it once.

Phone and Mac must be on the same Wi-Fi. Guest networks and many mesh systems
block devices from reaching each other, which looks identical to the app being
broken.

### If the phone cannot reach the Mac

Caddie is not signed with a Developer ID, so macOS does not automatically let it
accept incoming connections the way it does for notarised apps. If the firewall
is on and never prompted you:

```sh
sudo /usr/libexec/ApplicationFirewall/socketfilterfw \
  --unblockapp /Applications/Caddie.app/Contents/MacOS/companion-desktop
```

Then quit Caddie from the menu bar and open it again. A firewall decision binds
to the running process, so the rule does nothing until Caddie restarts.

To check the server itself, on the Mac:

```sh
curl -sk -o /dev/null -w "%{http_code}\n" https://$(ipconfig getifaddr en0):7842/api/health
```

`200` means the server is reachable on the network and the problem is between
the phone and the Mac. Anything else is the Mac.

### Installing the dmg directly

Grab it from [Releases](https://github.com/Oh-Damn/caddie/releases), drag Caddie
to Applications, then clear the quarantine flag as above. Building from source
avoids all of it, since quarantine comes from the download rather than the
compiler.

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

Produces a signed universal bundle and a `.dmg`. On an Apple Silicon Mac that
builds both architectures; if the bundle is only for this machine, build the
arm64 slice on its own and skip half the compile:

```bash
pnpm build:arm
```

macOS ties permission grants to an app's code signature, so an unsigned rebuild
loses its Accessibility and Automation grants. For grants that survive rebuilds,
make an identity once:

```bash
sh scripts/signing-identity.sh
CADDIE_SIGNING_IDENTITY="Caddie Dev" pnpm build:app
```

After re-signing, clear the stale grants:

```bash
tccutil reset Accessibility dev.caddie.desktop
tccutil reset AppleEvents dev.caddie.desktop
```

## Updates

Caddie checks GitHub on launch and offers the update on the pairing screen.
Downloading, verifying and installing it happens in the app; it restarts itself
when you say so.

Update archives are signed with a key that is not the code signing certificate.
The public half lives in `tauri.conf.json` and the app refuses any archive whose
signature does not match, so a tampered or unsigned download is rejected rather
than installed.

After an update macOS may ask for Accessibility and Automation again. That
happens when the code signature changes identity, so releases are signed with
one stable certificate to avoid it. See `scripts/ci-identity.sh`. A release
built without those secrets falls back to ad-hoc signing and will reset grants.

## Permissions

| Permission | Needed for |
| --- | --- |
| Accessibility | Keyboard shortcuts, media keys, window titles |
| Automation | Frontmost app, Music, Spotify, browser tabs |
| Local Network | Letting the phone reach this Mac |

### Browsers

Play and pause on a browser tab is driven by AppleScript, and Chrome-family
browsers refuse that by default. Turn on **View > Developer > Allow JavaScript
from Apple Events**, then **quit and reopen the browser**. The setting
does nothing until it restarts, which is the part people miss. Chrome, Brave and Arc
all need it; Safari does not.

Tab lists work without it. Only media control on a tab is affected, and Caddie
says so in the error when you hit it.

## Start over

```bash
pnpm purge
```

Removes build output and local state, so the next `pnpm dev` behaves like a
first run. Add `--deep` to drop the Rust `target` directory, `--force` to quit a
running app first, `--dry-run` to preview.

If the phone still shows a stale build, use **Clear cache and reload** in the
app's settings.

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
<sub><b>POWERED BY OHDAMN!</b></sub>
</div>
