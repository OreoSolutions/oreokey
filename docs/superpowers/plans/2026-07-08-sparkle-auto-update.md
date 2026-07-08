# Sparkle Auto-Update Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tích hợp Sparkle 2.x để OreoKey tự kiểm tra & cài bản cập nhật từ GitHub Releases, hỏi người dùng trước khi cài, kèm changelog.

**Architecture:** App SwiftPM executable, đóng gói `.app` thủ công qua `scripts/build.sh`. Thêm Sparkle làm SPM dependency, nhúng `Sparkle.framework` vào bundle (build.sh), đọc feed `appcast.xml` (raw file trong repo) trỏ tới DMG trên GitHub Releases. Chữ ký EdDSA của Sparkle độc lập với Apple code-signing. Phát hành qua script cục bộ `scripts/release.sh`.

**Tech Stack:** Swift/AppKit (app), Sparkle 2.x (XCFramework qua SPM), bash + python3 (build & release scripts), `gh` CLI, GitHub Releases.

## Global Constraints

- macOS 13+ (`LSMinimumSystemVersion` = 13.0, `platforms: [.macOS(.v13)]`).
- Bundle id: `com.oreosolutions.oreokey`. Repo: `OreoSolutions/oreokey`.
- `SUFeedURL` = `https://raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml` (verbatim).
- Sparkle version floor: `from: "2.6.0"`.
- Ký ad-hoc (`-`) khi dev; Developer ID qua biến `CODESIGN_ID` khi phát hành (đang chờ tài khoản). Mã/build phải chạy được ở cả hai chế độ.
- Khóa **riêng** EdDSA nằm trong Keychain đăng nhập, **không bao giờ** commit vào git.
- Engine Rust không đổi trong plan này.
- Chu kỳ tự kiểm tra: 86400s (24h).

## File Structure

- Modify `app/Package.swift` — thêm Sparkle dependency.
- Modify `app/Info.plist` — thêm 4 khóa Sparkle (`SUFeedURL`, `SUPublicEDKey`, `SUEnableAutomaticChecks`, `SUScheduledCheckInterval`).
- Create `app/Sources/OreoKey/Updater.swift` — bọc `SPUStandardUpdaterController`.
- Modify `app/Sources/OreoKey/AppDelegate.swift` — khởi động updater + mục menu "Kiểm tra bản mới…".
- Modify `scripts/build.sh` — rpath, copy + ký nested `Sparkle.framework`.
- Create `appcast.xml` (gốc repo) — feed khởi tạo (rỗng item).
- Create `scripts/roll-changelog.py` — cuốn `[Chưa phát hành]` → `[version] - ngày`.
- Create `scripts/update-appcast.py` — chèn `<item>` release vào `appcast.xml`.
- Create `.github/workflows/release.yml` — phát hành qua `workflow_dispatch`.
- Modify `README.md` — thay mục "Auto-update (Sparkle) chưa tích hợp" thành hướng dẫn thật.

---

### Task 1: Thêm Sparkle làm SPM dependency

**Files:**
- Modify: `app/Package.swift`

**Interfaces:**
- Produces: module `Sparkle` khả dụng cho target `OreoKey` (dùng ở Task 4).

- [ ] **Step 1: Sửa Package.swift**

Thay toàn bộ nội dung `app/Package.swift` bằng:

```swift
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "OreoKey",
    platforms: [.macOS(.v13)],
    dependencies: [
        .package(url: "https://github.com/sparkle-project/Sparkle", from: "2.6.0"),
    ],
    targets: [
        .systemLibrary(name: "COreoKey", path: "Sources/COreoKey"),
        .executableTarget(
            name: "OreoKey",
            dependencies: [
                "COreoKey",
                .product(name: "Sparkle", package: "Sparkle"),
            ],
            path: "Sources/OreoKey",
            linkerSettings: [
                .linkedLibrary("oreokey_core"),
                .linkedFramework("AppKit"),
                .linkedFramework("Carbon"),
                .linkedFramework("ApplicationServices"),
                .linkedFramework("ServiceManagement"),
            ]
        ),
    ]
)
```

- [ ] **Step 2: Giải quyết dependency (cần mạng)**

