#ifndef MOZUI_IOS_DEMO_FFI_H
#define MOZUI_IOS_DEMO_FFI_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct MozuiIosDemo MozuiIosDemo;
typedef MozuiIosDemo *MozuiIosDemoRef;

typedef struct MozuiIosHostMetrics {
    float bounds_width;
    float bounds_height;
    float visible_x;
    float visible_y;
    float visible_width;
    float visible_height;
    float scale_factor;
    int32_t appearance;
} MozuiIosHostMetrics;

enum {
    MOZUI_IOS_APPEARANCE_LIGHT = 0,
    MOZUI_IOS_APPEARANCE_DARK = 1,
};

enum {
    MOZUI_IOS_TOUCH_BEGAN = 0,
    MOZUI_IOS_TOUCH_MOVED = 1,
    MOZUI_IOS_TOUCH_ENDED = 2,
    MOZUI_IOS_TOUCH_CANCELLED = 3,
};

MozuiIosDemoRef mozui_ios_demo_new(void);
void mozui_ios_demo_free(MozuiIosDemoRef demo);
bool mozui_ios_demo_attach_view(MozuiIosDemoRef demo, void *ui_view, void *ui_view_controller);
void mozui_ios_demo_detach_view(MozuiIosDemoRef demo);
void mozui_ios_demo_update_metrics(MozuiIosDemoRef demo, struct MozuiIosHostMetrics metrics);
void mozui_ios_demo_handle_touch(MozuiIosDemoRef demo, float x, float y, int32_t phase);
void mozui_ios_demo_request_frame(MozuiIosDemoRef demo);
void mozui_ios_demo_handle_scroll(
    MozuiIosDemoRef demo,
    float x,
    float y,
    float dx,
    float dy,
    int32_t phase
);
bool mozui_ios_demo_insert_text(MozuiIosDemoRef demo, const char *text);
bool mozui_ios_demo_delete_backward(MozuiIosDemoRef demo);
bool mozui_ios_demo_accepts_text_input(MozuiIosDemoRef demo);
void mozui_ios_demo_enter_foreground(MozuiIosDemoRef demo);
void mozui_ios_demo_enter_background(MozuiIosDemoRef demo);
const char *mozui_ios_demo_last_error(MozuiIosDemoRef demo);

#ifdef __cplusplus
}
#endif

#endif
