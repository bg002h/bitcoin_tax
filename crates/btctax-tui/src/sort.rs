//! Shared, display-only column-sort primitives for the row views (Holdings / Disposals / Income).
//!
//! STRICTLY READ-ONLY: sorting reorders DISPLAY rows only. The per-view code builds a *typed,
//! borrowed* working set from the read-only snapshot, sorts THAT with [`stable_sort_by`], then
//! formats the sorted items into `ratatui::Row`s. Nothing here mutates `events`/`LedgerState`
//! (see `SPEC_sort_views.md` [R0-I1]/[R0-I2]).
//!
//! This module owns the *view-agnostic* pieces — the [`Dir`]/[`ViewSort`] state, the cursor/toggle
//! step helpers, the total-order stable sort, and the header-cell builder. The per-view
//! *comparator* (which column maps to which typed field) lives in each tab module, because it knows
//! its own row type.

use ratatui::{
    style::{Color, Modifier, Style},
    widgets::{Cell, Row},
};
use std::cmp::Ordering;

/// Sort direction for a view's focused column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Ascending: dates chronological, numerics low→high, enums by declaration order, `false`<`true`.
    Asc,
    /// Descending: the reverse of [`Dir::Asc`] on the primary key (the tie-break stays ascending so
    /// the order remains total AND deterministic regardless of direction — [R0-M-2]).
    Desc,
}

impl Dir {
    /// Flip the direction (used when `s` re-sorts the already-focused column).
    #[must_use]
    pub fn toggled(self) -> Dir {
        match self {
            Dir::Asc => Dir::Desc,
            Dir::Desc => Dir::Asc,
        }
    }

    /// The header arrow glyph for this direction (`▲` ascending, `▼` descending).
    #[must_use]
    pub fn arrow(self) -> &'static str {
        match self {
            Dir::Asc => "▲",
            Dir::Desc => "▼",
        }
    }
}

/// A view's session-only sort state: which rendered column is the sort key, and the direction.
///
/// `col` indexes the view's rendered columns (0-based, left→right). Per-view, session-only; not
/// persisted; reset on reload. Defaults live in each tab module (`DEFAULT_SORT`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewSort {
    /// Index of the rendered column used as the sort key.
    pub col: usize,
    /// Sort direction.
    pub dir: Dir,
}

impl ViewSort {
    /// Construct a sort state.
    #[must_use]
    pub const fn new(col: usize, dir: Dir) -> Self {
        ViewSort { col, dir }
    }
}

/// Move a column cursor one column left, clamped at column 0 (`h` / `←`).
#[must_use]
pub fn cursor_left(cursor: usize) -> usize {
    cursor.saturating_sub(1)
}

/// Move a column cursor one column right, clamped at the last column (`l` / `→`).
///
/// A `col_count` of 0 is degenerate (no columns) and returns 0.
#[must_use]
pub fn cursor_right(cursor: usize, col_count: usize) -> usize {
    if col_count == 0 {
        0
    } else {
        (cursor + 1).min(col_count - 1)
    }
}

/// Apply an `s` (sort) press for the given focused column.
///
/// - Focusing the column that is ALREADY the sort key → toggle the direction.
/// - Focusing a NEW column → make it the sort key, ascending.
pub fn apply_sort_key(sort: &mut ViewSort, cursor: usize) {
    if sort.col == cursor {
        sort.dir = sort.dir.toggled();
    } else {
        sort.col = cursor;
        sort.dir = Dir::Asc;
    }
}

/// Sort `items` by a `primary` comparator, breaking ties with a total-order `tiebreak`.
///
/// The `dir` is applied to the PRIMARY key only; the `tiebreak` is always ascending so the overall
/// order is TOTAL and DETERMINISTIC in both directions (no RNG) — see [R0-M-2]. `tiebreak` must
/// itself be a total order over `items` (e.g. a stable id, or `(EventId, leg index)`) so equal
/// primary keys never leave a row's position ambiguous. Uses a stable sort.
pub fn stable_sort_by<T>(
    items: &mut [T],
    dir: Dir,
    mut primary: impl FnMut(&T, &T) -> Ordering,
    mut tiebreak: impl FnMut(&T, &T) -> Ordering,
) {
    items.sort_by(|a, b| {
        let base = primary(a, b);
        let base = match dir {
            Dir::Asc => base,
            Dir::Desc => base.reverse(),
        };
        base.then_with(|| tiebreak(a, b))
    });
}

