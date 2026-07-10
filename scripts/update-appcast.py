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

def _inline(text: str) -> str:
    """Escape rồi áp định dạng inline: **đậm**, `mã`."""
    text = html.escape(text)
    text = re.sub(r"\*\*(.+?)\*\*", r"<strong>\1</strong>", text)
    text = re.sub(r"`([^`]+)`", r"<code>\1</code>", text)
    return text


def md_to_html(md: str) -> str:
    """Chuyển tập con Markdown của CHANGELOG (### tiêu đề, danh sách -,
    **đậm**, `mã`) sang HTML để Sparkle render thay vì hiện ký tự thô.
    Dòng nối (thụt lề dưới một mục -) được gộp vào chính mục đó."""
    lines = md.split("\n")
    out: list[str] = []
    in_list = False
    i = 0

    def close_list() -> None:
        nonlocal in_list
        if in_list:
            out.append("</ul>")
            in_list = False

    while i < len(lines):
        stripped = lines[i].strip()
        if not stripped:
            close_list()
            i += 1
            continue
        heading = re.match(r"^(#{1,6})\s+(.*)$", stripped)
        if heading:
            close_list()
            level = len(heading.group(1))
            out.append(f"<h{level}>{_inline(heading.group(2))}</h{level}>")
            i += 1
            continue
        if stripped.startswith("- "):
            if not in_list:
                out.append("<ul>")
                in_list = True
            item = [stripped[2:]]
            i += 1
            while i < len(lines) and lines[i].strip():
                nxt = lines[i].strip()
                if nxt.startswith("- ") or re.match(r"^#{1,6}\s", nxt):
                    break
                item.append(nxt)
                i += 1
            out.append(f"<li>{_inline(' '.join(item))}</li>")
            continue
        close_list()
        out.append(f"<p>{_inline(stripped)}</p>")
        i += 1

    close_list()
    return "\n".join(out)


notes = ""
if a.notes_file:
    notes = pathlib.Path(a.notes_file).read_text(encoding="utf-8").strip()
notes_html = md_to_html(notes)
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
