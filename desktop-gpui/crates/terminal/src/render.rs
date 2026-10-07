//! Terminal rendering engine for GPUI.
//!
//! Handles cell layout, background coalescing, text run batching, cursor drawing,
//! and selection highlights.

use crate::colors::ColorPalette;
use crate::event::GpuiEventProxy;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
use alacritty_terminal::selection::SelectionRange;
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::vte::ansi::CursorShape;
use gpui::{
    px, quad, transparent_black, App, Bounds, Edges, Font, FontFeatures, FontStyle, FontWeight,
    Hsla, Pixels, Point, SharedString, Size, TextAlign, TextRun, UnderlineStyle, Window,
};

/// Dimensions of a single character cell in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellDimensions {
    pub width: Pixels,
    pub height: Pixels,
}

/// A batched run of adjacent characters sharing identical styling.
#[derive(Debug, Clone, PartialEq)]
pub struct BatchedTextRun {
    pub text: String,
    pub start_col: usize,
    pub row: usize,
    pub fg_color: Hsla,
    pub bg_color: Hsla,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

/// Coalesced background rectangle covering one or more contiguous columns in a row.
#[derive(Debug, Clone, PartialEq)]
pub struct BackgroundRect {
    pub start_col: usize,
    pub end_col: usize,
    pub row: usize,
    pub color: Hsla,
}

impl BackgroundRect {
    /// Return true if this rectangle is adjacent to `other` on the same row with identical color.
    pub fn can_merge_with(&self, other: &Self) -> bool {
        self.row == other.row && self.color == other.color && self.end_col == other.start_col
    }
}

/// Terminal renderer with font caching and cell metrics.
#[derive(Clone, Debug)]
pub struct TerminalRenderer {
    pub font_family: String,
    pub font_size: Pixels,
    pub cell_width: Pixels,
    pub cell_height: Pixels,
    pub line_height_multiplier: f32,
    pub palette: ColorPalette,
}

impl TerminalRenderer {
    /// Construct a new terminal renderer with font and color palette settings.
    pub fn new(
        font_family: String,
        font_size: Pixels,
        line_height_multiplier: f32,
        palette: ColorPalette,
    ) -> Self {
        // Initial estimate before first font measurement pass
        let cell_width = font_size * 0.6;
        let cell_height = font_size * line_height_multiplier;

        Self {
            font_family,
            font_size,
            cell_width,
            cell_height,
            line_height_multiplier,
            palette,
        }
    }

    /// Return measured character cell dimensions.
    pub fn cell_dimensions(&self) -> CellDimensions {
        CellDimensions {
            width: self.cell_width,
            height: self.cell_height,
        }
    }

    /// Measure actual font metrics from the GPUI text system using 'M'.
    pub fn measure_cell(&mut self, window: &mut Window) {
        let font = Font {
            family: self.font_family.clone().into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        };

        let text_run = TextRun {
            len: 1,
            font,
            color: gpui::black(),
            background_color: None,
            underline: None,
            strikethrough: None,
        };

        let shaped = window
            .text_system()
            .shape_line("M".into(), self.font_size, &[text_run], None);

        if shaped.width > px(0.0) {
            self.cell_width = shaped.width;
        }

        let line_height = shaped.ascent + shaped.descent;
        if line_height > px(0.0) {
            self.cell_height = line_height * self.line_height_multiplier;
        }
    }