Run: `cd app && swift package resolve`
Expected: tải Sparkle, in dòng `Computed ... Sparkle ... at 2.x.y`. Không lỗi.

- [ ] **Step 3: Xác nhận framework artifact đã có mặt**

Run: `find app/.build/artifacts -type d -name 'Sparkle.framework' -path '*macos*'`
Expected: in ít nhất một đường dẫn kết thúc `…/Sparkle.xcframework/macos-arm64_x86_64/Sparkle.framework` (tên slice có thể khác chút, miễn chứa `macos`).

- [ ] **Step 4: Commit**

```bash
cd "$(git rev-parse --show-toplevel)"
git add app/Package.swift app/Package.resolved
git commit -m "build: thêm Sparkle 2.x làm SPM dependency"
```

---

### Task 2: Nhúng + ký Sparkle.framework trong build.sh

Sau khi link Sparkle, binary cần framework trong bundle mới chạy được. Task này làm `dist/OreoKey.app` khởi động được với Sparkle đã link.

**Files:**
- Modify: `scripts/build.sh`

**Interfaces:**
- Consumes: `Sparkle.framework` artifact (Task 1).
- Produces: `dist/OreoKey.app/Contents/Frameworks/Sparkle.framework` + rpath đúng; app chạy được.

- [ ] **Step 1: Thêm rpath vào cả hai lệnh swift build**

Trong `scripts/build.sh`, ở mục "2. Swift app", thêm cờ rpath vào **cả hai** lần gọi `swift build`. Sửa dòng build:

Tìm:
```bash
swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR"
```
Thay bằng:
```bash
swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR" \
    -Xlinker -rpath -Xlinker @executable_path/../Frameworks
```

Và tìm dòng `--show-bin-path`:
```bash
BIN="$(swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR" --show-bin-path)/OreoKey"
```
Thay bằng:
```bash
BIN="$(swift build -c release $ARCH_FLAGS -Xlinker -L"$LIBDIR" \
    -Xlinker -rpath -Xlinker @executable_path/../Frameworks --show-bin-path)/OreoKey"
```

- [ ] **Step 2: Nhúng framework — chèn trước bước "4. Ký"**

Trong `scripts/build.sh`, ngay **trước** khối `# 4. Ký`, chèn khối mới:

```bash
# 3b. Nhúng Sparkle.framework vào bundle
SPARKLE_FW="$(find "$ROOT/app/.build" -type d -name 'Sparkle.framework' -path '*macos*' | head -1)"
[[ -n "$SPARKLE_FW" ]] || { echo "❌ Không tìm thấy Sparkle.framework (chạy 'swift package resolve' trong app/?)"; exit 1; }
mkdir -p "$APP/Contents/Frameworks"
rm -rf "$APP/Contents/Frameworks/Sparkle.framework"
cp -R "$SPARKLE_FW" "$APP/Contents/Frameworks/"
```

- [ ] **Step 3: Ký nested-first — thay khối "4. Ký"**

Thay toàn bộ khối `# 4. Ký` hiện tại:

```bash
# 4. Ký
IDENTITY="${CODESIGN_ID:--}"
codesign --force --options runtime --sign "$IDENTITY" "$APP" 2>/dev/null \
    || codesign --force --sign - "$APP"
```

bằng:

```bash
# 4. Ký — nested (Sparkle) trước, app sau; hardened runtime khi có Developer ID
IDENTITY="${CODESIGN_ID:--}"
FW="$APP/Contents/Frameworks/Sparkle.framework"
sign() { codesign --force --options runtime --sign "$IDENTITY" "$1" 2>/dev/null \
    || codesign --force --sign "$IDENTITY" "$1"; }
for nested in \
    "$FW/Versions/Current/XPCServices/Installer.xpc" \
    "$FW/Versions/Current/XPCServices/Downloader.xpc" \
    "$FW/Versions/Current/Autoupdate" \
    "$FW/Versions/Current/Updater.app"; do
    [[ -e "$nested" ]] && sign "$nested"
done
sign "$FW"
sign "$APP"
```

- [ ] **Step 4: Build và kiểm tra nhúng**

Run: `./scripts/build.sh`
Expected: `✅ Đã build`. Không lỗi.

