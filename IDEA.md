Caddie

Turn your phone into a caddie for your desktop.

Vision
Caddie is a desktop + mobile web application that transforms a phone into an intelligent control surface for a computer.
Instead of acting as a generic remote control, Caddie automatically adapts its interface based on the application currently being used on the desktop.
For example:
* Open Spotify → media controls appear.
* Open VS Code → coding shortcuts appear.
* Open PowerPoint → presentation controls appear.
* Open OBS → streaming controls appear.
The goal is to eliminate manual profile switching and configuration while providing a polished, low-latency experience.

Problem
Current solutions fall into two categories:
1. Hardware macro pads (Stream Deck, Loupedeck)
2. Generic remote control apps (Unified Remote, KDE Connect, Remote Mouse)
Both have limitations.

Hardware
* Expensive
* Requires dedicated hardware
* Primarily targeted at creators

Remote apps
* Generic interfaces
* Require significant configuration
* Often have outdated UI
* Not aware of the current desktop application
Users shouldn't have to build their own control panels.
The software should already know what they're doing.

Solution
Caddie automatically detects the active desktop application and presents an optimized interface on the user's phone.
Example:

Spotify
* Play/Pause
* Previous/Next
* Volume
* Queue
* Album artwork

VS Code
* Run
* Debug
* Git
* Search
* Terminal
* Custom extension actions

PowerPoint
* Next slide
* Previous slide
* Timer
* Speaker notes

Browser
* Back
* Forward
* Refresh
* Tabs
* Bookmark
* Media controls

Core Principles
* Zero configuration
* Fast pairing
* Low latency
* Beautiful mobile-first UI
* Cross-platform
* Extensible plugin system

Architecture
Desktop App
      │
      │ WebSocket
      │
 Local Network
      │
 Phone (PWA)
The desktop application acts as the server.
The phone connects through the local network after pairing.
No cloud infrastructure is required for local usage.



Technology

Desktop
* Tauri
* Rust
* WebSocket server
* mDNS / Bonjour
* Native OS integrations


Phone
* Progressive Web App (PWA)
* React
* Tailwind CSS
* Installable from the browser



Communication

Discovery
* mDNS / Bonjour

Pairing
* QR Code
* Public/private key exchange
* Trusted devices

Messaging
WebSockets with JSON messages.
Example:
{
  "type": "media",
  "action": "play_pause"
}



Desktop Features
* Active application detection
* Media session detection
* Keyboard shortcut execution
* Clipboard sync
* Mouse & touchpad
* Plugin host

Plugin System
Applications can expose custom controls to the phone.
Example:
{
  "screen": "vscode",
  "title": "VS Code",
  "widgets": [
    {
      "type": "button",
      "title": "Run",
      "action": "run"
    },
    {
      "type": "button",
      "title": "Debug",
      "action": "debug"
    }
  ]
}
The phone renders the interface dynamically.
No application update is required when new plugins are added.

Supported Platforms
Desktop
* Windows
* macOS
* Linux
Phone
* iOS (PWA)
* Android (PWA)

MVP


Desktop
* Device discovery
* QR pairing
* Media controls
* Volume control
* Keyboard shortcuts
* Active application detection

Phone
* Installable PWA
* Dynamic layouts
* Touch controls
* Haptic feedback (where supported)

Future Features
* VS Code extension
* OBS integration
* Browser extension
* Presentation mode
* Clipboard history
* Community layouts
* Plugin marketplace
* Automation rules

Competitive Advantage
Existing products focus on programmable buttons.
Caddie focuses on context.
Instead of asking users to create pages of shortcuts, the application automatically displays the controls they need based on what they're currently doing.

Target Users
* Developers
* Designers
* Streamers
* Video editors
* Presenters
* Power users
* Students

Long-Term Vision
Become the universal companion layer between desktop applications and mobile devices.
Every desktop application should be able to expose a rich, touch-friendly interface on a nearby phone without requiring users to manually configure layouts or purchase dedicated hardware.

Initial Roadmap
Phase 1
* Desktop application
* PWA
* QR pairing
* Media controls
* Keyboard shortcuts
Phase 2
* VS Code integration
* Browser controls
* Presentation mode
* Clipboard sync
Phase 3
* Public plugin SDK
* Community integrations
* Automation engine
* Cross-device synchronization

Guiding Principle
Your phone should automatically become the perfect companion for whatever you're doing on your computer.
