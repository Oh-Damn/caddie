# Caddie tap

This directory is the contents of a **separate** repository. Homebrew only
recognises a tap whose repo name begins with `homebrew-`, so it cannot live
inside the main project.

## Publishing it, once

Create an empty public repo named `homebrew-caddie` under the same owner as
the main project, then:

```sh
cd packaging/homebrew-tap
git init
git add .
git commit -m "Caddie cask"
git remote add origin git@github.com:Oh-Damn/homebrew-caddie.git
git push -u origin main
```

After that people install with two lines:

```sh
brew tap Oh-Damn/caddie
brew install --cask --no-quarantine caddie
```

`brew tap Oh-Damn/caddie` resolves to the `homebrew-caddie` repo — the prefix is
implied and must not be typed.

## Cutting a release

1. Tag the main repo: `git tag v0.1.0 && git push origin v0.1.0`. The
   `release.yml` workflow builds the universal dmg and attaches it to a
   GitHub Release.
2. In this tap checkout, point the cask at it:

   ```sh
   sh update-cask.sh 0.1.0 Oh-Damn/caddie
   git commit -am "caddie 0.1.0" && git push
   ```

`update-cask.sh` downloads the published dmg, computes its sha256, and rewrites
the `version` and `sha256` fields. Never hand-edit the checksum: Homebrew
refuses the download if it does not match, and a stale one breaks every install.

## Why --no-quarantine

The build is signed locally rather than notarised by Apple, which needs a paid
Developer ID. Homebrew quarantines downloads by default, and macOS refuses to
open a quarantined app with no valid Developer ID — the "can't be opened" and
"damaged" dialogs. The flag skips the quarantine flag and the app opens
normally.

If Caddie ever gets a Developer ID, drop the flag from the instructions and
delete the first paragraph of the cask's `caveats`.
