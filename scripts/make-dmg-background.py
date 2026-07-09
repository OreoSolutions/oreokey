#!/usr/bin/env python3
"""Sinh ảnh nền cho cửa sổ cài đặt DMG → assets/dmg-background.png.

Chạy một lần khi muốn đổi thiết kế nền; ảnh kết quả được commit và
scripts/dmg-settings.py tham chiếu tới. Vẽ ở 2x rồi thu nhỏ (LANCZOS) cho nét.

Dùng: python3 scripts/make-dmg-background.py
"""
import os
from PIL import Image, ImageDraw, ImageFont

W, H = 620, 420          # kích thước cửa sổ (điểm) = kích thước ảnh 1x
S = 2                    # vẽ ở 2x cho mượt rồi thu nhỏ

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
OUT = os.path.join(ROOT, "assets", "dmg-background.png")

# Bảng màu
TOP = (245, 245, 247)          # gradient trên
BOTTOM = (232, 232, 237)       # gradient dưới
ACCENT = (124, 92, 175)        # tím theo logo (mũi tên)
INK = (60, 60, 67)             # chữ đậm (wordmark)
SUBTLE = (110, 110, 118)       # chữ hướng dẫn


def load_font(size, bold=False):
    candidates = (
        ["/System/Library/Fonts/SFNSDisplay-Bold.otf",
         "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
         "/Library/Fonts/Arial Bold.ttf"]
        if bold else
        ["/System/Library/Fonts/SFNS.ttf",
         "/System/Library/Fonts/Supplemental/Arial.ttf",
         "/System/Library/Fonts/Helvetica.ttc"]
    )
    for path in candidates:
        if os.path.exists(path):
            try:
                return ImageFont.truetype(path, size)
            except Exception:
                continue
    return ImageFont.load_default()


def centered(draw, text, font, cx, cy, fill):
    l, t, r, b = draw.textbbox((0, 0), text, font=font)
    draw.text((cx - (r - l) / 2, cy - (b - t) / 2 - t), text, font=font, fill=fill)


def main():
    img = Image.new("RGB", (W * S, H * S), TOP)
    d = ImageDraw.Draw(img)

    # Gradient dọc
    for y in range(H * S):
        f = y / (H * S - 1)
        c = tuple(round(TOP[i] + (BOTTOM[i] - TOP[i]) * f) for i in range(3))
        d.line([(0, y), (W * S, y)], fill=c)

    # Wordmark "OreoKey" phía trên, giữa
    centered(d, "OreoKey", load_font(30 * S, bold=True), (W / 2) * S, 40 * S, INK)

    # Mũi tên nối vùng icon (trái) → Applications (phải), ngang tâm icon y≈205
    ay = 205 * S
    x0, x1 = 250 * S, 366 * S
    shaft = 6 * S
    d.rounded_rectangle([x0, ay - shaft // 2, x1, ay + shaft // 2],
                        radius=shaft // 2, fill=ACCENT)
    head = 16 * S
    d.polygon([(x1, ay - head), (x1 + head, ay), (x1, ay + head)], fill=ACCENT)

    # Hướng dẫn phía dưới
    centered(d, "Kéo OreoKey vào thư mục Applications",
             load_font(15 * S), (W / 2) * S, 348 * S, SUBTLE)

    os.makedirs(os.path.dirname(OUT), exist_ok=True)
    img.resize((W, H), Image.LANCZOS).save(OUT, "PNG")
    print("Đã ghi", OUT)


if __name__ == "__main__":
    main()
