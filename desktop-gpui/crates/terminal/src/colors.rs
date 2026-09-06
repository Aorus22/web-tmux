use alacritty_terminal::vte::ansi::{Color, NamedColor, Rgb};
use gpui::{rgba, Hsla, Rgba};

/// Palette defining all 16 ANSI colors, special UI colors, and 256-color indexing.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorPalette {
    pub foreground: Hsla,
    pub background: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
    pub ansi: [Hsla; 16],
    indexed: [Hsla; 256],
}

impl Default for ColorPalette {
    fn default() -> Self {
        Self::dark_default()
    }
}

fn rgb_to_hsla(r: u8, g: u8, b: u8) -> Hsla {
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: 1.0,
    }
    .into()
}

fn hex_to_hsla(hex: u32) -> Hsla {
    rgba(hex).into()
}

impl ColorPalette {
    /// Create a standard dark palette matching WebTerm Dark theme.
    pub fn dark_default() -> Self {
        let foreground = hex_to_hsla(0xe4e4e7ff); // zinc-200
        let background = hex_to_hsla(0x18181bff); // zinc-900
        let cursor = hex_to_hsla(0x60a5faff);     // blue-400
        let selection = hex_to_hsla(0x3b82f666);  // blue-500 with alpha

        let ansi = [
            hex_to_hsla(0x27272aff), // Black
            hex_to_hsla(0xef4444ff), // Red
            hex_to_hsla(0x22c55eff), // Green
            hex_to_hsla(0xeab308ff), // Yellow
            hex_to_hsla(0x3b82f6ff), // Blue
            hex_to_hsla(0xa855f7ff), // Magenta
            hex_to_hsla(0x06b6d4ff), // Cyan
            hex_to_hsla(0xd4d4d8ff), // White
            hex_to_hsla(0x52525bff), // Bright Black
            hex_to_hsla(0xf87171ff), // Bright Red
            hex_to_hsla(0x4ade80ff), // Bright Green
            hex_to_hsla(0xfacc15ff), // Bright Yellow
            hex_to_hsla(0x60a5faff), // Bright Blue
            hex_to_hsla(0xc084fcff), // Bright Magenta
            hex_to_hsla(0x22d3eeff), // Bright Cyan
            hex_to_hsla(0xf4f4f5ff), // Bright White
        ];

        let indexed = Self::generate_256_table(&ansi);

        Self {
            foreground,
            background,
            cursor,
            selection,
            ansi,
            indexed,
        }
    }

    /// Create a standard light palette matching WebTerm Light theme.
    pub fn light_default() -> Self {
        let foreground = hex_to_hsla(0x27272aff); // zinc-800
        let background = hex_to_hsla(0xfafafaff); // zinc-50
        let cursor = hex_to_hsla(0x2563ebff);     // blue-600
        let selection = hex_to_hsla(0xbfdbfe88);  // blue-200 with alpha

        let ansi = [
            hex_to_hsla(0x18181bff), // Black
            hex_to_hsla(0xdc2626ff), // Red
            hex_to_hsla(0x16a34aff), // Green
            hex_to_hsla(0xca8a04ff), // Yellow
            hex_to_hsla(0x2563ebff), // Blue
            hex_to_hsla(0x9333eaff), // Magenta
            hex_to_hsla(0x0891b2ff), // Cyan
            hex_to_hsla(0x52525bff), // White
            hex_to_hsla(0x71717aff), // Bright Black
            hex_to_hsla(0xef4444ff), // Bright Red
            hex_to_hsla(0x22c55eff), // Bright Green
            hex_to_hsla(0xeab308ff), // Bright Yellow
            hex_to_hsla(0x3b82f6ff), // Bright Blue
            hex_to_hsla(0xa855f7ff), // Bright Magenta
            hex_to_hsla(0x06b6d4ff), // Bright Cyan
            hex_to_hsla(0x27272aff), // Bright White
        ];

        let indexed = Self::generate_256_table(&ansi);

        Self {
            foreground,
            background,
            cursor,
            selection,
            ansi,
            indexed,
        }
    }

    /// Generate the full 256-color lookup table from the 16 ANSI base colors.
    fn generate_256_table(ansi: &[Hsla; 16]) -> [Hsla; 256] {
        let mut table = [Hsla::default(); 256];

        // 0..15: ANSI colors
        table[..16].copy_from_slice(ansi);

        // 16..231: 6x6x6 color cube
        let steps = [0x00, 0x5f, 0x87, 0xaf, 0xd7, 0xff];
        let mut idx = 16;
        for &r in &steps {
            for &g in &steps {
                for &b in &steps {
                    table[idx] = rgb_to_hsla(r, g, b);
                    idx += 1;
                }
            }
        }

        // 232..255: 24 grayscale steps
        for i in 0..24 {
            let gray = (8 + i * 10) as u8;
            table[idx] = rgb_to_hsla(gray, gray, gray);
            idx += 1;
        }

        table
    }

