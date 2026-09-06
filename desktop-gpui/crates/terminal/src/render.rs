//! Terminal rendering engine for GPUI.
//!
//! Handles cell layout, background coalescing, text run batching, cursor drawing,
//! and selection highlights.

use crate::colors::ColorPalette;
use crate::event::GpuiEventProxy;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
use alacritty_terminal::term::cell::{Cell, Flags};
use alacritty_terminal::term::{Term, TermMode};
use alacritty_terminal::vte::ansi::CursorShape;
use gpui::{
    quad, transparent_black, App, Bounds, Edges, Font, FontFeatures, FontStyle, FontWeight, Hsla,
    Pixels, Point, SharedString, Size, TextAlign, TextRun, UnderlineStyle, Window, px,
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

        // 2. Iterate visible lines
        for line_idx in 0..num_lines {
            let line = Line(line_idx as i32);

            let cells: Vec<(usize, Cell)> = (0..num_cols)
                .map(|col_idx| {
                    let col = Column(col_idx);
                    let pt = AlacPoint::new(line, col);
                    let cell = grid[pt].clone();
                    (col_idx, cell)
                })
                .collect();

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

            // Paint selection highlight quads
            if let Some(ref sel) = selection_range {
                let mut sel_start: Option<usize> = None;
                for col in 0..num_cols {
                    let pt = AlacPoint::new(line, Column(col));
                    if sel.contains(pt) {
                        if sel_start.is_none() {
                            sel_start = Some(col);
                        }
                    } else if let Some(start) = sel_start.take() {
                        let x = origin.x + self.cell_width * (start as f32);
                        let y = origin.y + self.cell_height * (line_idx as f32);
                        let width = self.cell_width * ((col - start) as f32);
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
                if let Some(start) = sel_start {
                    let x = origin.x + self.cell_width * (start as f32);
                    let y = origin.y + self.cell_height * (line_idx as f32);
                    let width = self.cell_width * ((num_cols - start) as f32);
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

        // 3. Paint cursor
        let cursor_point = grid.cursor.point;
        let vi_mode = term.mode().contains(TermMode::VI);
        let show_cursor = vi_mode || term.mode().contains(TermMode::SHOW_CURSOR);

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
    use alacritty_terminal::vte::ansi::Color;

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
