//! Decoding of kitty's unicode placeholder cells (the `U=1` placement mode).
//!
//! A virtual placement has no anchor in the grid. Instead the application
//! prints cells containing U+10EEEE, and each such cell says which image, and
//! which cell of that image, it stands for. The image id travels in the cell's
//! foreground colour and the row/column travel as combining diacritics on the
//! placeholder character, indexed into [`ROWCOLUMN_DIACRITICS`].
//!
//! This module only decodes cells and groups them into runs; the drawing lives
//! in [`super::render_unicode_placeholders`].

use crate::terminal::model::ansi::Color;
use crate::terminal::model::cell::Cell;
use crate::terminal::model::char_or_str::CharOrStr;

/// The character every placeholder cell holds.
pub const PLACEHOLDER_CHAR: char = '\u{10EEEE}';

/// Combining characters that encode a row or column number, in kitty's
/// specified order: a character's index in this table *is* the number it
/// encodes. The table is sorted ascending, so [`diacritic_index`] can binary
/// search it directly instead of keeping a second sorted copy; the
/// `diacritic_table_is_sorted` test holds that invariant.
#[rustfmt::skip]
static ROWCOLUMN_DIACRITICS: [char; 297] = [
    '\u{0305}', '\u{030D}', '\u{030E}', '\u{0310}', '\u{0312}', '\u{033D}', '\u{033E}', '\u{033F}',
    '\u{0346}', '\u{034A}', '\u{034B}', '\u{034C}', '\u{0350}', '\u{0351}', '\u{0352}', '\u{0357}',
    '\u{035B}', '\u{0363}', '\u{0364}', '\u{0365}', '\u{0366}', '\u{0367}', '\u{0368}', '\u{0369}',
    '\u{036A}', '\u{036B}', '\u{036C}', '\u{036D}', '\u{036E}', '\u{036F}', '\u{0483}', '\u{0484}',
    '\u{0485}', '\u{0486}', '\u{0487}', '\u{0592}', '\u{0593}', '\u{0594}', '\u{0595}', '\u{0597}',
    '\u{0598}', '\u{0599}', '\u{059C}', '\u{059D}', '\u{059E}', '\u{059F}', '\u{05A0}', '\u{05A1}',
    '\u{05A8}', '\u{05A9}', '\u{05AB}', '\u{05AC}', '\u{05AF}', '\u{05C4}', '\u{0610}', '\u{0611}',
    '\u{0612}', '\u{0613}', '\u{0614}', '\u{0615}', '\u{0616}', '\u{0617}', '\u{0657}', '\u{0658}',
    '\u{0659}', '\u{065A}', '\u{065B}', '\u{065D}', '\u{065E}', '\u{06D6}', '\u{06D7}', '\u{06D8}',
    '\u{06D9}', '\u{06DA}', '\u{06DB}', '\u{06DC}', '\u{06DF}', '\u{06E0}', '\u{06E1}', '\u{06E2}',
    '\u{06E4}', '\u{06E7}', '\u{06E8}', '\u{06EB}', '\u{06EC}', '\u{0730}', '\u{0732}', '\u{0733}',
    '\u{0735}', '\u{0736}', '\u{073A}', '\u{073D}', '\u{073F}', '\u{0740}', '\u{0741}', '\u{0743}',
    '\u{0745}', '\u{0747}', '\u{0749}', '\u{074A}', '\u{07EB}', '\u{07EC}', '\u{07ED}', '\u{07EE}',
    '\u{07EF}', '\u{07F0}', '\u{07F1}', '\u{07F3}', '\u{0816}', '\u{0817}', '\u{0818}', '\u{0819}',
    '\u{081B}', '\u{081C}', '\u{081D}', '\u{081E}', '\u{081F}', '\u{0820}', '\u{0821}', '\u{0822}',
    '\u{0823}', '\u{0825}', '\u{0826}', '\u{0827}', '\u{0829}', '\u{082A}', '\u{082B}', '\u{082C}',
    '\u{082D}', '\u{0951}', '\u{0953}', '\u{0954}', '\u{0F82}', '\u{0F83}', '\u{0F86}', '\u{0F87}',
    '\u{135D}', '\u{135E}', '\u{135F}', '\u{17DD}', '\u{193A}', '\u{1A17}', '\u{1A75}', '\u{1A76}',
    '\u{1A77}', '\u{1A78}', '\u{1A79}', '\u{1A7A}', '\u{1A7B}', '\u{1A7C}', '\u{1B6B}', '\u{1B6D}',
    '\u{1B6E}', '\u{1B6F}', '\u{1B70}', '\u{1B71}', '\u{1B72}', '\u{1B73}', '\u{1CD0}', '\u{1CD1}',
    '\u{1CD2}', '\u{1CDA}', '\u{1CDB}', '\u{1CE0}', '\u{1DC0}', '\u{1DC1}', '\u{1DC3}', '\u{1DC4}',
    '\u{1DC5}', '\u{1DC6}', '\u{1DC7}', '\u{1DC8}', '\u{1DC9}', '\u{1DCB}', '\u{1DCC}', '\u{1DD1}',
    '\u{1DD2}', '\u{1DD3}', '\u{1DD4}', '\u{1DD5}', '\u{1DD6}', '\u{1DD7}', '\u{1DD8}', '\u{1DD9}',
    '\u{1DDA}', '\u{1DDB}', '\u{1DDC}', '\u{1DDD}', '\u{1DDE}', '\u{1DDF}', '\u{1DE0}', '\u{1DE1}',
    '\u{1DE2}', '\u{1DE3}', '\u{1DE4}', '\u{1DE5}', '\u{1DE6}', '\u{1DFE}', '\u{20D0}', '\u{20D1}',
    '\u{20D4}', '\u{20D5}', '\u{20D6}', '\u{20D7}', '\u{20DB}', '\u{20DC}', '\u{20E1}', '\u{20E7}',
    '\u{20E9}', '\u{20F0}', '\u{2CEF}', '\u{2CF0}', '\u{2CF1}', '\u{2DE0}', '\u{2DE1}', '\u{2DE2}',
    '\u{2DE3}', '\u{2DE4}', '\u{2DE5}', '\u{2DE6}', '\u{2DE7}', '\u{2DE8}', '\u{2DE9}', '\u{2DEA}',
    '\u{2DEB}', '\u{2DEC}', '\u{2DED}', '\u{2DEE}', '\u{2DEF}', '\u{2DF0}', '\u{2DF1}', '\u{2DF2}',
    '\u{2DF3}', '\u{2DF4}', '\u{2DF5}', '\u{2DF6}', '\u{2DF7}', '\u{2DF8}', '\u{2DF9}', '\u{2DFA}',
    '\u{2DFB}', '\u{2DFC}', '\u{2DFD}', '\u{2DFE}', '\u{2DFF}', '\u{A66F}', '\u{A67C}', '\u{A67D}',
    '\u{A6F0}', '\u{A6F1}', '\u{A8E0}', '\u{A8E1}', '\u{A8E2}', '\u{A8E3}', '\u{A8E4}', '\u{A8E5}',
    '\u{A8E6}', '\u{A8E7}', '\u{A8E8}', '\u{A8E9}', '\u{A8EA}', '\u{A8EB}', '\u{A8EC}', '\u{A8ED}',
    '\u{A8EE}', '\u{A8EF}', '\u{A8F0}', '\u{A8F1}', '\u{AAB0}', '\u{AAB2}', '\u{AAB3}', '\u{AAB7}',
    '\u{AAB8}', '\u{AABE}', '\u{AABF}', '\u{AAC1}', '\u{FE20}', '\u{FE21}', '\u{FE22}', '\u{FE23}',
    '\u{FE24}', '\u{FE25}', '\u{FE26}', '\u{10A0F}', '\u{10A38}', '\u{1D185}', '\u{1D186}',
    '\u{1D187}', '\u{1D188}', '\u{1D189}', '\u{1D1AA}', '\u{1D1AB}', '\u{1D1AC}', '\u{1D1AD}',
    '\u{1D242}', '\u{1D243}', '\u{1D244}',
];

