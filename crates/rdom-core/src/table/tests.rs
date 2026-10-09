//! The table grid's slot assignment (HTML §4.9.12.1, CSS Tables 3 §3.3).

use super::*;

fn span(columns: usize, rows: usize) -> CellSpan {
    CellSpan::new(columns, rows)
}

/// `(row, column, columns, rows)` of every cell, row by row.
fn placed(slots: &TableSlots) -> Vec<Vec<(usize, usize, usize, usize)>> {
    slots
        .cells
        .iter()
        .map(|r| {
            r.iter()
                .map(|s| (s.row, s.column, s.columns, s.rows))
                .collect()
        })
        .collect()
}

// ── Slots ──────────────────────────────────────────────────────────

/// HTML §4.9.12.1 "algorithm for processing rows": each cell takes the
/// first slot of its row no cell covers, left to right.
#[test]
fn cells_take_the_next_free_slot() {
    let slots = assign_slots(&[vec![vec![span(1, 1), span(1, 1)], vec![span(1, 1)]]]);
    assert_eq!(slots.columns, 2);
    assert_eq!(slots.rows, 2);
    assert_eq!(
        placed(&slots),
        vec![vec![(0, 0, 1, 1), (0, 1, 1, 1)], vec![(1, 0, 1, 1)]]
    );
}

/// A `colspan` covers that many slots of its row; the table is as wide
/// as its widest row (HTML §4.9.12.1 step "If xwidth < xcurrent + colspan").
#[test]
fn a_colspan_covers_its_slots() {
    let slots = assign_slots(&[vec![vec![span(2, 1), span(1, 1)], vec![span(1, 1)]]]);
    assert_eq!(slots.columns, 3);
    assert_eq!(
        placed(&slots),
        vec![vec![(0, 0, 2, 1), (0, 2, 1, 1)], vec![(1, 0, 1, 1)]]
    );
}

/// A `rowspan` covers the slots below it: the next row's cells skip them.
#[test]
fn a_rowspan_pushes_the_cells_below_it_aside() {
    let slots = assign_slots(&[vec![
        vec![span(1, 2), span(1, 1)],
        vec![span(1, 1)],
        vec![span(1, 1), span(1, 1)],
    ]]);
    assert_eq!(
        placed(&slots),
        vec![
            vec![(0, 0, 1, 2), (0, 1, 1, 1)],
            vec![(1, 1, 1, 1)],
            vec![(2, 0, 1, 1), (2, 1, 1, 1)],
        ]
    );
}

/// A covered slot between free ones: the cell after it lands past it.
#[test]
fn a_covered_slot_in_the_middle_is_skipped() {
    let slots = assign_slots(&[vec![
        vec![span(1, 1), span(1, 2), span(1, 1)],
        vec![span(1, 1), span(1, 1)],
    ]]);
    assert_eq!(placed(&slots)[1], vec![(1, 0, 1, 1), (1, 2, 1, 1)]);
}

/// CSS Tables 3 §3.3 (and browsers): a cell spans no row past its row
/// group's last; `rowspan="0"` (HTML) spans to that row.
#[test]
fn rowspans_end_at_their_row_group() {
    let slots = assign_slots(&[
        vec![vec![span(1, 5), span(1, 1)], vec![span(1, 1)]],
        vec![vec![span(1, 1), span(1, 1)]],
    ]);
    assert_eq!(slots.rows, 3);
    assert_eq!(
        placed(&slots),
        vec![
            vec![(0, 0, 1, 2), (0, 1, 1, 1)],
            vec![(1, 1, 1, 1)],
            vec![(2, 0, 1, 1), (2, 1, 1, 1)],
        ]
    );
    let zero = assign_slots(&[vec![vec![span(1, 0)], vec![span(1, 1)], vec![span(1, 1)]]]);
    assert_eq!(
        placed(&zero),
        vec![vec![(0, 0, 1, 3)], vec![(1, 1, 1, 1)], vec![(2, 1, 1, 1)]]
    );
}

/// An empty row still counts as a row, and a table of none has no columns.
#[test]
fn empty_rows_and_tables() {
    let slots = assign_slots(&[vec![vec![], vec![span(1, 1)]]]);
    assert_eq!((slots.rows, slots.columns), (2, 1));
    let none = assign_slots(&[]);
    assert_eq!((none.rows, none.columns), (0, 0));
}

// ── Spans from attributes ──────────────────────────────────────────

/// HTML §4.9.11: `colspan` is a valid non-negative integer greater than
/// zero, clamped to 1000 — 0 and garbage are 1; `rowspan` clamped to
/// 65534, 0 kept (to the end of the row group); both parsed by the rules
/// for parsing non-negative integers (§2.3.4.2: leading white space and
/// `+`, trailing garbage ignored).
#[test]
fn spans_parse_as_html_says() {
    let s = |c: Option<&str>, r: Option<&str>| {
        let s = CellSpan::from_attributes(c, r);
        (s.columns(), s.rows())
    };
    assert_eq!(s(None, None), (1, 1));
    assert_eq!(s(Some("3"), Some("2")), (3, 2));
    assert_eq!(s(Some(" +4px"), Some("0")), (4, 0));
    assert_eq!(s(Some("0"), Some("x")), (1, 1));
    assert_eq!(s(Some("-2"), Some("-1")), (1, 1));
    assert_eq!(s(Some("5000"), Some("70000")), (1000, 65534));
}

