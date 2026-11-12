use super::*;
use crate::style::{parse_grid_auto_tracks, parse_grid_template_tracks};

fn text_grid(
    columns: &str,
    texts: &[&str],
    cell_style: DivStyle,
) -> (PaintEngine, DomId, Vec<DomId>) {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(DivStyle {
        display: LayoutDisplay::Grid,
        width: CssDimension::Percent(1.0),
        grid_template_columns: parse_grid_template_tracks(columns).unwrap(),
        justify_content: Some(LayoutJustifyContent::FlexStart),
        ..DivStyle::default()
    });
    let cells = texts
        .iter()
        .map(|text| {
            let cell = engine.create_element(cell_style.clone());
            let text = engine.create_text(*text);
            engine.append_child(cell, text);
            engine.append_child(root, cell);
            cell
        })
        .collect();
    engine.set_root(root);
    (engine, root, cells)
}

fn layout(engine: &PaintEngine, node: DomId) -> taffy::Layout {
    engine.arena.layout(engine.node_for(node).unwrap())
}

#[test]
fn grid_minmax_content_tracks_wrap_and_remeasure_when_constrained() {
    let (mut engine, root, cells) = text_grid(
        "minmax(0, max-content) minmax(0, max-content)",
        &["#", "abcdefghijklmnopqrst", "!", "last row"],
        DivStyle {
            min_width: CssDimension::Length(0.0),
            overflow_wrap: CssOverflowWrap::Anywhere,
            ..DivStyle::default()
        },
    );
    for width in [32, 10, 7, 32] {
        let frame = engine.render_frame(width, 20).unwrap();
        let first = layout(&engine, cells[0]);
        let second = layout(&engine, cells[1]);
        let next_row = layout(&engine, cells[2]);
        let available = (width as usize - 1).min(20);
        let lines = 20_usize.div_ceil(available);
        assert_eq!(layout(&engine, root).size.width, width as f32);
        assert_eq!(first.size.width, 1.0);
        assert_eq!(second.size.width, available as f32);
        assert_eq!(second.location.x, 1.0);
        assert!(second.size.width > first.size.width);
        assert!(second.location.x + second.size.width <= width as f32);
        assert_eq!(second.size.height, lines as f32);
        assert_eq!(next_row.location.y, lines as f32);
        for (offset, character) in "abcdefghijklmnopqrst".chars().enumerate() {
            assert_eq!(
                frame
                    .cell(1 + offset % available, offset / available)
                    .unwrap()
                    .character,
                character
            );
        }
        assert_eq!(frame.cell(0, lines).unwrap().character, '!');
        if width == 32 {
            // max-content tracks do not stretch to fill spare space.
            for x in 21..32 {
                assert_eq!(frame.cell(x, 0).unwrap().character, ' ');
            }
        }
    }
}

#[test]
fn grid_minmax_fractional_tracks_can_shrink_below_automatic_minima() {
    let cell_style = DivStyle {
        min_width: CssDimension::Auto,
        overflow_wrap: CssOverflowWrap::BreakWord,
        ..DivStyle::default()
    };
    let (mut explicit, _, explicit_cells) = text_grid(
        "minmax(0, 1fr) minmax(0, 1fr)",
        &["abcdefghijklmnop", "xyz"],
        cell_style.clone(),
    );
    let frame = explicit.render_frame(10, 10).unwrap();
    assert_eq!(layout(&explicit, explicit_cells[0]).size.width, 5.0);
    assert_eq!(layout(&explicit, explicit_cells[1]).location.x, 5.0);
    assert_eq!(layout(&explicit, explicit_cells[1]).size.width, 5.0);
    assert_eq!(layout(&explicit, explicit_cells[0]).size.height, 4.0);
    assert_eq!(frame.cell(0, 3).unwrap().character, 'p');

    let (mut bare, _, bare_cells) = text_grid("1fr 1fr", &["abcdefghijklmnop", "xyz"], cell_style);
    bare.render_frame(10, 10).unwrap();
    assert_eq!(layout(&bare, bare_cells[0]).size.width, 16.0);
    assert!(
        layout(&bare, bare_cells[1]).location.x + layout(&bare, bare_cells[1]).size.width > 10.0
    );
}