/// The number a row/column diacritic encodes, or `None` if `c` isn't one.
pub fn diacritic_index(c: char) -> Option<u32> {
    ROWCOLUMN_DIACRITICS
        .binary_search(&c)
        .ok()
        .map(|index| index as u32)
}

/// Whether this cell stands in for part of a virtually placed image. Cheap
/// enough to call for every cell on every frame.
pub fn is_unicode_placeholder(cell: &Cell) -> bool {
    cell.c == PLACEHOLDER_CHAR
}

/// One decoded placeholder cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceholderCell {
    pub image_id: u32,
    pub placement_id: u32,
    /// Row of the image this cell shows, if the cell spelled it out.
    pub row: Option<u32>,
    /// Column of the image this cell shows, if the cell spelled it out.
    pub col: Option<u32>,
}

/// Decodes a placeholder cell, or returns `None` if this isn't one or its image
/// id can't be recovered.
pub fn parse_placeholder_cell(cell: &Cell) -> Option<PlaceholderCell> {
    if !is_unicode_placeholder(cell) {
        return None;
    }

    let mut image_id = image_id_from_foreground(cell.fg)?;

    // The base character comes first in the grapheme; the rest are the
    // diacritics, in the order they were printed. A cell with no zerowidth
    // characters reports a bare `Char` and so carries no position at all.
    let grapheme = match cell.raw_content() {
        CharOrStr::Str(grapheme) => grapheme,
        CharOrStr::Char(_) => "",
    };
    let mut diacritics = grapheme.chars().skip(1).filter_map(diacritic_index);

    let row = diacritics.next();
    let col = diacritics.next();

    // A third diacritic carries the top byte of a 32-bit image id, which does
    // not fit in a 24-bit colour.
    if let Some(most_significant_byte) = diacritics.next() {
        // Only the first 256 diacritics can encode a byte. A higher index is a
        // malformed cell, and decoding it anyway would silently name a
        // different image.
        if most_significant_byte > 0xff {
            return None;
        }
        image_id |= most_significant_byte << 24;
    }

    Some(PlaceholderCell {
        image_id,
        // Placement ids are carried in the cell's underline colour, which this
        // fork's `Cell` has nowhere to store. Every placeholder therefore reads
        // as placement 0, and the renderer falls back to an image's sole
        // virtual placement when it has exactly one. Images displayed through
        // several simultaneous virtual placements are consequently not
        // distinguishable here.
        placement_id: 0,
        row,
        col,
    })
}