Run: `ls dist/OreoKey.app/Contents/Frameworks/`
Expected: `Sparkle.framework`.

Run: `otool -l dist/OreoKey.app/Contents/MacOS/OreoKey | grep -A2 LC_RPATH | grep Frameworks`
Expected: có dòng `path @executable_path/../Frameworks`.

Run: `codesign --verify --deep --strict dist/OreoKey.app && echo VERIFY_OK`
Expected: `VERIFY_OK` (ad-hoc vẫn verify được cấu trúc).

- [ ] **Step 5: Xác nhận app khởi động được với Sparkle đã link**

Run: `open dist/OreoKey.app` rồi `sleep 3 && pgrep -x OreoKey && echo RUNNING`
Expected: `RUNNING` (không crash vì thiếu framework). Sau đó `killall OreoKey` để dọn.

- [ ] **Step 6: Commit**

```bash
git add scripts/build.sh
git commit -m "build: nhúng và ký Sparkle.framework vào bundle"
```

---

### Task 3: Khóa EdDSA + khóa Info.plist

**Files:**
- Modify: `app/Info.plist`

**Interfaces:**
- Produces: `SUPublicEDKey` trong Info.plist (Sparkle dùng để xác minh DMG); khóa riêng trong Keychain (release.sh dùng ở Task 5).

- [ ] **Step 1: Định vị công cụ generate_keys**

Run: `find app/.build/artifacts -name generate_keys -o -name generate_keys.app 2>/dev/null; ls app/.build/artifacts/*/Sparkle/bin/ 2>/dev/null`
Expected: in đường dẫn tới `generate_keys` (thường `app/.build/artifacts/sparkle/Sparkle/bin/generate_keys`). Ghi lại đường dẫn này là `$GENKEYS`.

Nếu KHÔNG tìm thấy trong artifacts, tải bộ công cụ Sparkle rời:
```bash
curl -L -o /tmp/sparkle.tar.xz https://github.com/sparkle-project/Sparkle/releases/download/2.6.4/Sparkle-2.6.4.tar.xz
mkdir -p /tmp/sparkle && tar -xf /tmp/sparkle.tar.xz -C /tmp/sparkle
# $GENKEYS = /tmp/sparkle/bin/generate_keys ; sign_update = /tmp/sparkle/bin/sign_update
```

- [ ] **Step 2: Sinh cặp khóa (một lần)**

Run: `"$GENKEYS"` (đường dẫn từ Step 1)
Expected: khóa riêng lưu vào Keychain đăng nhập; in ra một dòng khóa công khai dạng
`<string>BASE64_PUBLIC_KEY</string>` kèm hướng dẫn "Insert this into your Info.plist". Copy giá trị base64.

Nếu đã có khóa từ trước, chạy `"$GENKEYS" -p` để chỉ in lại khóa công khai.

- [ ] **Step 3: Thêm 4 khóa vào Info.plist**

Trong `app/Info.plist`, thêm vào trong `<dict>` (trước `</dict>` cuối) — thay `BASE64_PUBLIC_KEY` bằng giá trị thật từ Step 2:

```xml
	<key>SUFeedURL</key>
	<string>https://raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml</string>
	<key>SUPublicEDKey</key>
	<string>BASE64_PUBLIC_KEY</string>
	<key>SUEnableAutomaticChecks</key>
	<true/>
	<key>SUScheduledCheckInterval</key>
	<integer>86400</integer>
```

- [ ] **Step 4: Xác nhận plist hợp lệ**

Run: `plutil -lint app/Info.plist`
Expected: `app/Info.plist: OK`.

Run: `/usr/libexec/PlistBuddy -c 'Print :SUFeedURL' app/Info.plist`
Expected: URL appcast đúng.

- [ ] **Step 5: Commit**

```bash
git add app/Info.plist
git commit -m "feat: thêm khóa cấu hình Sparkle (feed + public EdDSA key)"
```

---

### Task 4: Updater.swift + mục menu "Kiểm tra bản mới…"

**Files:**
- Create: `app/Sources/OreoKey/Updater.swift`
- Modify: `app/Sources/OreoKey/AppDelegate.swift`