    /// Layout row cells into coalesced background rectangles and batched text runs.
    pub fn layout_row(
        &self,
        row: usize,
        cells: impl Iterator<Item = (usize, Cell)>,
    ) -> (Vec<BackgroundRect>, Vec<BatchedTextRun>) {
        let mut backgrounds = Vec::new();
        let mut text_runs = Vec::new();

        let mut current_run: Option<BatchedTextRun> = None;
        let mut current_bg: Option<BackgroundRect> = None;

        for (col, cell) in cells {
            if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                continue;
            }

            let mut fg_color = self.palette.resolve(&cell.fg);
            let mut bg_color = self.palette.resolve(&cell.bg);

            if cell.flags.contains(Flags::INVERSE) {
                std::mem::swap(&mut fg_color, &mut bg_color);
            }
            if cell.flags.contains(Flags::DIM) {
                fg_color.a *= 0.6;
            }
            if cell.flags.contains(Flags::HIDDEN) {
                fg_color = bg_color;
            }

            let bold = cell.flags.contains(Flags::BOLD);
            let italic = cell.flags.contains(Flags::ITALIC);
            let underline = cell.flags.contains(Flags::UNDERLINE);

            let ch = if cell.c == '\0' { ' ' } else { cell.c };

            // Background quad accumulation
            if let Some(ref mut bg) = current_bg {
                if bg.color == bg_color && bg.end_col == col {
                    bg.end_col = col + 1;
                } else {
                    backgrounds.push(bg.clone());
                    current_bg = Some(BackgroundRect {
                        start_col: col,
                        end_col: col + 1,
                        row,
                        color: bg_color,
                    });
                }
            } else {
                current_bg = Some(BackgroundRect {
                    start_col: col,
                    end_col: col + 1,
                    row,
                    color: bg_color,
                });
            }

            // Text run batching
            if let Some(ref mut run) = current_run {
                if run.fg_color == fg_color
                    && run.bg_color == bg_color
                    && run.bold == bold
                    && run.italic == italic
                    && run.underline == underline
                {
                    run.text.push(ch);
                } else {
                    text_runs.push(run.clone());
                    current_run = Some(BatchedTextRun {
                        text: ch.to_string(),
                        start_col: col,
                        row,
                        fg_color,
                        bg_color,
                        bold,
                        italic,
                        underline,
                    });
                }
            } else {
                current_run = Some(BatchedTextRun {
                    text: ch.to_string(),
                    start_col: col,
                    row,
                    fg_color,
                    bg_color,
                    bold,
                    italic,
                    underline,
                });
            }
        }

        if let Some(run) = current_run {
            text_runs.push(run);
        }
        if let Some(bg) = current_bg {
            backgrounds.push(bg);
        }

        let merged_backgrounds = self.merge_backgrounds(backgrounds);
        (merged_backgrounds, text_runs)
    }

    /// Merge adjacent background rectangles of identical color.
    pub fn merge_backgrounds(&self, mut rects: Vec<BackgroundRect>) -> Vec<BackgroundRect> {
        if rects.is_empty() {
            return rects;
        }

        let mut merged = Vec::new();
        let mut current = rects.remove(0);

        for rect in rects {
            if current.can_merge_with(&rect) {
                current.end_col = rect.end_col;
            } else {
                merged.push(current);
                current = rect;
            }
        }

        merged.push(current);
        merged
    }

    /// Whether the cell renders nothing visible: a plain space on the default
    /// background, with no decoration that would make the blank itself
    /// visible (inverse block, underline, colored background).
    pub fn is_blank_cell(&self, cell: &Cell) -> bool {
        if cell.c != ' ' && cell.c != '\0' {
            return false;
        }
        if cell.flags.intersects(Flags::INVERSE | Flags::UNDERLINE) {
            return false;
        }
        self.palette.resolve(&cell.bg) == self.palette.background
    }

    /// Exclusive end column of a row's visible content, i.e. one past the last
    /// non-blank cell (0 for an empty row). Cells past it are empty pane area.
    pub fn content_end_exclusive(&self, cells: &[(usize, Cell)]) -> usize {
        cells
            .iter()
            .filter(|(_, cell)| !self.is_blank_cell(cell))
            .map(|(col, _)| col + 1)
            .max()
            .unwrap_or(0)
    }

    /// Contiguous selected column ranges on one grid line, each clamped to the
    /// row's `content_end` (exclusive). A text selection highlights only the
    /// row's content: cells past the last visible glyph are empty pane area,
    /// `selection_text` never copies them, and a span made entirely of blank
    /// cells (empty pane area) disappears instead of flooding the row.
    pub fn selection_row_spans(
        &self,
        sel: &SelectionRange,
        line: Line,
        num_cols: usize,
        content_end: usize,
    ) -> Vec<(usize, usize)> {
        let mut spans = Vec::new();
        let mut start_col: Option<usize> = None;

        for col in 0..=num_cols {
            let contained = col < num_cols && sel.contains(AlacPoint::new(line, Column(col)));
            if contained {
                if start_col.is_none() {
                    start_col = Some(col);
                }
            } else if let Some(start) = start_col.take() {
                let end = col.min(content_end);
                if end > start {
                    spans.push((start, end));
                }
            }
        }

        spans
    }