#[test]
fn grid_minmax_fractional_tracks_fit_odd_cell_widths_with_wide_glyphs() {
    let (mut engine, root, cells) = text_grid(
        "minmax(0, 1fr) minmax(0, 1fr)",
        &[
            "\u{754c}\u{754c}\u{754c}\u{754c}\u{754c}\u{754c}",
            "abcdefghijkl",
        ],
        DivStyle {
            min_width: CssDimension::Length(0.0),
            overflow_wrap: CssOverflowWrap::Anywhere,
            ..DivStyle::default()
        },
    );
    engine.set_style(
        root,
        DivStyle {
            display: LayoutDisplay::Grid,
            width: CssDimension::Length(9.0),
            grid_template_columns: parse_grid_template_tracks("minmax(0, 1fr) minmax(0, 1fr)")
                .unwrap(),
            ..DivStyle::default()
        },
    );
    let frame = engine.render_frame(12, 10).unwrap();
    let first = layout(&engine, cells[0]);
    let second = layout(&engine, cells[1]);
    assert_eq!(first.size.width + second.size.width, 9.0);
    assert_eq!(second.location.x, first.size.width);
    assert_eq!(second.location.x + second.size.width, 9.0);
    assert_eq!(first.size.height, 3.0);
    assert_eq!(layout(&engine, root).size.height, 3.0);
    for y in 0..3 {
        assert_eq!(frame.cell(0, y).unwrap().character, '\u{754c}');
        assert!(frame.cell(1, y).unwrap().wide_continuation);
        assert_eq!(frame.cell(2, y).unwrap().character, '\u{754c}');
        assert!(frame.cell(3, y).unwrap().wide_continuation);
        for x in 9..12 {
            assert_eq!(frame.cell(x, y).unwrap().character, ' ');
        }
    }
}

#[test]
fn grid_minmax_template_rows_respect_fixed_minimum() {
    let (mut engine, root, cells) = text_grid("1fr", &["a", "b"], DivStyle::default());
    engine.set_style(
        root,
        DivStyle {
            display: LayoutDisplay::Grid,
            width: CssDimension::Length(10.0),
            height: CssDimension::Length(4.0),
            grid_template_columns: parse_grid_template_tracks("1fr").unwrap(),
            grid_template_rows: parse_grid_template_tracks("minmax(3, 1fr) minmax(0, 1fr)")
                .unwrap(),
            ..DivStyle::default()
        },
    );
    let frame = engine.render_frame(10, 10).unwrap();
    assert_eq!(layout(&engine, cells[0]).size.height, 3.0);
    assert_eq!(layout(&engine, cells[1]).size.height, 1.0);
    assert_eq!(layout(&engine, cells[1]).location.y, 3.0);
    assert_eq!(frame.cell(0, 3).unwrap().character, 'b');
}

#[test]
fn grid_minmax_fractional_width_measures_and_paints_the_same_wrapped_rows() {
    let (mut engine, _, cells) = text_grid(
        "minmax(0, 1fr) minmax(0, 1fr)",
        &["abcde", "fghij", "#", "#"],
        DivStyle {
            min_width: CssDimension::Length(0.0),
            overflow_wrap: CssOverflowWrap::Anywhere,
            ..DivStyle::default()
        },
    );
    let frame = engine.render_frame(9, 6).unwrap();
    assert_eq!(layout(&engine, cells[0]).size.height, 2.0);
    assert_eq!(layout(&engine, cells[1]).size.height, 2.0);
    assert_eq!(layout(&engine, cells[2]).location.y, 2.0);
    let second_x = layout(&engine, cells[1]).location.x as usize;
    assert_eq!(frame.cell(0, 1).unwrap().character, 'e');
    assert_eq!(frame.cell(second_x, 1).unwrap().character, 'j');
    assert_eq!(frame.cell(0, 2).unwrap().character, '#');
}

#[test]
fn grid_minmax_automatic_tracks_size_both_axes() {
    for flow in [LayoutGridAutoFlow::Row, LayoutGridAutoFlow::Column] {
        let (mut engine, root, cells) = text_grid("1fr", &["a", "b"], DivStyle::default());
        engine.set_style(
            root,
            DivStyle {
                display: LayoutDisplay::Grid,
                width: CssDimension::Length(10.0),
                height: CssDimension::Length(6.0),
                grid_auto_flow: flow,
                grid_auto_columns: parse_grid_auto_tracks("minmax(20%, 1fr)").unwrap(),
                grid_auto_rows: parse_grid_auto_tracks("minmax(2, 1fr)").unwrap(),
                ..DivStyle::default()
            },
        );
        engine.render_frame(10, 10).unwrap();
        let first = layout(&engine, cells[0]);
        let second = layout(&engine, cells[1]);
        match flow {
            LayoutGridAutoFlow::Row => {
                assert_eq!(first.size.width, 10.0);
                assert_eq!(first.size.height, 3.0);
                assert_eq!(second.location.y, 3.0);
                assert_eq!(second.location.y + second.size.height, 6.0);
            }
            LayoutGridAutoFlow::Column => {
                assert_eq!(first.size.width, 5.0);
                assert_eq!(first.size.height, 6.0);
                assert_eq!(second.location.x, 5.0);
                assert_eq!(second.location.x + second.size.width, 10.0);
            }
            _ => unreachable!(),
        }
    }
}