**Interfaces:**
- Consumes: module `Sparkle` (Task 1), khóa Info.plist (Task 3).
- Produces: `Updater.shared` chạy nền + mục menu bấm thủ công.

- [ ] **Step 1: Tạo Updater.swift**

Tạo `app/Sources/OreoKey/Updater.swift`:

```swift
import Sparkle

/// Bọc bộ cập nhật Sparkle. Khởi tạo `shared` là bắt đầu kiểm tra nền
/// theo cấu hình Info.plist (SUFeedURL, SUEnableAutomaticChecks, interval).
final class Updater {
    static let shared = Updater()

    let controller: SPUStandardUpdaterController

    private init() {
        controller = SPUStandardUpdaterController(
            startingUpdater: true,
            updaterDelegate: nil,
            userDriverDelegate: nil
        )
    }
}
```

- [ ] **Step 2: Khởi động updater khi app chạy**

Trong `app/Sources/OreoKey/AppDelegate.swift`, trong `applicationDidFinishLaunching`, ngay sau dòng `setupStatusItem()`, thêm:

```swift
        _ = Updater.shared  // bắt đầu kiểm tra cập nhật nền
```

- [ ] **Step 3: Thêm mục menu "Kiểm tra bản mới…"**

Trong `setupStatusItem()` của `AppDelegate.swift`, ngay **sau** khối tạo `settings` menu item (dòng `menu.addItem(settings)`), thêm:

```swift
        let updates = NSMenuItem(
            title: "Kiểm tra bản mới…",
            action: #selector(SPUStandardUpdaterController.checkForUpdates(_:)),
            keyEquivalent: "")
        updates.target = Updater.shared.controller
        menu.addItem(updates)
```

Thêm `import Sparkle` ở đầu file `AppDelegate.swift` (cùng nhóm với `import AppKit`).

- [ ] **Step 4: Build**

Run: `./scripts/build.sh`
Expected: `✅ Đã build`, không lỗi biên dịch.

- [ ] **Step 5: Kiểm tra thủ công — mục menu xuất hiện & tự vô hiệu khi đang kiểm tra**

Run: `open dist/OreoKey.app`
Kỳ vọng: bấm icon menu bar → thấy mục **"Kiểm tra bản mới…"** ngay dưới "Cài đặt…". Bấm nó → Sparkle kiểm tra feed (có thể báo "hiện là bản mới nhất" hoặc lỗi mạng nếu appcast chưa có — bình thường ở giai đoạn này). Sau đó `killall OreoKey`.

- [ ] **Step 6: Commit**

```bash
git add app/Sources/OreoKey/Updater.swift app/Sources/OreoKey/AppDelegate.swift
git commit -m "feat: bộ cập nhật Sparkle + mục menu 'Kiểm tra bản mới'"
```

---

### Task 5: appcast.xml + script Python + GitHub Actions release

**Files:**
- Create: `appcast.xml`
- Create: `scripts/roll-changelog.py`
- Create: `scripts/update-appcast.py`
- Create: `.github/workflows/release.yml`

**Interfaces:**
- Consumes: `build.sh`, `make-dmg.sh` (Task 2), `sign_update`, secret `SPARKLE_ED_PRIVATE_KEY`.
- Produces: phát hành qua `workflow_dispatch`; `appcast.xml` được cập nhật & commit về `main` bởi CI.

- [ ] **Step 1: Tạo appcast.xml khởi tạo**

Tạo `appcast.xml` ở gốc repo:

```xml
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0"
     xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle"
     xmlns:dc="http://purl.org/dc/elements/1.1/">
  <channel>
    <title>OreoKey</title>
    <link>https://raw.githubusercontent.com/OreoSolutions/oreokey/main/appcast.xml</link>
    <description>Cập nhật OreoKey</description>
    <language>vi</language>
    <!-- RELEASE_ITEMS -->
  </channel>
</rss>
```

- [ ] **Step 2: Tạo scripts/roll-changelog.py**

Tạo `scripts/roll-changelog.py`:

