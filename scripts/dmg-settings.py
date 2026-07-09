# Cấu hình dmgbuild cho cửa sổ cài đặt OreoKey.
# dmgbuild chạy file này qua exec() nên KHÔNG có __file__; mọi đường dẫn
# nhận qua cờ -D từ make-dmg.sh (app, background — đường dẫn tuyệt đối).
#   dmgbuild -s scripts/dmg-settings.py -D app=<.app> -D background=<png> "OreoKey X.Y.Z" OreoKey.dmg
import os.path

application = defines.get("app") or os.path.abspath(os.path.join("dist", "OreoKey.app"))
_appname = os.path.basename(application)
background = defines.get("background") or os.path.abspath(
    os.path.join("assets", "dmg-background.png"))

format = "UDZO"
files = [application]
symlinks = {"Applications": "/Applications"}

# Cửa sổ: icon-view, nền thương hiệu, ẩn mọi thanh cho gọn.
window_rect = ((200, 200), (620, 420))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
arrange_by = None

icon_size = 128
text_size = 13
# Tâm icon khớp bố cục ảnh nền: app bên trái, Applications bên phải.
icon_locations = {
    _appname: (150, 205),
    "Applications": (470, 205),
}
