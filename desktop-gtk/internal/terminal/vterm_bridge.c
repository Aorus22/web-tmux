#include "vterm_bridge.h"
#include <stdlib.h>
#include <string.h>

extern void goVTermOutput(uintptr_t handle, void *data, size_t len);
extern void goVTermDamage(uintptr_t handle);
extern int goVTermScrollbackPush(uintptr_t handle, void *cells, int count);
extern void goVTermScrollbackClear(uintptr_t handle);

struct VTBridge {
  VTerm *term;
  VTermState *state;
  VTermScreen *screen;
  uintptr_t handle;
  VTermPos cursor;
  int cursor_visible;
  int cursor_shape;
  int mouse_mode;
  VTDamage damage;
};

static void damage_reset(VTBridge *bridge) {
  bridge->damage.valid = 0;
  bridge->damage.full = 0;
}

/* Union one vterm rect into the accumulated damage. A pending FULL stays FULL;
 * two partial rects expand to cover both. */
static void damage_union_rect(VTBridge *bridge, VTermRect rect) {
  if (bridge->damage.full) return;
  if (!bridge->damage.valid) {
    bridge->damage.valid = 1;
    bridge->damage.r0 = rect.start_row;
    bridge->damage.c0 = rect.start_col;
    bridge->damage.r1 = rect.end_row;
    bridge->damage.c1 = rect.end_col;
    return;
  }
  if (rect.start_row < bridge->damage.r0) bridge->damage.r0 = rect.start_row;
  if (rect.start_col < bridge->damage.c0) bridge->damage.c0 = rect.start_col;
  if (rect.end_row > bridge->damage.r1) bridge->damage.r1 = rect.end_row;
  if (rect.end_col > bridge->damage.c1) bridge->damage.c1 = rect.end_col;
}

static void damage_mark_full(VTBridge *bridge) {
  bridge->damage.valid = 1;
  bridge->damage.full = 1;
}

static void output_cb(const char *data, size_t len, void *user) {
  VTBridge *bridge = user;
  goVTermOutput(bridge->handle, (void *)data, len);
}

static int damage_cb(VTermRect rect, void *user) {
  VTBridge *bridge = user;
  damage_union_rect(bridge, rect);
  goVTermDamage(bridge->handle);
  return 1;
}

static int movecursor_cb(VTermPos pos, VTermPos oldpos, int visible, void *user) {
  (void)oldpos;
  VTBridge *bridge = user;
  bridge->cursor = pos;
  bridge->cursor_visible = visible;
  goVTermDamage(bridge->handle);
  return 1;
}

static int settermprop_cb(VTermProp prop, VTermValue *value, void *user) {
  VTBridge *bridge = user;
  if (prop == VTERM_PROP_CURSORVISIBLE) bridge->cursor_visible = value->boolean;
  if (prop == VTERM_PROP_CURSORSHAPE) bridge->cursor_shape = value->number;
  if (prop == VTERM_PROP_MOUSE) bridge->mouse_mode = value->number;
  goVTermDamage(bridge->handle);
  return 1;
}

static int bell_cb(void *user) {
  VTBridge *bridge = user;
  goVTermDamage(bridge->handle);
  return 1;
}

static int resize_cb(int rows, int cols, void *user) {
  (void)rows; (void)cols;
  VTBridge *bridge = user;
  damage_mark_full(bridge);
  goVTermDamage(bridge->handle);
  return 1;
}

static int sb_pushline_cb(int cols, const VTermScreenCell *cells, void *user) {
  VTBridge *bridge = user;
  // A scroll shifts every viewport row, so the whole screen is stale.
  damage_mark_full(bridge);
  return goVTermScrollbackPush(bridge->handle, (void *)cells, cols);
}

static int sb_popline_cb(int cols, VTermScreenCell *cells, void *user) {
  (void)cols; (void)cells; (void)user;
  return 0;
}

static int sb_clear_cb(void *user) {
  VTBridge *bridge = user;
  // Dropping history shifts which global row each viewport row shows.
  damage_mark_full(bridge);
  goVTermScrollbackClear(bridge->handle);
  return 1;
}

static const VTermScreenCallbacks callbacks = {
  .damage = damage_cb,
  .moverect = NULL,
  .movecursor = movecursor_cb,
  .settermprop = settermprop_cb,
  .bell = bell_cb,
  .resize = resize_cb,
  .sb_pushline = sb_pushline_cb,
  .sb_popline = sb_popline_cb,
  .sb_clear = sb_clear_cb,
};

VTBridge *vt_bridge_new(int rows, int cols, uintptr_t handle) {
  VTBridge *bridge = calloc(1, sizeof(VTBridge));
  if (!bridge) return NULL;
  bridge->term = vterm_new(rows, cols);
  if (!bridge->term) { free(bridge); return NULL; }
  bridge->handle = handle;
  bridge->cursor_visible = 1;
  bridge->cursor_shape = VTERM_PROP_CURSORSHAPE_BLOCK;
  vterm_set_utf8(bridge->term, 1);
  vterm_output_set_callback(bridge->term, output_cb, bridge);
  bridge->state = vterm_obtain_state(bridge->term);
  bridge->screen = vterm_obtain_screen(bridge->term);
  vterm_screen_set_callbacks(bridge->screen, &callbacks, bridge);
  vterm_screen_enable_altscreen(bridge->screen, 1);
  vterm_screen_enable_reflow(bridge->screen, true);
  vterm_screen_set_damage_merge(bridge->screen, VTERM_DAMAGE_SCROLL);
  vterm_screen_reset(bridge->screen, 1);
  return bridge;
}