```python
#!/usr/bin/env python3
"""Cuốn mục [Chưa phát hành] trong CHANGELOG.md thành [version] - <ngày>.
Dùng: roll-changelog.py <version> [--date YYYY-MM-DD]
In ra stdout phần release notes (nội dung mục vừa cuốn)."""
import sys, re, datetime, argparse, pathlib

p = argparse.ArgumentParser()
p.add_argument("version")
p.add_argument("--date", default=datetime.date.today().isoformat())
p.add_argument("--file", default="CHANGELOG.md")
a = p.parse_args()

path = pathlib.Path(a.file)
s = path.read_text(encoding="utf-8")
if "## [Chưa phát hành]" not in s:
    sys.exit("Không thấy mục '## [Chưa phát hành]' trong CHANGELOG.md")

s = s.replace("## [Chưa phát hành]",
              f"## [Chưa phát hành]\n\n## [{a.version}] - {a.date}", 1)
path.write_text(s, encoding="utf-8")

# Trích notes của version vừa tạo (để dùng làm release notes)
m = re.search(rf"## \[{re.escape(a.version)}\].*?\n(.*?)(?=\n## \[|\Z)", s, re.S)
print((m.group(1).strip() if m else "").strip())
```

- [ ] **Step 3: Tạo scripts/update-appcast.py**

Tạo `scripts/update-appcast.py`:

```python
#!/usr/bin/env python3
"""Chèn một <item> release vào appcast.xml tại mốc <!-- RELEASE_ITEMS -->.
Dùng: update-appcast.py <version> <build> <download_url> <sig_attrs> [--notes-file F]
  <sig_attrs>: chuỗi thuộc tính từ sign_update, vd:
     sparkle:edSignature="..." length="123"
"""
import sys, re, html, datetime, email.utils, argparse, pathlib

p = argparse.ArgumentParser()
p.add_argument("version")
p.add_argument("build")
p.add_argument("download_url")
p.add_argument("sig_attrs")
p.add_argument("--notes-file")
p.add_argument("--appcast", default="appcast.xml")
a = p.parse_args()

notes = ""
if a.notes_file:
    notes = pathlib.Path(a.notes_file).read_text(encoding="utf-8").strip()
notes_html = "<pre>" + html.escape(notes) + "</pre>"
pubdate = email.utils.format_datetime(datetime.datetime.now(datetime.timezone.utc))

item = f"""    <item>
      <title>{a.version}</title>
      <pubDate>{pubdate}</pubDate>
      <sparkle:version>{a.build}</sparkle:version>
      <sparkle:shortVersionString>{a.version}</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>13.0</sparkle:minimumSystemVersion>
      <description><![CDATA[{notes_html}]]></description>
      <enclosure url="{a.download_url}"
                 sparkle:version="{a.build}"
                 sparkle:shortVersionString="{a.version}"
                 type="application/octet-stream"
                 {a.sig_attrs} />
    </item>
    <!-- RELEASE_ITEMS -->"""

path = pathlib.Path(a.appcast)
s = path.read_text(encoding="utf-8")
if "    <!-- RELEASE_ITEMS -->" not in s:
    sys.exit("Thiếu mốc <!-- RELEASE_ITEMS --> trong appcast.xml")
path.write_text(s.replace("    <!-- RELEASE_ITEMS -->", item, 1), encoding="utf-8")
print("appcast.xml: đã chèn item", a.version)
```

- [ ] **Step 4: Kiểm cú pháp Python + XML hợp lệ**

Run: `python3 -m py_compile scripts/roll-changelog.py scripts/update-appcast.py && echo PY_OK`
Expected: `PY_OK`.

Run: `xmllint --noout appcast.xml && echo XML_OK`
Expected: `XML_OK`.

- [ ] **Step 5: Kiểm tra hai script bằng bản sao tạm (không đụng file thật)**

```bash
TMP="$(mktemp -d)"
cp CHANGELOG.md appcast.xml "$TMP/"
python3 scripts/roll-changelog.py 9.9.9 --file "$TMP/CHANGELOG.md" > "$TMP/notes.txt"
python3 scripts/update-appcast.py 9.9.9 42 \
    "https://example.com/OreoKey.dmg" 'sparkle:edSignature="AAA" length="1"' \
    --notes-file "$TMP/notes.txt" --appcast "$TMP/appcast.xml"
xmllint --noout "$TMP/appcast.xml" && echo ROUNDTRIP_OK
grep -q '## \[9.9.9\]' "$TMP/CHANGELOG.md" && echo CHANGELOG_OK
rm -rf "$TMP"
```
Expected: in `ROUNDTRIP_OK` và `CHANGELOG_OK`. (File thật không đổi.)

