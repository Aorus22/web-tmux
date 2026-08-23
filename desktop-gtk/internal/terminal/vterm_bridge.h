#ifndef TMUX_GUI_VTERM_BRIDGE_H
#define TMUX_GUI_VTERM_BRIDGE_H

#include <stddef.h>
#include <stdint.h>
#include <vterm.h>

typedef struct VTBridge VTBridge;

typedef struct {
  uint32_t chars[VTERM_MAX_CHARS_PER_CELL];
  int width;
  unsigned int bold;
  unsigned int underline;
  unsigned int italic;
  unsigned int blink;
  unsigned int reverse;
  unsigned int conceal;
  unsigned int strike;
  uint8_t fg_r, fg_g, fg_b;
  uint8_t bg_r, bg_g, bg_b;
} VTCell;

VTBridge *vt_bridge_new(int rows, int cols, uintptr_t handle);
void vt_bridge_free(VTBridge *bridge);
void vt_bridge_feed(VTBridge *bridge, const char *data, size_t len);
void vt_bridge_reset(VTBridge *bridge, int hard);
void vt_bridge_resize(VTBridge *bridge, int rows, int cols);
int vt_bridge_cell(VTBridge *bridge, int row, int col, VTCell *cell);
void vt_bridge_cursor(VTBridge *bridge, int *row, int *col, int *visible, int *shape);
int vt_bridge_mouse_mode(VTBridge *bridge);
void vt_bridge_key(VTBridge *bridge, int key, int modifiers);
void vt_bridge_unichar(VTBridge *bridge, uint32_t rune, int modifiers);
void vt_bridge_paste_start(VTBridge *bridge);
void vt_bridge_paste_end(VTBridge *bridge);
void vt_bridge_mouse_move(VTBridge *bridge, int row, int col, int modifiers);
void vt_bridge_mouse_button(VTBridge *bridge, int button, int pressed, int modifiers);
void vt_bridge_set_default_colors(VTBridge *bridge, uint8_t fr, uint8_t fg, uint8_t fb,
                                  uint8_t br, uint8_t bg, uint8_t bb);
void vt_bridge_set_palette(VTBridge *bridge, int index, uint8_t r, uint8_t g, uint8_t b);
void vt_bridge_cell_from_array(VTBridge *bridge, const void *cells, int index, VTCell *cell);

#endif