/// Recovers the image id a placeholder cell's foreground colour encodes.
fn image_id_from_foreground(foreground: Color) -> Option<u32> {
    match foreground {
        Color::Indexed(index) => Some(index as u32),
        Color::Spec(color) => {
            Some(((color.r as u32) << 16) | ((color.g as u32) << 8) | color.b as u32)
        }
        // Only the 16 palette colours name an image id; `Foreground`, `Cursor`
        // and friends have no numeric meaning here.
        Color::Named(named) => {
            let index = named.into_color_index();
            (index <= 15).then_some(index as u32)
        }
    }
}

/// A horizontal strip of an image to draw with a single quad: one image, one
/// placement, one image row, and consecutive image columns backed by
/// consecutive grid cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceholderRun {
    pub image_id: u32,
    pub placement_id: u32,
    /// Row of the image to sample.
    pub image_row: u32,
    /// First column of the image to sample.
    pub image_col_start: u32,
    /// Last column of the image to sample, inclusive.
    pub image_col_end: u32,
    /// First grid column the run covers.
    pub col_start: usize,
    /// How many grid cells the run covers.
    pub len: usize,
}

/// Resolved position of the placeholder cell to the left, used to fill in
/// diacritics the application left out.
#[derive(Clone, Copy)]
struct PreviousCell {
    grid_col: usize,
    image_id: u32,
    placement_id: u32,
    row: u32,
    col: u32,
}