- [ ] **Step 6: Tạo .github/workflows/release.yml**

Tạo `.github/workflows/release.yml`:

```yaml
name: Release
on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version (vd 0.2.0)'
        required: true

permissions:
  contents: write   # commit appcast + tạo tag/release

jobs:
  release:
    runs-on: macos-14
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Thêm Rust targets
        run: rustup target add aarch64-apple-darwin x86_64-apple-darwin

      - name: Resolve Swift packages
        run: cd app && swift package resolve

      - name: Bump version
        id: bump
        run: |
          V="${{ inputs.version }}"
          /usr/libexec/PlistBuddy -c "Set :CFBundleShortVersionString $V" app/Info.plist
          OLD=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' app/Info.plist)
          NEW=$((OLD + 1))
          /usr/libexec/PlistBuddy -c "Set :CFBundleVersion $NEW" app/Info.plist
          sed -i '' -E "s/^version = \".*\"/version = \"$V\"/" core/Cargo.toml
          echo "build=$NEW" >> "$GITHUB_OUTPUT"

      - name: Cuốn changelog
        run: python3 scripts/roll-changelog.py "${{ inputs.version }}" > /tmp/notes.txt

      - name: Build + DMG
        env:
          CODESIGN_ID: ${{ secrets.CODESIGN_ID }}
          NOTARY_PROFILE: ${{ secrets.NOTARY_PROFILE }}
        run: |
          ./scripts/build.sh --universal
          ./scripts/make-dmg.sh

      - name: Ký EdDSA (Sparkle)
        id: sign
        env:
          SPARKLE_ED_PRIVATE_KEY: ${{ secrets.SPARKLE_ED_PRIVATE_KEY }}
        run: |
          SIGN_UPDATE="$(find app/.build/artifacts -name sign_update | head -1)"
          [ -n "$SIGN_UPDATE" ] || { echo "Không tìm thấy sign_update"; exit 1; }
          printf '%s' "$SPARKLE_ED_PRIVATE_KEY" > /tmp/sparkle_ed_priv
          # sign_update in ra: sparkle:edSignature="..." length="..."
          ATTRS="$("$SIGN_UPDATE" -f /tmp/sparkle_ed_priv dist/OreoKey.dmg)"
          rm -f /tmp/sparkle_ed_priv
          echo "attrs=$ATTRS" >> "$GITHUB_OUTPUT"

      - name: Cập nhật appcast
        run: |
          python3 scripts/update-appcast.py \
            "${{ inputs.version }}" "${{ steps.bump.outputs.build }}" \
            "https://github.com/OreoSolutions/oreokey/releases/download/v${{ inputs.version }}/OreoKey.dmg" \
            '${{ steps.sign.outputs.attrs }}' \
            --notes-file /tmp/notes.txt

      - name: Commit, tag, push
        run: |
          git config user.name  'github-actions[bot]'
          git config user.email 'github-actions[bot]@users.noreply.github.com'
          git add app/Info.plist core/Cargo.toml CHANGELOG.md appcast.xml
          git commit -m "release: ${{ inputs.version }}"
          git tag "v${{ inputs.version }}"
          git push origin HEAD:main
          git push origin "v${{ inputs.version }}"

      - name: Tạo GitHub Release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release create "v${{ inputs.version }}" dist/OreoKey.dmg \
            --title "OreoKey ${{ inputs.version }}" \
            --notes-file /tmp/notes.txt
```

**Lưu ý cờ `sign_update`:** bản plan dùng `-f <file>` để đọc khóa riêng từ
file. Nếu `sign_update --help` (trong artifact) báo cờ khác (một số bản dùng
`-s <base64-key>` hoặc `--ed-key-file`), đổi cho khớp — chạy
`"$SIGN_UPDATE" --help` một lần để xác nhận trước khi hoàn tất.

- [ ] **Step 7: Chuẩn bị secret khóa riêng EdDSA (một lần, thủ công)**

