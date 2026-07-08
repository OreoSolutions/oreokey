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
