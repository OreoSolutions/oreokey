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
