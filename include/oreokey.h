// OreoKey core — C ABI cho lớp vỏ Swift.
// Mọi char* trả về phải giải phóng bằng ok_str_free.
#ifndef OREOKEY_H
#define OREOKEY_H

#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// Vòng đời event tap. ok_start trả false nếu chưa có quyền Accessibility.
bool ok_start(void);
void ok_stop(void);
bool ok_is_running(void);

// Quyền Accessibility.
bool ok_ax_trusted(void);

// Bật/tắt tiếng Việt (trạng thái thực tế của app đang focus).
bool ok_get_enabled(void);
void ok_set_enabled(bool on);

// Settings dạng JSON (schema: xem core/src/config.rs).
char *ok_settings_json_get(void);
bool ok_settings_json_set(const char *json);

// Callback khi trạng thái VN/EN đổi (hotkey, đổi app).
typedef void (*ok_status_cb)(bool vn_on);
void ok_set_status_callback(ok_status_cb cb);

// Swift báo app frontmost đổi.
void ok_notify_frontmost_app(const char *bundle_id);

// Chuyển mã: 0 = Unicode, 1 = VNI-Windows, 2 = TCVN3.
char *ok_convert(const char *text, int from, int to);

void ok_str_free(char *p);

#ifdef __cplusplus
}
#endif

#endif // OREOKEY_H
