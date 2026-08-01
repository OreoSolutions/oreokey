# OreoKey Ubuntu Packaging and Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Produce verifiable Ubuntu .deb artifacts and release automation for the completed IBus, Fcitx5, and GTK configuration deliverables.

**Architecture:** Debian package definitions build already-tested components separately. GitHub Actions builds each Ubuntu series/architecture, runs unit and package checks, emits checksums, and publishes only verified artifacts.

**Tech Stack:** Debian debhelper, dpkg, lintian, Docker/Ubuntu containers, GitHub Actions, Bash, Rust, Meson, CMake.

## Global Constraints

- Do not package until both preceding Ubuntu plans are green on native builds.
- Ship separate `oreokey-config`, `ibus-oreokey`, and `fcitx5-oreokey` packages; do not force both frameworks onto users.
- Package builds must not access user config, credentials, Apple signing material, or macOS release files.
- Build and validate Ubuntu 22.04/24.04 on amd64 and arm64.
- Release artifacts include SHA-256 checksums and exact Ubuntu series/architecture in their filenames.

---

## File Structure

| File | Responsibility |
| --- | --- |
| `packaging/debian/control` | Source and binary package metadata/dependencies. |
| `packaging/debian/rules` | Cargo, Meson and CMake build/install entry point. |
| `packaging/debian/changelog` | Debian version history. |
| `packaging/debian/*.install` | Split staging manifests for each binary package. |
| `packaging/debian/tests/smoke` | Installed-package smoke test. |
| `scripts/build-deb.sh` | Reproducible local/container package entry point. |
| `packaging/docker/Dockerfile.22.04` | 22.04 build environment. |
| `packaging/docker/Dockerfile.24.04` | 24.04 build environment. |
| `.github/workflows/ubuntu.yml` | Pull-request Ubuntu build/test matrix. |
| `.github/workflows/release.yml` | Tagged-release package, checksum and upload jobs. |
| `docs/install-ubuntu.md` | User installation, selection, troubleshooting and removal guide. |

### Task 1: Add Debian source packaging

**Files:**
- Create: `packaging/debian/control`, `packaging/debian/rules`, `packaging/debian/changelog`, `packaging/debian/oreokey-config.install`, `packaging/debian/ibus-oreokey.install`, `packaging/debian/fcitx5-oreokey.install`, `packaging/debian/source/format`
- Test: `packaging/debian/tests/smoke`

**Interfaces:**
- Consumes: staged output from IBus, Fcitx5, and config-app builds.
- Produces: three version-matched binary packages from one source package.

- [ ] **Step 1: Write failing package-content assertions**

```sh
dpkg-deb --contents "$1" | grep -q 'usr/bin/oreokey-config'
dpkg-deb --contents "$2" | grep -q 'usr/share/ibus/component/oreokey.xml'
dpkg-deb --contents "$3" | grep -q 'usr/share/fcitx5/inputmethod/oreokey-vi.conf'
```

- [ ] **Step 2: Run test to verify it fails**

Run: `dpkg-buildpackage -us -uc -b -d`

Expected: FAIL because `packaging/debian` is absent.

- [ ] **Step 3: Define three packages with strict dependencies**

`oreokey-config` owns the binary, desktop entry, icons and shared Rust library. `ibus-oreokey` depends on an exactly equal `oreokey-config` version plus IBus runtime. `fcitx5-oreokey` depends on an exactly equal config package version plus Fcitx5 runtime. Do not create a meta-package that depends on both frameworks. Rules build Rust once, Meson once and CMake once into `debian/tmp`, then split files through manifests.

- [ ] **Step 4: Build and inspect packages**

Run: `dpkg-buildpackage -us -uc -b -d && lintian ../oreokey_*.changes`

Expected: package build succeeds and lintian has no errors.

- [ ] **Step 5: Commit**

```bash
git add packaging/debian
git commit -m "build(deb): package OreoKey Linux components"
```

### Task 2: Create reproducible package checks

**Files:**
- Create: `scripts/build-deb.sh`, `packaging/debian/tests/smoke`, `packaging/docker/Dockerfile.22.04`, `packaging/docker/Dockerfile.24.04`
- Test: `packaging/debian/tests/smoke`

**Interfaces:**
- Consumes: `OREOKEY_UBUNTU_SERIES` and `OREOKEY_ARCH`.
- Produces: three .deb files and `SHA256SUMS` under `dist/ubuntu/$series/$arch`.

- [ ] **Step 1: Write failing no-user-data assertions**

```sh
! dpkg-deb --contents "$1" | grep -E '(/home/|Library/Application Support|\.ssh|appcast\.xml)'
dpkg-deb --info "$1" | grep -q '^ Package: '
```

- [ ] **Step 2: Run test to verify it fails**

Run: `packaging/debian/tests/smoke /tmp/oreokey-empty`