Xuất khóa riêng đã sinh ở Task 3 rồi thêm vào GitHub Secrets:
```bash
"$GENKEYS" -x /tmp/oreokey_ed_priv        # $GENKEYS từ Task 3 Step 1
gh secret set SPARKLE_ED_PRIVATE_KEY < /tmp/oreokey_ed_priv
rm -f /tmp/oreokey_ed_priv
```
Expected: `✓ Set Actions secret SPARKLE_ED_PRIVATE_KEY`.
(Nếu `gh` chưa đăng nhập: `gh auth login`. Có thể dán tay ở Settings → Secrets → Actions.)

- [ ] **Step 8: Kiểm workflow hợp lệ cú pháp YAML**

Run: `python3 -c "import yaml,sys; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML_OK')"`
Expected: `YAML_OK`. (Nếu thiếu PyYAML: `python3 -c "import json; print('skip')"` và kiểm tra bằng mắt cấu trúc indent.)

- [ ] **Step 9: Commit**

```bash
git add appcast.xml scripts/roll-changelog.py scripts/update-appcast.py .github/workflows/release.yml
git commit -m "release: appcast + workflow GitHub Actions (workflow_dispatch)"
```

---

### Task 6: Kiểm thử end-to-end luồng cập nhật (appcast giả)

Xác nhận app thật sự phát hiện & hiển thị bản mới. Dùng version giả cao để ép hộp thoại update, không cần phát hành thật.

**Files:**
- (không sửa mã; chỉ tạo file tạm để test)

- [ ] **Step 1: Build bản hiện tại (0.1.0)**

Run: `./scripts/build.sh` (nếu chưa có bản mới nhất trong `dist/`).
Expected: `✅ Đã build`.

- [ ] **Step 2: Tạo appcast giả version cao trong scratchpad**

Ghi file `/private/tmp/claude-501/-Users-quanguyen-Desktop-OreoSolutions-oreokey/e62bdbfa-1671-4b59-929d-fa42de314ef6/scratchpad/fake-appcast.xml` (dùng DMG 0.1.0 thật nếu đã build, hoặc chỉ cần metadata để hộp thoại hiện — Sparkle hiện dialog trước khi tải):

```xml
<?xml version="1.0" encoding="utf-8"?>
<rss version="2.0" xmlns:sparkle="http://www.andymatuschak.org/xml-namespaces/sparkle">
  <channel>
    <title>OreoKey</title>
    <item>
      <title>9.9.9</title>
      <sparkle:version>999</sparkle:version>
      <sparkle:shortVersionString>9.9.9</sparkle:shortVersionString>
      <sparkle:minimumSystemVersion>13.0</sparkle:minimumSystemVersion>
      <description><![CDATA[<pre>Bản thử nghiệm ép cập nhật.</pre>]]></description>
      <enclosure url="https://github.com/OreoSolutions/oreokey/releases/download/v9.9.9/OreoKey.dmg"
                 sparkle:version="999" type="application/octet-stream" length="1" />
    </item>
  </channel>
</rss>
```

- [ ] **Step 3: Trỏ app tới appcast giả tạm thời**