/// Build a header `Row` from column `titles`, marking the sort column with its direction arrow and
/// highlighting the cursor column.
///
/// - The sort column's title gets a trailing ` ▲`/`▼`.
/// - The cursor column's cell is highlighted (cyan + bold + underline) so the user can see which
///   column `s` will act on.
///
/// Returned as `Row<'static>` (titles are copied into owned `String`s) so callers need not keep the
/// slice alive.
#[must_use]
pub fn header_row(titles: &[&str], sort: ViewSort, cursor: usize) -> Row<'static> {
    let cells: Vec<Cell<'static>> = titles
        .iter()
        .enumerate()
        .map(|(i, title)| {
            let mut label = (*title).to_string();
            if i == sort.col {
                label.push(' ');
                label.push_str(sort.dir.arrow());
            }
            let style = if i == cursor {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            Cell::from(label).style(style)
        })
        .collect();
    Row::new(cells)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dir_toggles_both_ways() {
        assert_eq!(Dir::Asc.toggled(), Dir::Desc);
        assert_eq!(Dir::Desc.toggled(), Dir::Asc);
    }

    #[test]
    fn dir_arrow_matches_direction() {
        assert_eq!(Dir::Asc.arrow(), "▲");
        assert_eq!(Dir::Desc.arrow(), "▼");
    }

    #[test]
    fn cursor_left_clamps_at_zero() {
        assert_eq!(cursor_left(0), 0);
        assert_eq!(cursor_left(1), 0);
        assert_eq!(cursor_left(5), 4);
    }

    #[test]
    fn cursor_right_clamps_at_last_column() {
        assert_eq!(cursor_right(0, 3), 1);
        assert_eq!(cursor_right(2, 3), 2, "clamps at col_count-1");
        assert_eq!(cursor_right(0, 0), 0, "degenerate zero-column is a no-op");
    }

    #[test]
    fn apply_sort_key_toggles_same_column() {
        let mut s = ViewSort::new(2, Dir::Asc);
        apply_sort_key(&mut s, 2);
        assert_eq!(s, ViewSort::new(2, Dir::Desc), "same col toggles dir");
        apply_sort_key(&mut s, 2);
        assert_eq!(s, ViewSort::new(2, Dir::Asc), "toggles back");
    }

    #[test]
    fn apply_sort_key_focuses_new_column_ascending() {
        let mut s = ViewSort::new(2, Dir::Desc);
        apply_sort_key(&mut s, 5);
        assert_eq!(
            s,
            ViewSort::new(5, Dir::Asc),
            "new col sorts ascending first, regardless of prior dir"
        );
    }

    #[test]
    fn stable_sort_asc_and_desc_with_tiebreak() {
        // Items: (primary_key, stable_id). Two items share primary_key = 1.
        let mut items = vec![(2, 'a'), (1, 'c'), (1, 'b'), (3, 'd')];
        stable_sort_by(
            &mut items,
            Dir::Asc,
            |a, b| a.0.cmp(&b.0),
            |a, b| a.1.cmp(&b.1),
        );
        assert_eq!(items, vec![(1, 'b'), (1, 'c'), (2, 'a'), (3, 'd')]);

        stable_sort_by(
            &mut items,
            Dir::Desc,
            |a, b| a.0.cmp(&b.0),
            |a, b| a.1.cmp(&b.1),
        );
        // Primary descending; the tie among primary==1 stays ASCENDING by stable id (b before c).
        assert_eq!(items, vec![(3, 'd'), (2, 'a'), (1, 'b'), (1, 'c')]);
    }

    #[test]
    fn stable_sort_is_total_when_tiebreak_is_total() {
        // All equal primary keys → order is fully determined by the tie-break (deterministic).
        let mut items = vec![(0, 3usize), (0, 1), (0, 2), (0, 0)];
        stable_sort_by(
            &mut items,
            Dir::Desc,
            |a, b| a.0.cmp(&b.0),
            |a, b| a.1.cmp(&b.1),
        );
        assert_eq!(items, vec![(0, 0), (0, 1), (0, 2), (0, 3)]);
    }
}