void vt_bridge_free(VTBridge *bridge) {
  if (!bridge) return;
  vterm_free(bridge->term);
  free(bridge);
}

void vt_bridge_feed(VTBridge *bridge, const char *data, size_t len) {
  if (!bridge || !data || !len) return;
  vterm_input_write(bridge->term, data, len);
  vterm_screen_flush_damage(bridge->screen);
}

void vt_bridge_reset(VTBridge *bridge, int hard) {
  if (!bridge) return;
  bridge->cursor = (VTermPos){.row=0, .col=0};
  bridge->cursor_visible = 1;
  bridge->cursor_shape = VTERM_PROP_CURSORSHAPE_BLOCK;
  bridge->mouse_mode = 0;
  vterm_screen_reset(bridge->screen, hard);
  vterm_screen_flush_damage(bridge->screen);
}

void vt_bridge_resize(VTBridge *bridge, int rows, int cols) {
  if (!bridge || rows < 1 || cols < 2) return;
  vterm_set_size(bridge->term, rows, cols);
  vterm_screen_flush_damage(bridge->screen);
}

static void convert_cell(VTBridge *bridge, const VTermScreenCell *source, VTCell *cell) {
  memset(cell, 0, sizeof(*cell));
  memcpy(cell->chars, source->chars, sizeof(cell->chars));
  cell->width = source->width;
  cell->bold = source->attrs.bold;
  cell->underline = source->attrs.underline;
  cell->italic = source->attrs.italic;
  cell->blink = source->attrs.blink;
  cell->reverse = source->attrs.reverse;
  cell->conceal = source->attrs.conceal;
  cell->strike = source->attrs.strike;
  VTermColor fg = source->fg;
  VTermColor bg = source->bg;
  vterm_screen_convert_color_to_rgb(bridge->screen, &fg);
  vterm_screen_convert_color_to_rgb(bridge->screen, &bg);
  cell->fg_r = fg.rgb.red; cell->fg_g = fg.rgb.green; cell->fg_b = fg.rgb.blue;
  cell->bg_r = bg.rgb.red; cell->bg_g = bg.rgb.green; cell->bg_b = bg.rgb.blue;
}

int vt_bridge_cell(VTBridge *bridge, int row, int col, VTCell *cell) {
  if (!bridge || !cell) return 0;
  VTermScreenCell source;
  if (!vterm_screen_get_cell(bridge->screen, (VTermPos){.row=row,.col=col}, &source)) return 0;
  convert_cell(bridge, &source, cell);
  return 1;
}

void vt_bridge_cell_from_array(VTBridge *bridge, const void *cells, int index, VTCell *cell) {
  const VTermScreenCell *line = cells;
  convert_cell(bridge, &line[index], cell);
}

void vt_bridge_cursor(VTBridge *bridge, int *row, int *col, int *visible, int *shape) {
  if (!bridge) return;
  *row = bridge->cursor.row; *col = bridge->cursor.col;
  *visible = bridge->cursor_visible; *shape = bridge->cursor_shape;
}

int vt_bridge_mouse_mode(VTBridge *bridge) { return bridge ? bridge->mouse_mode : 0; }
void vt_bridge_key(VTBridge *bridge, int key, int modifiers) { if (bridge) vterm_keyboard_key(bridge->term, key, modifiers); }
void vt_bridge_unichar(VTBridge *bridge, uint32_t rune, int modifiers) { if (bridge) vterm_keyboard_unichar(bridge->term, rune, modifiers); }
void vt_bridge_paste_start(VTBridge *bridge) { if (bridge) vterm_keyboard_start_paste(bridge->term); }
void vt_bridge_paste_end(VTBridge *bridge) { if (bridge) vterm_keyboard_end_paste(bridge->term); }
void vt_bridge_mouse_move(VTBridge *bridge, int row, int col, int modifiers) { if (bridge) vterm_mouse_move(bridge->term,row,col,modifiers); }
void vt_bridge_mouse_button(VTBridge *bridge, int button, int pressed, int modifiers) { if (bridge) vterm_mouse_button(bridge->term,button,pressed != 0,modifiers); }

void vt_bridge_set_default_colors(VTBridge *bridge, uint8_t fr, uint8_t fg, uint8_t fb,
                                  uint8_t br, uint8_t bg, uint8_t bb) {
  if (!bridge) return;
  VTermColor f, b; vterm_color_rgb(&f,fr,fg,fb); vterm_color_rgb(&b,br,bg,bb);
  vterm_state_set_default_colors(bridge->state,&f,&b);
}

void vt_bridge_set_palette(VTBridge *bridge, int index, uint8_t r, uint8_t g, uint8_t b) {
  if (!bridge || index < 0 || index > 255) return;
  VTermColor color; vterm_color_rgb(&color,r,g,b);
  vterm_state_set_palette_color(bridge->state,index,&color);
}

void vt_bridge_take_damage(VTBridge *bridge, VTDamage *damage) {
  if (!bridge) return;
  if (damage) *damage = bridge->damage;
  damage_reset(bridge);
}