Run: phục vụ file qua HTTP cục bộ (Sparkle cần http/https, không nhận file://):
```bash
cd "$(dirname <đường dẫn scratchpad ở trên>)" && python3 -m http.server 8765 &
```
Rồi tạm sửa `SUFeedURL` trong **bundle đã build** (không sửa nguồn):
```bash
/usr/libexec/PlistBuddy -c 'Set :SUFeedURL http://localhost:8765/fake-appcast.xml' \
    dist/OreoKey.app/Contents/Info.plist
```

- [ ] **Step 4: Chạy app & kích hoạt kiểm tra**

Run: `open dist/OreoKey.app`, bấm menu → **"Kiểm tra bản mới…"**.
Expected: hộp thoại Sparkle hiện ra báo **"OreoKey 9.9.9 hiện đã có"** kèm release notes "Bản thử nghiệm ép cập nhật". (Bấm "Để sau"/hủy — không cần cài; enclosure length=1 sẽ lỗi nếu tải, nhưng mục tiêu là xác nhận PHÁT HIỆN + hộp thoại.)

- [ ] **Step 5: Dọn dẹp**

```bash
kill %1 2>/dev/null || true      # dừng http.server
killall OreoKey 2>/dev/null || true
rm -f dist/OreoKey.app/Contents/Info.plist.bak
./scripts/build.sh               # build lại bản sạch (feed thật)
```

- [ ] **Step 6: Ghi nhận kết quả**

Không commit gì (chỉ test). Ghi lại trong PR/mô tả: hộp thoại phát hiện bản mới hoạt động với ký ad-hoc; auto-install liền mạch chờ Developer ID.

---

### Task 7: Cập nhật README mục auto-update

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Thay phần Sparkle "chưa tích hợp"**

Trong `README.md`, mục "## Phát hành thật", thay mục 3 (bắt đầu "Auto-update (Sparkle) chưa tích hợp…") bằng:

```markdown
3. Phát hành qua **GitHub Actions**: tab Actions → workflow "Release" →
   "Run workflow" → nhập version (vd `0.2.0`). CI tự bump version, cuốn mục
   `[Chưa phát hành]` trong `CHANGELOG.md`, tag `vX`, build universal, đóng DMG
   (ký + notarize nếu có secret `CODESIGN_ID`/`NOTARY_PROFILE`), ký EdDSA, cập
   nhật `appcast.xml`, và tạo GitHub Release kèm DMG. Trước khi viết mục changelog
   cho bản mới, điền vào phần `[Chưa phát hành]`.

   Cài đặt một lần: sinh khóa Sparkle bằng `generate_keys` (kèm trong artifact
   Sparkle), dán khóa công khai vào `SUPublicEDKey` ở `app/Info.plist`, và đặt
   khóa riêng vào secret `SPARKLE_ED_PRIVATE_KEY` (`generate_keys -x` để xuất).
```

- [ ] **Step 2: Xác nhận không còn nhắc "chưa tích hợp"**

Run: `grep -n "chưa tích hợp" README.md || echo NONE`
Expected: `NONE`.

- [ ] **Step 3: Commit**

```bash
git add README.md
git commit -m "docs: hướng dẫn phát hành & auto-update qua Sparkle"
```

---

## Self-Review

- **Spec coverage:** Appcast raw-file host (Task 3 Info.plist + Task 5 appcast), hỏi-trước-khi-cài (mặc định Sparkle + Task 3 `SUEnableAutomaticChecks`), phát hành qua GitHub Actions `workflow_dispatch` (Task 5), cuốn changelog từ `[Chưa phát hành]` (Task 5 `roll-changelog.py`), nhúng framework thủ công (Task 2), khóa EdDSA + secret (Task 3 + Task 5 Step 7), menu "Kiểm tra bản mới" (Task 4), kiểm thử appcast giả (Task 6). Không delta / không notarize-CI bản đầu (ngoài phạm vi). ✓
- **Placeholder scan:** `BASE64_PUBLIC_KEY` là giá trị người dùng dán ở Task 3 Step 2 (có hướng dẫn lấy), không phải placeholder bỏ ngỏ. Đường dẫn artifact tìm bằng `find` vì SPM có thể đổi tên thư mục. Cờ `sign_update` được xác nhận qua `--help` (Task 5 Step 6 note). ✓
- **Type/tên nhất quán:** `Updater.shared.controller` (Task 4) là `SPUStandardUpdaterController`; menu action `checkForUpdates(_:)` khớp selector của lớp đó. Mốc `    <!-- RELEASE_ITEMS -->` (4 dấu cách) khớp giữa `appcast.xml` (Task 5 Step 1) và `update-appcast.py` (Step 3). Secret `SPARKLE_ED_PRIVATE_KEY` khớp giữa workflow (Step 6) và bước đặt secret (Step 7). `SUFeedURL` + repo `OreoSolutions/oreokey` khớp Global Constraints. ✓
- **Rủi ro đã ghi:** đường dẫn slice XCFramework & thứ tự ký nested (Task 2); công cụ Sparkle có thể nằm ngoài artifact (Task 3 Step 1 có fallback tải tarball); cờ `sign_update` khác bản (Task 5 note); push về `main` có thể vướng branch protection (ghi trong spec).