/// Groups a row's placeholder cells into runs.
///
/// `cells` must yield `(grid column, cell)` in ascending column order. Gaps in
/// the grid columns break a run, as do changes of image, placement or image
/// row.
///
/// Applications may leave out diacritics for brevity: a cell without a column
/// continues the previous cell's column, and a cell without a row repeats the
/// previous cell's row. Inheritance only applies across immediately adjacent
/// cells of the same placement; anywhere else an omitted number means zero.
pub fn build_runs(cells: impl Iterator<Item = (usize, PlaceholderCell)>) -> Vec<PlaceholderRun> {
    let mut runs = Vec::new();
    let mut current: Option<PlaceholderRun> = None;
    let mut previous: Option<PreviousCell> = None;

    for (grid_col, cell) in cells {
        let inherited = previous.filter(|previous| {
            previous.grid_col + 1 == grid_col
                && previous.image_id == cell.image_id
                && previous.placement_id == cell.placement_id
        });

        let row = cell
            .row
            .or(inherited.map(|previous| previous.row))
            .unwrap_or(0);
        let col = cell
            .col
            .or(inherited.map(|previous| previous.col + 1))
            .unwrap_or(0);

        let extends_current = current.is_some_and(|run| {
            run.image_id == cell.image_id
                && run.placement_id == cell.placement_id
                && run.image_row == row
                && run.col_start + run.len == grid_col
                && run.image_col_end + 1 == col
        });

        match current.as_mut() {
            Some(run) if extends_current => {
                run.image_col_end = col;
                run.len += 1;
            }
            _ => {
                runs.extend(current.take());
                current = Some(PlaceholderRun {
                    image_id: cell.image_id,
                    placement_id: cell.placement_id,
                    image_row: row,
                    image_col_start: col,
                    image_col_end: col,
                    col_start: grid_col,
                    len: 1,
                });
            }
        }

        previous = Some(PreviousCell {
            grid_col,
            image_id: cell.image_id,
            placement_id: cell.placement_id,
            row,
            col,
        });
    }

    runs.extend(current);
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terminal::model::ansi::NamedColor;
    use warpui::color::ColorU;

    /// A placeholder cell carrying `diacritics` after the base character.
    fn placeholder_cell(foreground: Color, diacritics: &[char]) -> Cell {
        let mut cell = Cell::default();
        cell.c = PLACEHOLDER_CHAR;
        cell.fg = foreground;
        for diacritic in diacritics {
            cell.push_zerowidth(*diacritic, false);
        }
        cell
    }

    /// The diacritic that encodes `index`.
    fn diacritic(index: u32) -> char {
        ROWCOLUMN_DIACRITICS[index as usize]
    }

    #[test]
    fn diacritic_table_is_sorted() {
        assert_eq!(ROWCOLUMN_DIACRITICS.len(), 297);
        assert!(
            ROWCOLUMN_DIACRITICS
                .windows(2)
                .all(|pair| pair[0] < pair[1]),
            "binary_search in diacritic_index requires a sorted, duplicate-free table"
        );
    }

    #[test]
    fn diacritic_index_round_trips() {
        for index in [0, 1, 2, 42, 150, 295, 296] {
            assert_eq!(
                diacritic_index(diacritic(index)),
                Some(index),
                "index {index} did not round trip"
            );
        }

        // Spot-check the first and last entries against the spec's values.
        assert_eq!(diacritic(0), '\u{0305}');
        assert_eq!(diacritic(296), '\u{1D244}');
    }

    #[test]
    fn non_diacritics_have_no_index() {
        for c in ['a', ' ', PLACEHOLDER_CHAR, '\u{0301}'] {
            assert_eq!(diacritic_index(c), None, "{c:?} should not be a diacritic");
        }
    }

    #[test]
    fn non_placeholder_cells_are_rejected() {
        let mut cell = Cell::default();
        cell.c = 'a';
        cell.fg = Color::Indexed(7);

        assert_eq!(parse_placeholder_cell(&cell), None);
    }

    #[test]
    fn indexed_foreground_gives_image_id() {
        let cell = placeholder_cell(Color::Indexed(42), &[diacritic(3), diacritic(5)]);

        let parsed = parse_placeholder_cell(&cell).expect("cell should parse");
        assert_eq!(parsed.image_id, 42);
        assert_eq!(parsed.row, Some(3));
        assert_eq!(parsed.col, Some(5));
        assert_eq!(parsed.placement_id, 0);
    }

    #[test]
    fn direct_foreground_gives_24_bit_image_id() {
        let cell = placeholder_cell(
            Color::Spec(ColorU::new(0x12, 0x34, 0x56, 0xff)),
            &[diacritic(0)],
        );

        let parsed = parse_placeholder_cell(&cell).expect("cell should parse");
        assert_eq!(parsed.image_id, 0x123456);
        assert_eq!(parsed.row, Some(0));
        // Only one diacritic was printed, so the column is left to inheritance.
        assert_eq!(parsed.col, None);
    }

    #[test]
    fn third_diacritic_supplies_the_image_id_high_byte() {
        let cell = placeholder_cell(
            Color::Spec(ColorU::new(0x00, 0x00, 0x01, 0xff)),
            &[diacritic(0), diacritic(0), diacritic(0xAB)],
        );

        let parsed = parse_placeholder_cell(&cell).expect("cell should parse");
        assert_eq!(parsed.image_id, 0xAB00_0001);
    }

    #[test]
    fn only_palette_colors_name_an_image_id() {
        let palette = placeholder_cell(Color::Named(NamedColor::BrightWhite), &[diacritic(0)]);
        assert_eq!(
            parse_placeholder_cell(&palette).map(|cell| cell.image_id),
            Some(15)
        );

        let default = placeholder_cell(Color::Named(NamedColor::Foreground), &[diacritic(0)]);
        assert_eq!(parse_placeholder_cell(&default), None);
    }

    /// Shorthand for a parsed cell at a grid column.
    fn parsed(grid_col: usize, row: Option<u32>, col: Option<u32>) -> (usize, PlaceholderCell) {
        (
            grid_col,
            PlaceholderCell {
                image_id: 1,
                placement_id: 0,
                row,
                col,
            },
        )
    }

    #[test]
    fn fully_specified_cells_batch_into_one_run() {
        let cells = vec![
            parsed(0, Some(0), Some(0)),
            parsed(1, Some(0), Some(1)),
            parsed(2, Some(0), Some(2)),
        ];

        let runs = build_runs(cells.into_iter());

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].image_row, 0);
        assert_eq!(runs[0].image_col_start, 0);
        assert_eq!(runs[0].image_col_end, 2);
        assert_eq!(runs[0].col_start, 0);
        assert_eq!(runs[0].len, 3);
    }

    #[test]
    fn omitted_diacritics_inherit_from_the_left() {
        // The common shorthand: only the first cell spells out its position.
        let cells = vec![
            parsed(4, Some(2), Some(0)),
            parsed(5, None, None),
            parsed(6, None, None),
        ];

        let runs = build_runs(cells.into_iter());

        assert_eq!(
            runs.len(),
            1,
            "inherited cells should join the run: {runs:?}"
        );
        assert_eq!(runs[0].image_row, 2);
        assert_eq!(runs[0].image_col_start, 0);
        assert_eq!(runs[0].image_col_end, 2);
        assert_eq!(runs[0].col_start, 4);
        assert_eq!(runs[0].len, 3);
    }

    #[test]
    fn a_row_only_cell_inherits_the_column() {
        let cells = vec![parsed(0, Some(1), Some(7)), parsed(1, Some(1), None)];

        let runs = build_runs(cells.into_iter());

        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].image_col_start, 7);
        assert_eq!(runs[0].image_col_end, 8);
    }

    #[test]
    fn a_gap_in_grid_columns_splits_runs() {
        let cells = vec![parsed(0, Some(0), Some(0)), parsed(2, Some(0), Some(1))];

        let runs = build_runs(cells.into_iter());

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].col_start, 0);
        assert_eq!(runs[1].col_start, 2);
        // The gap also stops inheritance, so cell 2 keeps its explicit column.
        assert_eq!(runs[1].image_col_start, 1);
    }

    #[test]
    fn a_new_row_splits_runs() {
        let cells = vec![
            parsed(0, Some(0), Some(0)),
            parsed(1, Some(1), Some(1)),
            parsed(2, None, None),
        ];

        let runs = build_runs(cells.into_iter());

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].image_row, 0);
        assert_eq!(runs[1].image_row, 1);
        assert_eq!(runs[1].len, 2, "the third cell inherits row 1: {runs:?}");
    }

    #[test]
    fn a_different_image_splits_runs() {
        let mut cells = vec![parsed(0, Some(0), Some(0)), parsed(1, Some(0), Some(1))];
        cells[1].1.image_id = 9;

        let runs = build_runs(cells.into_iter());

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].image_id, 1);
        assert_eq!(runs[1].image_id, 9);
    }

    #[test]
    fn no_cells_means_no_runs() {
        assert!(build_runs(std::iter::empty()).is_empty());
    }

    /// The decoder is only useful if the print path keeps a placeholder cell
    /// whole: the base character in `Cell::c`, the diacritics as zerowidth
    /// characters, and the image id in the foreground colour.
    #[test]
    fn printed_placeholder_cells_survive_the_grid() {
        let mut terminal = crate::terminal::model::TerminalModel::mock(None, None);
        terminal.simulate_cmd("kitty");

        // A 24-bit foreground carrying image id 0x010203, then two placeholder
        // cells: the first spells out row 0 and column 0, the second omits both.
        let mut printed = String::from("\x1b[38;2;1;2;3m");
        printed.push(PLACEHOLDER_CHAR);
        printed.push(diacritic(0));
        printed.push(diacritic(0));
        printed.push(PLACEHOLDER_CHAR);
        terminal.process_bytes(printed.as_str());

        let grid = terminal
            .block_list()
            .active_block()
            .output_grid()
            .grid_handler();
        let row = grid.row(0).expect("the printed row should exist");

        let first = parse_placeholder_cell(&row[0]).expect("first cell should parse");
        assert_eq!(first.image_id, 0x01_0203);
        assert_eq!(first.row, Some(0));
        assert_eq!(first.col, Some(0));

        let second = parse_placeholder_cell(&row[1]).expect("second cell should parse");
        assert_eq!(second.image_id, 0x01_0203);
        assert_eq!(second.row, None, "trailing diacritics were omitted");
        assert_eq!(second.col, None, "trailing diacritics were omitted");

        // Together they are a single quad covering image columns 0 and 1.
        let runs = build_runs([(0, first), (1, second)].into_iter());
        assert_eq!(runs.len(), 1, "{runs:?}");
        assert_eq!(runs[0].image_col_start, 0);
        assert_eq!(runs[0].image_col_end, 1);
        assert_eq!(runs[0].len, 2);
    }
}