    /// Resolve an Alacritty Color (Named, Spec RGB, or Indexed) to a GPUI Hsla color.
    pub fn resolve(&self, color: &Color) -> Hsla {
        match color {
            Color::Named(named) => self.resolve_named(named),
            Color::Spec(Rgb { r, g, b }) => rgb_to_hsla(*r, *g, *b),
            Color::Indexed(idx) => self.indexed[*idx as usize],
        }
    }

    /// Resolve a NamedColor from Alacritty.
    pub fn resolve_named(&self, named: &NamedColor) -> Hsla {
        match named {
            NamedColor::Black => self.ansi[0],
            NamedColor::Red => self.ansi[1],
            NamedColor::Green => self.ansi[2],
            NamedColor::Yellow => self.ansi[3],
            NamedColor::Blue => self.ansi[4],
            NamedColor::Magenta => self.ansi[5],
            NamedColor::Cyan => self.ansi[6],
            NamedColor::White => self.ansi[7],
            NamedColor::BrightBlack => self.ansi[8],
            NamedColor::BrightRed => self.ansi[9],
            NamedColor::BrightGreen => self.ansi[10],
            NamedColor::BrightYellow => self.ansi[11],
            NamedColor::BrightBlue => self.ansi[12],
            NamedColor::BrightMagenta => self.ansi[13],
            NamedColor::BrightCyan => self.ansi[14],
            NamedColor::BrightWhite => self.ansi[15],
            NamedColor::Foreground => self.foreground,
            NamedColor::Background => self.background,
            NamedColor::Cursor => self.cursor,
            NamedColor::DimBlack => self.ansi[0],
            NamedColor::DimRed => self.ansi[1],
            NamedColor::DimGreen => self.ansi[2],
            NamedColor::DimYellow => self.ansi[3],
            NamedColor::DimBlue => self.ansi[4],
            NamedColor::DimMagenta => self.ansi[5],
            NamedColor::DimCyan => self.ansi[6],
            NamedColor::DimWhite => self.ansi[7],
            NamedColor::BrightForeground => self.ansi[15],
            NamedColor::DimForeground => self.ansi[7],
        }
    }

    /// Create a palette from raw RGB u32 hex values (0xRRGGBB).
    pub fn from_rgb_u32(
        foreground: u32,
        background: u32,
        cursor: u32,
        selection: u32,
        ansi: [u32; 16],
    ) -> Self {
        let foreground_hsla = hex_to_hsla((foreground << 8) | 0xff);
        let background_hsla = hex_to_hsla((background << 8) | 0xff);
        let cursor_hsla = hex_to_hsla((cursor << 8) | 0xff);
        let selection_hsla = hex_to_hsla((selection << 8) | 0x66);
        let ansi_hsla = ansi.map(|c| hex_to_hsla((c << 8) | 0xff));
        let indexed = Self::generate_256_table(&ansi_hsla);

        Self {
            foreground: foreground_hsla,
            background: background_hsla,
            cursor: cursor_hsla,
            selection: selection_hsla,
            ansi: ansi_hsla,
            indexed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::NamedColor;

    #[test]
    fn test_named_color_resolution() {
        let palette = ColorPalette::dark_default();
        let red = palette.resolve(&Color::Named(NamedColor::Red));
        assert_eq!(red, palette.ansi[1]);
    }

    #[test]
    fn test_truecolor_resolution() {
        let palette = ColorPalette::dark_default();
        let rgb_color = palette.resolve(&Color::Spec(Rgb {
            r: 120,
            g: 200,
            b: 250,
        }));
        let expected: Hsla = Rgba {
            r: 120.0 / 255.0,
            g: 200.0 / 255.0,
            b: 250.0 / 255.0,
            a: 1.0,
        }
        .into();
        assert_eq!(rgb_color, expected);
    }

    #[test]
    fn test_indexed_color_resolution() {
        let palette = ColorPalette::dark_default();
        // Index 1 is Red
        assert_eq!(palette.resolve(&Color::Indexed(1)), palette.ansi[1]);
        // Index 232 is first grayscale step (gray = 8)
        let gray_expected: Hsla = Rgba {
            r: 8.0 / 255.0,
            g: 8.0 / 255.0,
            b: 8.0 / 255.0,
            a: 1.0,
        }
        .into();
        assert_eq!(palette.resolve(&Color::Indexed(232)), gray_expected);
    }
}
