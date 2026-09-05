cask "caddie" do
  version "0.1.0"
  sha256 "8182b3861f86f3f0e50d68e6f7ade9715b917022d52351cd299572e9a56a73b7"

  url "https://github.com/Oh-Damn/caddie/releases/download/v#{version}/Caddie_#{version}_universal.dmg",
      verified: "github.com/Oh-Damn/caddie/"
  name "Caddie"
  desc "Phone control surface for your Mac"
  homepage "https://github.com/Oh-Damn/caddie"

  livecheck do
    url :url
    strategy :github_latest
  end

  depends_on macos: :monterey

  app "Caddie.app"

  uninstall quit: "dev.caddie.desktop"

  zap trash: [
    "~/Library/Application Support/dev.caddie.desktop",
    "~/Library/Caches/dev.caddie.desktop",
    "~/Library/HTTPStorages/dev.caddie.desktop",
    "~/Library/Logs/dev.caddie.desktop",
    "~/Library/Preferences/dev.caddie.desktop.plist",
    "~/Library/Saved Application State/dev.caddie.desktop.savedState",
    "~/Library/WebKit/dev.caddie.desktop",
  ]

  caveats <<~EOS
    Caddie is not notarised by Apple, so macOS quarantines it and refuses
    to open it. Homebrew no longer has a --no-quarantine option, so clear
    the flag by hand:

      xattr -dr com.apple.quarantine /Applications/Caddie.app

    Caddie serves a web app to your phone over your local network. If the
    macOS firewall is on, allow the incoming connections when prompted. If
    no prompt appears and your phone cannot load the page:

      sudo /usr/libexec/ApplicationFirewall/socketfilterfw \\
        --unblockapp /Applications/Caddie.app/Contents/MacOS/companion-desktop

    Then quit Caddie from the menu bar and open it again.

    Caddie lives in the menu bar, not the Dock.
  EOS
end