    /// Paint full terminal content onto the GPUI window.
    pub fn paint(
        &self,
        bounds: Bounds<Pixels>,
        padding: Edges<Pixels>,
        term: &Term<GpuiEventProxy>,
        focused: bool,
        window: &mut Window,
        cx: &mut App,
    ) {
        let grid = term.grid();
        let num_lines = grid.screen_lines();
        let num_cols = grid.columns();
        // Scrollback: `display_offset` is how many history lines the viewport is
        // scrolled back by. `grid[Line(n)]` addresses live-screen rows, so the
        // row we read for screen row `i` is `i - display_offset` (negative lines
        // index into history). Without this the viewport never moved even though
        // `scroll_display` changed the offset.
        let display_offset = grid.display_offset() as i32;

        // 1. Paint default background covering the full element bounds
        window.paint_quad(quad(
            bounds,
            px(0.0),
            self.palette.background,
            Edges::default(),
            transparent_black(),
            Default::default(),
        ));

        let origin = Point {
            x: bounds.origin.x + padding.left,
            y: bounds.origin.y + padding.top,
        };

        let selection_range = term.selection.as_ref().and_then(|s| s.to_range(term));

        let base_height = self.cell_height / self.line_height_multiplier;
        let vertical_offset = (self.cell_height - base_height) / 2.0;

        // 2. Iterate visible lines (screen rows; `line` is the grid line shown
        // at that row, i.e. shifted back into history when scrolled).
        for line_idx in 0..num_lines {
            let line = Line(line_idx as i32 - display_offset);

            let cells: Vec<(usize, Cell)> = (0..num_cols)
                .map(|col_idx| {
                    let col = Column(col_idx);
                    let pt = AlacPoint::new(line, col);
                    let cell = grid[pt].clone();
                    (col_idx, cell)
                })
                .collect();

            let content_end = match &selection_range {
                // Block selections keep their rectangle shape; stream and
                // line selections stop at the row's content end.
                Some(sel) if !sel.is_block => self.content_end_exclusive(&cells),
                _ => num_cols,
            };
            let (backgrounds, text_runs) = self.layout_row(line_idx, cells.into_iter());

            // Paint non-default background quads
            for bg in backgrounds {
                if bg.color == self.palette.background {
                    continue;
                }

                let x = origin.x + self.cell_width * (bg.start_col as f32);
                let y = origin.y + self.cell_height * (bg.row as f32);
                let width = self.cell_width * ((bg.end_col - bg.start_col) as f32);

                let rect = Bounds {
                    origin: Point { x, y },
                    size: Size {
                        width,
                        height: self.cell_height,
                    },
                };

                window.paint_quad(quad(
                    rect,
                    px(0.0),
                    bg.color,
                    Edges::default(),
                    transparent_black(),
                    Default::default(),
                ));
            }

            // Paint selection highlight quads, clamped to the row's content
            // end so the empty pane area right of the text is never covered.
            if let Some(ref sel) = selection_range {
                for (start, end) in self.selection_row_spans(sel, line, num_cols, content_end) {
                    let x = origin.x + self.cell_width * (start as f32);
                    let y = origin.y + self.cell_height * (line_idx as f32);
                    let width = self.cell_width * ((end - start) as f32);
                    let rect = Bounds {
                        origin: Point { x, y },
                        size: Size {
                            width,
                            height: self.cell_height,
                        },
                    };
                    window.paint_quad(quad(
                        rect,
                        px(0.0),
                        self.palette.selection,
                        Edges::default(),
                        transparent_black(),
                        Default::default(),
                    ));
                }
            }

            // Paint text runs
            for run in text_runs {
                if run.text.trim().is_empty() {
                    continue;
                }

                let x = origin.x + self.cell_width * (run.start_col as f32);
                let y = origin.y + self.cell_height * (run.row as f32) + vertical_offset;

                let font = Font {
                    family: self.font_family.clone().into(),
                    features: FontFeatures::default(),
                    fallbacks: None,
                    weight: if run.bold {
                        FontWeight::BOLD
                    } else {
                        FontWeight::NORMAL
                    },
                    style: if run.italic {
                        FontStyle::Italic
                    } else {
                        FontStyle::Normal
                    },
                };

                let text_run = TextRun {
                    len: run.text.len(),
                    font,
                    color: run.fg_color,
                    background_color: None,
                    underline: if run.underline {
                        Some(UnderlineStyle {
                            thickness: px(1.0),
                            color: Some(run.fg_color),
                            wavy: false,
                        })
                    } else {
                        None
                    },
                    strikethrough: None,
                };

                let text: SharedString = run.text.into();
                let shaped = window
                    .text_system()
                    .shape_line(text, self.font_size, &[text_run], None);

                let _ = shaped.paint(
                    Point { x, y },
                    self.cell_height,
                    TextAlign::Left,
                    None,
                    window,
                    cx,
                );
            }
        }

        // 3. Paint cursor (only at the live screen: while the viewport is
        // scrolled back the cursor row is not on screen, so it is hidden —
        // standard terminal behaviour).
        let cursor_point = grid.cursor.point;
        let vi_mode = term.mode().contains(TermMode::VI);
        let show_cursor = display_offset == 0
            && (vi_mode || term.mode().contains(TermMode::SHOW_CURSOR));

        if show_cursor && cursor_point.line.0 >= 0 && (cursor_point.line.0 as usize) < num_lines {
            let cursor_x = origin.x + self.cell_width * (cursor_point.column.0 as f32);
            let cursor_y = origin.y + self.cell_height * (cursor_point.line.0 as f32);
            let cursor_color = self.palette.cursor;
            let shape = term.cursor_style().shape;

            if focused {
                match shape {
                    CursorShape::Hidden => {}
                    CursorShape::Beam => {
                        let beam_bounds = Bounds {
                            origin: Point {
                                x: cursor_x,
                                y: cursor_y,
                            },
                            size: Size {
                                width: px(2.0),
                                height: self.cell_height,
                            },
                        };
                        window.paint_quad(quad(
                            beam_bounds,
                            px(0.0),
                            cursor_color,
                            Edges::default(),
                            transparent_black(),
                            Default::default(),
                        ));
                    }
                    CursorShape::Underline => {
                        let ul_bounds = Bounds {
                            origin: Point {
                                x: cursor_x,
                                y: cursor_y + self.cell_height - px(2.0),
                            },
                            size: Size {
                                width: self.cell_width,
                                height: px(2.0),
                            },
                        };
                        window.paint_quad(quad(
                            ul_bounds,
                            px(0.0),
                            cursor_color,
                            Edges::default(),
                            transparent_black(),
                            Default::default(),
                        ));
                    }
                    CursorShape::HollowBlock => {
                        let hollow_bounds = Bounds {
                            origin: Point {
                                x: cursor_x,
                                y: cursor_y,
                            },
                            size: Size {
                                width: self.cell_width,
                                height: self.cell_height,
                            },
                        };
                        window.paint_quad(quad(
                            hollow_bounds,
                            px(0.0),
                            transparent_black(),
                            Edges::all(px(1.0)),
                            cursor_color,
                            Default::default(),
                        ));
                    }
                    CursorShape::Block => {
                        let block_bounds = Bounds {
                            origin: Point {
                                x: cursor_x,
                                y: cursor_y,
                            },
                            size: Size {
                                width: self.cell_width,
                                height: self.cell_height,
                            },
                        };
                        let mut block_color = cursor_color;
                        block_color.a = 0.75;
                        window.paint_quad(quad(
                            block_bounds,
                            px(0.0),
                            block_color,
                            Edges::default(),
                            transparent_black(),
                            Default::default(),
                        ));
                    }
                }
            } else if shape != CursorShape::Hidden {
                // Hollow block when unfocused
                let hollow_bounds = Bounds {
                    origin: Point {
                        x: cursor_x,
                        y: cursor_y,
                    },
                    size: Size {
                        width: self.cell_width,
                        height: self.cell_height,
                    },
                };
                window.paint_quad(quad(
                    hollow_bounds,
                    px(0.0),
                    transparent_black(),
                    Edges::all(px(1.0)),
                    cursor_color,
                    Default::default(),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::selection::SelectionRange;
    use alacritty_terminal::vte::ansi::Color;

    fn blank_cell() -> Cell {
        Cell::default() // space on default background
    }

    #[test]
    fn test_is_blank_cell() {
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".into(),
            px(14.0),
            1.2,
            ColorPalette::dark_default(),
        );

        // Plain space / empty cell: blank.
        assert!(renderer.is_blank_cell(&blank_cell()));

        let mut nul = Cell::default();
        nul.c = '\0';
        assert!(renderer.is_blank_cell(&nul));

        // Any glyph: content.
        let mut text = Cell::default();
        text.c = 'x';
        assert!(!renderer.is_blank_cell(&text));

        // Space with a colored background: content (e.g. powerline segments).
        let mut colored = blank_cell();
        colored.bg = Color::Indexed(1);
        assert!(!renderer.is_blank_cell(&colored));

        // Inverse or underlined space: the blank itself is visible.
        let mut inverse = blank_cell();
        inverse.flags.insert(Flags::INVERSE);
        assert!(!renderer.is_blank_cell(&inverse));

        let mut underlined = blank_cell();
        underlined.flags.insert(Flags::UNDERLINE);
        assert!(!renderer.is_blank_cell(&underlined));
    }

    #[test]
    fn test_content_end_exclusive() {
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".into(),
            px(14.0),
            1.2,
            ColorPalette::dark_default(),
        );

        let mut hi = Cell::default();
        hi.c = 'H';
        let cells = vec![
            (0, hi.clone()),
            (1, blank_cell()),
            (2, blank_cell()),
            (3, blank_cell()),
        ];
        assert_eq!(renderer.content_end_exclusive(&cells), 1);

        // Trailing colored blanks still count as content.
        let mut block = blank_cell();
        block.bg = Color::Indexed(4);
        let cells = vec![(0, hi.clone()), (1, block), (2, blank_cell())];
        assert_eq!(renderer.content_end_exclusive(&cells), 2);

        // Empty row: nothing to select.
        let cells = vec![(0, blank_cell()), (1, blank_cell())];
        assert_eq!(renderer.content_end_exclusive(&cells), 0);
    }

    #[test]
    fn test_selection_row_spans_clamp_to_content_end() {
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".into(),
            px(14.0),
            1.2,
            ColorPalette::dark_default(),
        );

        // Stream selection spanning rows 0..2 across all 80 columns: on a row
        // whose content ends at column 10, the highlight must stop there.
        let sel = SelectionRange::new(
            AlacPoint::new(Line(0), Column(0)),
            AlacPoint::new(Line(2), Column(79)),
            false,
        );
        let spans = renderer.selection_row_spans(&sel, Line(1), 80, 10);
        assert_eq!(spans, vec![(0, 10)]);

        // A fully blank row inside the selection: no highlight at all.
        let spans = renderer.selection_row_spans(&sel, Line(1), 80, 0);
        assert_eq!(spans, Vec::new());

        // Content end beyond the selection end: the selection end wins.
        let sel = SelectionRange::new(
            AlacPoint::new(Line(1), Column(0)),
            AlacPoint::new(Line(1), Column(5)),
            false,
        );
        let spans = renderer.selection_row_spans(&sel, Line(1), 80, 40);
        assert_eq!(spans, vec![(0, 6)]);

        // Span starting past the content end (drag began in the empty area):
        // nothing is highlighted on that row.
        let sel = SelectionRange::new(
            AlacPoint::new(Line(1), Column(30)),
            AlacPoint::new(Line(2), Column(79)),
            false,
        );
        let spans = renderer.selection_row_spans(&sel, Line(1), 80, 10);
        assert_eq!(spans, Vec::new());
    }

    #[test]
    fn test_renderer_creation() {
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".into(),
            px(14.0),
            1.2,
            ColorPalette::dark_default(),
        );
        assert_eq!(renderer.font_family, "JetBrains Mono");
        assert_eq!(renderer.font_size, px(14.0));
        let dims = renderer.cell_dimensions();
        assert!(dims.width > px(0.0));
        assert!(dims.height > px(0.0));
    }

    #[test]
    fn test_merge_backgrounds() {
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".into(),
            px(14.0),
            1.2,
            ColorPalette::dark_default(),
        );

        let c1 = gpui::red();
        let c2 = gpui::blue();

        let rects = vec![
            BackgroundRect {
                start_col: 0,
                end_col: 5,
                row: 0,
                color: c1,
            },
            BackgroundRect {
                start_col: 5,
                end_col: 10,
                row: 0,
                color: c1,
            },
            BackgroundRect {
                start_col: 10,
                end_col: 15,
                row: 0,
                color: c2,
            },
        ];

        let merged = renderer.merge_backgrounds(rects);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].start_col, 0);
        assert_eq!(merged[0].end_col, 10);
        assert_eq!(merged[0].color, c1);
        assert_eq!(merged[1].start_col, 10);
        assert_eq!(merged[1].end_col, 15);
        assert_eq!(merged[1].color, c2);
    }

    #[test]
    fn test_layout_row_batching() {
        let renderer = TerminalRenderer::new(
            "JetBrains Mono".into(),
            px(14.0),
            1.2,
            ColorPalette::dark_default(),
        );

        let mut c1 = Cell::default();
        c1.c = 'H';
        c1.fg = Color::Indexed(1);

        let mut c2 = Cell::default();
        c2.c = 'i';
        c2.fg = Color::Indexed(1);

        let mut c3 = Cell::default();
        c3.c = '!';
        c3.fg = Color::Indexed(2);

        let cells = vec![(0, c1), (1, c2), (2, c3)];
        let (_bg, text_runs) = renderer.layout_row(0, cells.into_iter());

        assert_eq!(text_runs.len(), 2);
        assert_eq!(text_runs[0].text, "Hi");
        assert_eq!(text_runs[0].start_col, 0);
        assert_eq!(text_runs[1].text, "!");
        assert_eq!(text_runs[1].start_col, 2);
    }
}
