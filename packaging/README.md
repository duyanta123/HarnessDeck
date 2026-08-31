# Packaging

Manifests for the five package managers HarnessDeck ships through. Every file
here is generated — including `bucket/harnessdeck.json` at the repository root —
so **edit [`generate.mjs`](generate.mjs), not the output**:

```sh
node packaging/generate.mjs          # newest release
node packaging/generate.mjs v0.1.0   # a particular one
```

The script reads the release from the GitHub API, takes each SHA-256 from the
release's own `SHA256SUMS.txt` when it has one and downloads and hashes the assets
when it does not, and rewrites every manifest below. Five registries want the same
three facts — version, URL, digest — in five different shapes, and a hand-edited
one is how a bucket ends up installing last month's build.

| Channel       | Manifest                                              | State                                       |
| ------------- | ----------------------------------------------------- | ------------------------------------------- |
| Scoop         | [`bucket/harnessdeck.json`](../bucket/harnessdeck.json) | live from this repository once the first release lands |
| winget        | `winget/`                                             | generated at release time, needs a pull request |
| Homebrew Cask | `homebrew/harnessdeck.rb`                             | generated at release time, needs its own tap repository |
| AUR           | `aur/`                                                | generated at release time, needs an upload to aur.archlinux.org |
| Flathub       | `flathub/`                                            | generated at release time; locally buildable |

Until the first tagged release exists, the generated manifests are absent on
purpose: there is no published artifact to point them at, and a manifest with a
placeholder digest is worse than none.

## Publishing

Only Scoop is served from here until the external registries receive their
one-time submissions. Every published release runs the generator, validates all
five manifests on their native toolchains and commits the validated files back
to `main`. After a channel has been registered, its maintainer can consume those
committed files without recalculating release URLs or digests by hand.

**Scoop** needs nothing. A bucket is a repository with a `bucket/` directory, so
this one already is:

```powershell
scoop bucket add harnessdeck https://github.com/duyanta123/HarnessDeck
scoop install harnessdeck
```

Scoop has no concept of running an installer, so the manifest drives the NSIS one
itself with `/S /NS /D=$dir`. Those are the flags Tauri's installer template
actually reads: `/S` and `/D` are NSIS built-ins, `/NS` suppresses the shortcuts
Scoop makes for itself. `/D=` must come last and must not be quoted — NSIS takes
the remainder of the command line as the path, which is also why a directory with
a space in it is safe.

**winget** wants a pull request to
[microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) under
`manifests/d/duyanta123/HarnessDeck/<version>/`. Check it first — this is the one
channel with a validator that runs locally:

```powershell
winget validate --manifest packaging\winget
```

It packages the NSIS installer rather than the MSI, for two reasons: it is a
megabyte smaller, and it installs per-user, so `winget install` needs no
elevation. The `Moniker` field is what makes the short form resolve:

```powershell
winget install harnessdeck
```

**Homebrew** requires a cask to live in a repository named `homebrew-<something>`,
so this one cannot be tapped from here. Create `duyanta123/homebrew-harnessdeck`
with the cask at `Casks/harnessdeck.rb`, then:

```sh
brew tap duyanta123/harnessdeck https://github.com/duyanta123/homebrew-harnessdeck
brew install --cask harnessdeck
brew audit --cask --online harnessdeck   # what the tap's CI will run
```

**AUR** needs a push to `ssh://aur@aur.archlinux.org/harnessdeck-bin.git` with the
`PKGBUILD` and `.SRCINFO` at the repository root. Regenerate `.SRCINFO` on a
machine with makepkg rather than trusting the transcription here — the AUR reads
that file instead of executing the PKGBUILD, so a stale one shows the wrong
version to everyone:

```sh
makepkg --printsrcinfo > .SRCINFO
namcap PKGBUILD
makepkg -si   # actually install it once before publishing
```

**Flathub** uses GNOME 49, which supplies the WebKitGTK 4.1 ABI used by Tauri,
and the AppStream metadata includes a real application screenshot. CI builds the
manifest. It deliberately requests `--filesystem=home`, because projects are the
application's input. The remaining product boundary is tool execution: a
Flatpak-installed Harness sees the sandbox's commands, not every compiler and
CLI on the host. Until a reviewed host-tool bridge exists, this channel must not
be advertised as equivalent to the native `.deb`/AppImage and should not be
submitted merely to increase a channel count.

## Mirrors

See [MIRRORS.md](MIRRORS.md). Short version: package managers verify the digest
themselves, which makes them the safest way to install from a slow network.