/// HTML §4.9.3 / §4.9.4: `span` on `<col>` / `<colgroup>` is clamped to
/// 1..=1000, garbage and 0 being 1.
#[test]
fn column_spans_parse_as_html_says() {
    assert_eq!(column_span(None), 1);
    assert_eq!(column_span(Some("3")), 3);
    assert_eq!(column_span(Some("0")), 1);
    assert_eq!(column_span(Some("2000")), 1000);
}

// ── The HTML attributes, read from the DOM ─────────────────────────

/// HTML §4.9.11: only a `<td>` / `<th>` (ASCII case-insensitively) has
/// `colspan` / `rowspan`; any other cell spans one slot. HTML §4.9.3 /
/// §4.9.4: only a `<col>` / `<colgroup>` has `span`.
#[test]
fn spans_are_read_from_html_elements_only() {
    let mut dom: crate::Dom = crate::Dom::new();
    let root = dom.root();
    let mk = |dom: &mut crate::Dom, tag: &str, attrs: &[(&str, &str)]| {
        let id = dom.create_element(tag);
        for (k, v) in attrs {
            dom.set_attribute(id, k, v).unwrap();
        }
        dom.append_child(root, id).unwrap();
        id
    };
    let td = mk(&mut dom, "TD", &[("colspan", "3"), ("rowspan", "2")]);
    let th = mk(&mut dom, "th", &[("colspan", "2")]);
    let div = mk(&mut dom, "div", &[("colspan", "3"), ("rowspan", "2")]);
    assert_eq!(cell_span_of(&dom, td), CellSpan::new(3, 2));
    assert_eq!(cell_span_of(&dom, th), CellSpan::new(2, 1));
    assert_eq!(cell_span_of(&dom, div), CellSpan::new(1, 1));
    let col = mk(&mut dom, "Col", &[("span", "4")]);
    let group = mk(&mut dom, "colgroup", &[("span", "2")]);
    let other = mk(&mut dom, "div", &[("span", "4")]);
    assert_eq!(column_span_of(&dom, col), 4);
    assert_eq!(column_span_of(&dom, group), 2);
    assert_eq!(column_span_of(&dom, other), 1);
}

/// A `CellSpan` is always within HTML's ranges, however it is made — so
/// placement cannot overflow.
#[test]
fn spans_are_clamped_however_made() {
    let huge = CellSpan::new(usize::MAX, usize::MAX);
    assert_eq!((huge.columns(), huge.rows()), (1000, 65534));
    let slots = assign_slots(&[vec![vec![huge, huge]]]);
    assert_eq!(slots.columns, 2000);
    assert_eq!(
        placed(&slots),
        vec![vec![(0, 0, 1000, 1), (0, 1000, 1000, 1)]]
    );
}

// ── Cost (C14G-CORE-GAPS) ───────────────────────────────────────────

/// Architect N17: a row's cells skip the slots covered from above in time
/// independent of how many columns they cover — 65 cells of `colspan=1000
/// rowspan=0` then 10 000 one-cell rows step a few times a row (each row
/// walked 65 000 covered columns, 6.5·10⁸ steps).
#[test]
fn covered_columns_are_skipped_in_bounded_steps() {
    let mut rows = vec![vec![span(1000, 0); 65]];
    rows.extend(std::iter::repeat_n(vec![span(1, 1)], 10_000));
    probe::take();
    let slots = assign_slots(&[rows]);
    assert_eq!(slots.cells[1][0].column, 65_000);
    let steps = probe::take();
    assert!(steps < 100_000, "{steps} steps");
}

/// The run-based coverage places every cell where the column-by-column
/// algorithm (HTML §4.9.12.1 as written) does: 300 pseudo-random tables of
/// mixed spans, overlapping rowspans and `rowspan=0` included.
#[test]
fn coverage_runs_place_cells_as_the_slot_by_slot_algorithm() {
    fn naive(groups: &[Vec<Vec<CellSpan>>]) -> TableSlots {
        let mut out = TableSlots::default();
        for group in groups {
            let first_row = out.rows;
            let end = first_row + group.len();
            let mut covered_until: Vec<usize> = Vec::new();
            for (r, row) in group.iter().enumerate() {
                let y = first_row + r;
                let mut x = 0usize;
                let mut placed = Vec::new();
                for cell in row {
                    while covered_until.get(x).is_some_and(|&u| u > y) {
                        x += 1;
                    }
                    let rows = match cell.rows {
                        0 => end - y,
                        n => n.min(end - y),
                    };
                    if covered_until.len() < x + cell.columns {
                        covered_until.resize(x + cell.columns, 0);
                    }
                    for u in &mut covered_until[x..x + cell.columns] {
                        *u = (*u).max(y + rows);
                    }
                    placed.push(Slot {
                        row: y,
                        column: x,
                        columns: cell.columns,
                        rows,
                    });
                    x += cell.columns;
                }
                out.columns = out.columns.max(x).max(covered_until.len());
                out.cells.push(placed);
            }
            out.rows = end;
        }
        out
    }
    let mut seed = 0x2545_f491_4f6c_dd1du64;
    let mut next = |n: u64| {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        (seed % n) as usize
    };
    for _ in 0..300 {
        let groups: Vec<Vec<Vec<CellSpan>>> = (0..1 + next(3))
            .map(|_| {
                (0..1 + next(6))
                    .map(|_| (0..next(5)).map(|_| span(1 + next(3), next(4))).collect())
                    .collect()
            })
            .collect();
        assert_eq!(assign_slots(&groups), naive(&groups), "{groups:?}");
    }
}