Expected: FAIL because required artifacts do not exist.

- [ ] **Step 3: Implement validated container build script**

Require series `22.04` or `24.04` and arch `amd64` or `arm64`; reject other inputs. Build inside the matching Dockerfile, copy only packages and checksum file to the exact dist directory, then run smoke checks. Use a fresh temporary container/output directory and never delete a caller-provided directory.

- [ ] **Step 4: Build one supported target locally**

Run: `OREOKEY_UBUNTU_SERIES=22.04 OREOKEY_ARCH=amd64 ./scripts/build-deb.sh`

Expected: three packages plus checksums under `dist/ubuntu/22.04/amd64`.

- [ ] **Step 5: Commit**

```bash
git add scripts/build-deb.sh packaging/docker packaging/debian/tests
git commit -m "test(deb): add Ubuntu package smoke checks"
```

### Task 3: Add CI and release artifacts

**Files:**
- Create: `.github/workflows/ubuntu.yml`
- Modify: `.github/workflows/release.yml`
- Test: `.github/workflows/ubuntu.yml`

**Interfaces:**
- Consumes: pull requests, pushes, and tag `v*`.
- Produces: verified artifacts named `*_ubuntu{series}_{arch}.deb` plus checksums.

- [ ] **Step 1: Write failing workflow matrix assertions**

```sh
yq -e '.jobs.test.strategy.matrix.series == ["22.04", "24.04"]' .github/workflows/ubuntu.yml
yq -e '.jobs.package.strategy.matrix.arch == ["amd64", "arm64"]' .github/workflows/ubuntu.yml
```

- [ ] **Step 2: Run test to verify it fails**

Run: `yq -e '.jobs' .github/workflows/ubuntu.yml`

Expected: FAIL because the workflow is absent.

- [ ] **Step 3: Implement non-release matrix**

Run core tests, IBus Meson tests, Fcitx5 CTest, GTK tests and Debian smoke checks inside 22.04/24.04 build environments. Use a native arm64 runner for arm64 release artifacts; do not silently emulate it. Upload an artifact only after that job's tests pass.

- [ ] **Step 4: Extend tagged release safely**

For tag releases, download all four series/architecture artifact sets, verify every checksum against `SHA256SUMS`, and attach packages to the existing GitHub Release. Keep macOS signing/notarization unchanged; a Linux failure must remain visible and must not upload unverified packages.

- [ ] **Step 5: Validate and run a local package target**

Run: `actionlint .github/workflows/ubuntu.yml .github/workflows/release.yml && OREOKEY_UBUNTU_SERIES=24.04 OREOKEY_ARCH=amd64 ./scripts/build-deb.sh`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/ubuntu.yml .github/workflows/release.yml
git commit -m "ci: build OreoKey Ubuntu packages"
```

### Task 4: Document installation and real-session verification

**Files:**
- Create: `docs/install-ubuntu.md`
- Modify: `README.md`, `docs/testing-checklist.md`, `CHANGELOG.md`
- Test: `docs/install-ubuntu.md`

**Interfaces:**
- Consumes: package names and artifact naming from Tasks 1-3.
- Produces: user-facing install/remove path for exactly one engine framework.

- [ ] **Step 1: Write failing documentation checks**

```sh
rg -q 'sudo apt install ./oreokey-config_' docs/install-ubuntu.md
rg -q 'ibus-oreokey' docs/install-ubuntu.md
rg -q 'fcitx5-oreokey' docs/install-ubuntu.md
```

- [ ] **Step 2: Run test to verify it fails**

Run: `rg 'oreokey-config' docs/install-ubuntu.md`

Expected: FAIL because the guide is absent.

- [ ] **Step 3: Write exact setup and removal paths**

Document selecting matching LTS/architecture packages, installing `oreokey-config` plus exactly one engine package, GNOME Input Sources for IBus, and `im-config` plus Fcitx5 Configuration for Fcitx5. Include app launch, Telex/VNI smoke sequence, diagnostic commands, and `apt remove`. Offer deletion of `~/.config/oreokey` only as an optional, explicit user action.

- [ ] **Step 4: Add real-machine acceptance matrix**

Add four rows: 22.04 GNOME Wayland IBus, 22.04 GNOME Xorg IBus, 24.04 KDE Wayland Fcitx5, and 24.04 KDE X11 Fcitx5. Each row verifies install, engine selection, Telex, VNI, hotkey, macro, restart, persistence, update and uninstall.

- [ ] **Step 5: Run documentation and workspace tests**

Run: `rg -q 'fcitx5-oreokey' docs/install-ubuntu.md && cargo test --workspace`

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add README.md docs/install-ubuntu.md docs/testing-checklist.md CHANGELOG.md
git commit -m "docs: add Ubuntu installation guide"
```

