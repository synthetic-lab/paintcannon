use std::time::Duration;

use super::*;

use crate::selection::{SelectionMouseEvent, SelectionMouseEventType};
use crate::style::{
    Background, CssDimension, CssFontWeight, LayoutFlexDirection, LayoutOverflow,
    TransitionProperty, TransitionSpec,
};
use crate::transition::TransitionEventType;

fn block_style(width: CssDimension, height: CssDimension) -> DivStyle {
    let mut style = DivStyle::default();
    style.width = width;
    style.height = height;
    style
}

#[test]
fn clean_ticks_skip_paint_and_paint_only_changes_skip_layout() {
    let mut engine = PaintEngine::new();
    let input = engine.create_input_with_id(
        DomId(1),
        block_style(CssDimension::Length(6.0), CssDimension::Length(1.0)),
        "value",
    );
    engine.set_root(input);
    engine.set_input_focused(input, true);

    let mut output = Vec::new();
    assert!(engine
        .flush_if_dirty_to(
            6,
            1,
            &mut output,
            TermProfile::TrueColor,
            false,
            Instant::now()
        )
        .unwrap());
    let layout_passes = engine.layout_passes();
    output.clear();

    assert!(!engine
        .flush_if_dirty_to(
            6,
            1,
            &mut output,
            TermProfile::TrueColor,
            false,
            Instant::now()
        )
        .unwrap());
    assert!(output.is_empty());

    engine.set_terminal_focused(false);
    assert_eq!(engine.dirtiness, Dirtiness::Paint);
    assert!(engine
        .flush_if_dirty_to(
            6,
            1,
            &mut output,
            TermProfile::TrueColor,
            false,
            Instant::now()
        )
        .unwrap());
    assert_eq!(engine.layout_passes(), layout_passes);
}

fn scroll_engine() -> (PaintEngine, DomId) {
    let mut engine = PaintEngine::new();
    let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(1.0));
    viewport_style.overflow_y = LayoutOverflow::Scroll;
    let viewport = engine.create_element(viewport_style);
    let mut content_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
    content_style.display = crate::style::LayoutDisplay::Flex;
    content_style.flex_direction = LayoutFlexDirection::Column;
    let content = engine.create_element(content_style);
    for text in ["aaaaa", "bbbbb"] {
        let row = engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
        let text = engine.create_text(text);
        engine.append_child(row, text);
        engine.append_child(content, row);
    }
    engine.append_child(viewport, content);
    engine.set_root(viewport);
    (engine, viewport)
}

#[test]
fn scroll_offset_render_does_not_recompute_layout() {
    let (mut engine, viewport) = scroll_engine();
    let first = engine.render_frame(5, 1).unwrap();
    assert_eq!(first.cell(0, 0).unwrap().character, 'a');
    let passes = engine.layout_passes();

    engine.set_scroll_offset(viewport, 0, 1);
    let second = engine.render_frame(5, 1).unwrap();

    assert_eq!(engine.layout_passes(), passes);
    assert_eq!(second.cell(0, 0).unwrap().character, 'b');
}

#[test]
fn render_clamps_scroll_offset_after_viewport_grows() {
    let mut engine = PaintEngine::new();
    let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Percent(1.0));
    viewport_style.overflow_y = LayoutOverflow::Scroll;
    let viewport = engine.create_element(viewport_style);
    let mut content_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
    content_style.display = crate::style::LayoutDisplay::Flex;
    content_style.flex_direction = LayoutFlexDirection::Column;
    let content = engine.create_element(content_style);
    for index in 0..10 {
        let row = engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
        let text = engine.create_text(format!("{index}{index}{index}{index}{index}"));
        engine.append_child(row, text);
        engine.append_child(content, row);
    }
    engine.append_child(viewport, content);
    engine.set_root(viewport);

    engine.render_frame(5, 3).unwrap();
    engine.set_scroll_offset_for_size(viewport, 0, 100, 5, 3);
    let small = engine.render_frame(5, 3).unwrap();
    assert_eq!(small.cell(0, 0).unwrap().character, '7');

    let large = engine.render_frame(5, 8).unwrap();
    assert_eq!(large.cell(0, 0).unwrap().character, '2');
    assert_eq!(large.cell(0, 7).unwrap().character, '9');
}

#[test]
fn resize_metrics_query_before_paint_does_not_skip_scroll_clamping() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(10.0), CssDimension::Percent(1.0));
    root_style.display = crate::style::LayoutDisplay::Flex;
    root_style.flex_direction = LayoutFlexDirection::Row;
    let root = engine.create_element(root_style);
    let mut viewports = Vec::new();
    for _ in 0..2 {
        let mut viewport_style = block_style(CssDimension::Length(5.0), CssDimension::Percent(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = engine.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = engine.create_element(content_style);
        for index in 0..10 {
            let row =
                engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
            let text = engine.create_text(format!("{index}{index}{index}{index}{index}"));
            engine.append_child(row, text);
            engine.append_child(content, row);
        }
        engine.append_child(viewport, content);
        engine.append_child(root, viewport);
        viewports.push(viewport);
    }
    engine.set_root(root);

    engine.render_frame(10, 3).unwrap();
    for viewport in &viewports {
        engine.set_scroll_offset_for_size(*viewport, 0, 100, 10, 3);
    }
    let small = engine.render_frame(10, 3).unwrap();
    assert_eq!(small.cell(0, 0).unwrap().character, '7');
    assert_eq!(small.cell(5, 0).unwrap().character, '7');

    // Octo reads and re-pins its transcript from a resize RAF. That operation
    // must not prevent the same layout pass from clamping every scroll node.
    let resized_metrics = engine.scroll_metrics_for_size(viewports[1], 10, 8).unwrap();
    assert_eq!(resized_metrics.client_height, 8);
    let max_scroll = resized_metrics
        .scroll_height
        .saturating_sub(resized_metrics.client_height);
    engine.set_scroll_offset_for_size(viewports[1], 0, max_scroll, 10, 8);
    let large = engine.render_frame(10, 8).unwrap();

    assert_eq!(engine.scroll_metrics(viewports[0]).unwrap().scroll_top, 2);
    assert_eq!(engine.scroll_metrics(viewports[1]).unwrap().scroll_top, 2);
    assert_eq!(large.cell(0, 0).unwrap().character, '2');
    assert_eq!(large.cell(5, 0).unwrap().character, '2');
    assert_eq!(large.cell(0, 7).unwrap().character, '9');
    assert_eq!(large.cell(5, 7).unwrap().character, '9');
}

#[test]
fn scroll_metrics_query_before_first_render_computes_layout() {
    let mut engine = PaintEngine::new();

    let mut root_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
    root_style.display = crate::style::LayoutDisplay::Flex;
    root_style.flex_direction = LayoutFlexDirection::Column;
    let root = engine.create_element(root_style);

    let header = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Length(2.0),
    ));

    let mut body_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
    body_style.display = crate::style::LayoutDisplay::Flex;
    body_style.flex_direction = LayoutFlexDirection::Row;
    body_style.flex_grow = 1.0;
    body_style.flex_shrink = 1.0;
    body_style.flex_basis = CssDimension::Length(0.0);
    let body = engine.create_element(body_style);

    let viewport = engine.create_element(block_style(
        CssDimension::Percent(0.8),
        CssDimension::Percent(1.0),
    ));
    let rail = engine.create_element(block_style(
        CssDimension::Percent(0.2),
        CssDimension::Percent(1.0),
    ));
    let scrollbar = engine.create_text("#");
    engine.append_child(rail, scrollbar);

    engine.append_child(body, viewport);
    engine.append_child(body, rail);
    engine.append_child(root, header);
    engine.append_child(root, body);
    engine.set_root(root);

    let rail_metrics = engine.scroll_metrics_for_size(rail, 80, 24).unwrap();
    let viewport_metrics = engine.scroll_metrics_for_size(viewport, 80, 24).unwrap();

    assert_eq!(rail_metrics.client_height, 22);
    assert_eq!(viewport_metrics.client_height, 22);
    assert_eq!(engine.layout_passes(), 1);
}

#[test]
fn scrollbar_hit_testing_uses_rendered_regions_and_suppresses_selection() {
    let mut engine = PaintEngine::new();
    let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(3.0));
    viewport_style.overflow_x = LayoutOverflow::Scroll;
    viewport_style.overflow_y = LayoutOverflow::Scroll;
    let viewport = engine.create_element(viewport_style);
    let child = engine.create_element(block_style(
        CssDimension::Length(10.0),
        CssDimension::Length(5.0),
    ));
    engine.append_child(viewport, child);
    engine.set_root(viewport);

    engine.render_frame(6, 3).unwrap();

    let hit = engine.scrollbar_hit_at(5, 1).unwrap();
    assert_eq!(hit.target_id, viewport);
    assert_eq!(hit.axis, ScrollbarAxis::Vertical);

    assert!(scrollbar_suppresses_selection(
        &mut engine,
        SelectionMouseEvent {
            event_type: SelectionMouseEventType::Down,
            x: 5,
            y: 1,
            button: 0,
        },
    ));
    assert!(scrollbar_suppresses_selection(
        &mut engine,
        SelectionMouseEvent {
            event_type: SelectionMouseEventType::Drag,
            x: 5,
            y: 0,
            button: 0,
        },
    ));
    assert!(scrollbar_suppresses_selection(
        &mut engine,
        SelectionMouseEvent {
            event_type: SelectionMouseEventType::Up,
            x: 5,
            y: 0,
            button: 0,
        },
    ));
}

#[test]
fn viewport_automatically_enables_scrollbars_for_descendant_overflow() {
    let mut engine = PaintEngine::new();
    let viewport = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Percent(1.0),
    ));
    let root = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Percent(1.0),
    ));
    let content = engine.create_element(block_style(
        CssDimension::Length(20.0),
        CssDimension::Length(10.0),
    ));
    assert!(engine.append_child(root, content));
    assert!(engine.append_child(viewport, root));
    assert!(engine.set_viewport(viewport));
    assert!(engine.set_root(viewport));

    engine.render_frame(10, 4).unwrap();

    let metrics = engine.scroll_metrics(viewport).unwrap();
    assert_eq!(metrics.client_width, 9);
    assert_eq!(metrics.client_height, 3);
    assert_eq!(metrics.scroll_width, 20);
    assert_eq!(metrics.scroll_height, 10);
    let vertical = engine.scrollbar_hit_at(9, 1).unwrap();
    assert_eq!(vertical.target_id, viewport);
    assert_eq!(vertical.axis, ScrollbarAxis::Vertical);
    let horizontal = engine.scrollbar_hit_at(2, 3).unwrap();
    assert_eq!(horizontal.target_id, viewport);
    assert_eq!(horizontal.axis, ScrollbarAxis::Horizontal);

    let layout_passes = engine.layout_passes();
    let thumb_color = Background::Rgb(56, 189, 248);
    assert!(engine.mutate_style(
        root,
        StyleMutation::ScrollbarColor(ScrollbarColor::Colors {
            thumb: thumb_color,
            track: Background::Rgb(17, 24, 39),
        }),
    ));
    let recolored = engine.render_frame(10, 4).unwrap();
    assert_eq!(engine.layout_passes(), layout_passes);
    assert_eq!(recolored.cell(9, 0).unwrap().background, thumb_color);

    engine.set_scroll_offset(viewport, 8, 6).unwrap();
    engine.render_frame(30, 12).unwrap();
    let resized = engine.scroll_metrics(viewport).unwrap();
    assert_eq!(resized.scroll_left, 0);
    assert_eq!(resized.scroll_top, 0);
    assert!(engine.scrollbar_hit_at(29, 1).is_none());
    assert!(engine.scrollbar_hit_at(1, 11).is_none());
}

#[test]
fn viewport_does_not_render_scrollbars_when_content_fits() {
    let mut engine = PaintEngine::new();
    let viewport = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Percent(1.0),
    ));
    let content = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Percent(1.0),
    ));
    assert!(engine.append_child(viewport, content));
    assert!(engine.set_viewport(viewport));
    assert!(engine.set_root(viewport));

    engine.render_frame(10, 4).unwrap();

    assert!(engine.scrollbar_hit_at(9, 1).is_none());
    let viewport_node = engine.node_for(viewport).unwrap();
    let style = engine.arena.style(viewport_node);
    assert!(style.overflow_x == LayoutOverflow::Hidden);
    assert!(style.overflow_y == LayoutOverflow::Hidden);
}

#[test]
fn viewport_frame_is_unchanged_when_scrolling_past_the_end() {
    let mut engine = PaintEngine::new();
    let viewport = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Percent(1.0),
    ));
    let root = engine.create_element(block_style(
        CssDimension::Percent(1.0),
        CssDimension::Percent(1.0),
    ));
    let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
    content_style.display = crate::style::LayoutDisplay::Flex;
    content_style.flex_direction = LayoutFlexDirection::Column;
    let content = engine.create_element(content_style);
    for index in 0..80 {
        let row = engine.create_element(block_style(
            CssDimension::Percent(1.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text(format!("{index:02}"));
        assert!(engine.append_child(row, text));
        assert!(engine.append_child(content, row));
    }
    assert!(engine.append_child(root, content));
    assert!(engine.append_child(viewport, root));
    assert!(engine.set_viewport(viewport));
    assert!(engine.set_root(viewport));

    engine.render_frame(10, 4).unwrap();
    let metrics = engine.scroll_metrics(viewport).unwrap();
    assert_eq!(metrics.client_height, 4);
    assert_eq!(metrics.scroll_height, 80);
    let max_top = metrics.scroll_height - metrics.client_height;
    let at_max_metrics = engine.set_scroll_offset(viewport, 0, max_top).unwrap();
    let at_max = engine.render_frame(10, 4).unwrap();

    let past_end_metrics = engine
        .set_scroll_offset(viewport, 0, max_top.saturating_add(3))
        .unwrap();
    let past_end = engine.render_frame(10, 4).unwrap();

    assert_eq!(at_max_metrics, past_end_metrics);
    for y in 0..at_max.height() {
        for x in 0..at_max.width() {
            assert_eq!(at_max.cell(x, y), past_end.cell(x, y));
        }
    }
    assert!((0..past_end.height())
        .flat_map(|y| (0..past_end.width()).map(move |x| (x, y)))
        .any(|(x, y)| past_end
            .cell(x, y)
            .is_some_and(|cell| cell.character != ' ')));
}

#[test]
fn selection_in_later_scroll_pane_does_not_capture_hidden_text_from_earlier_pane() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(20.0), CssDimension::Length(2.0));
    root_style.display = crate::style::LayoutDisplay::Flex;
    root_style.flex_direction = LayoutFlexDirection::Row;
    let root = engine.create_element(root_style);

    let mut left_style = block_style(CssDimension::Length(10.0), CssDimension::Length(2.0));
    left_style.overflow_y = LayoutOverflow::Scroll;
    let left = engine.create_element(left_style);
    let mut left_content_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
    left_content_style.display = crate::style::LayoutDisplay::Flex;
    left_content_style.flex_direction = LayoutFlexDirection::Column;
    let left_content = engine.create_element(left_content_style);
    for index in 0..8 {
        let row = engine.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text(format!("left-{index}"));
        engine.append_child(row, text);
        engine.append_child(left_content, row);
    }
    engine.append_child(left, left_content);

    let mut right_style = block_style(CssDimension::Length(10.0), CssDimension::Length(2.0));
    right_style.overflow_y = LayoutOverflow::Scroll;
    let right = engine.create_element(right_style);
    let mut right_content_style = block_style(CssDimension::Length(10.0), CssDimension::Auto);
    right_content_style.display = crate::style::LayoutDisplay::Flex;
    right_content_style.flex_direction = LayoutFlexDirection::Column;
    let right_content = engine.create_element(right_content_style);
    for text in ["RIGHT-0", "RIGHT-1"] {
        let row = engine.create_element(block_style(
            CssDimension::Length(10.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text(text);
        engine.append_child(row, text);
        engine.append_child(right_content, row);
    }
    engine.append_child(right, right_content);

    engine.append_child(root, left);
    engine.append_child(root, right);
    engine.set_root(root);

    engine.render_frame(20, 2).unwrap();
    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Down,
        x: 10,
        y: 0,
        button: 0,
    });
    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Drag,
        x: 16,
        y: 0,
        button: 0,
    });
    engine.render_frame(20, 2).unwrap();

    let action = engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Up,
        x: 16,
        y: 0,
        button: 0,
    });

    assert_eq!(
        action,
        SelectionAction::CopyToClipboard("RIGHT-0".to_string())
    );
}

#[test]
fn selection_drag_below_scroll_pane_scrolls_pane_before_selecting_outside() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(4.0));
    root_style.display = crate::style::LayoutDisplay::Flex;
    root_style.flex_direction = LayoutFlexDirection::Column;
    let root = engine.create_element(root_style);

    let mut viewport_style = block_style(CssDimension::Length(12.0), CssDimension::Length(2.0));
    viewport_style.overflow_y = LayoutOverflow::Scroll;
    let viewport = engine.create_element(viewport_style);
    let mut content_style = block_style(CssDimension::Length(12.0), CssDimension::Auto);
    content_style.display = crate::style::LayoutDisplay::Flex;
    content_style.flex_direction = LayoutFlexDirection::Column;
    let content = engine.create_element(content_style);
    for index in 0..6 {
        let row = engine.create_element(block_style(
            CssDimension::Length(12.0),
            CssDimension::Length(1.0),
        ));
        let text = engine.create_text(format!("RIGHT-{index}"));
        engine.append_child(row, text);
        engine.append_child(content, row);
    }
    engine.append_child(viewport, content);

    let footer = engine.create_element(block_style(
        CssDimension::Length(12.0),
        CssDimension::Length(2.0),
    ));
    let footer_text = engine.create_text("FOOTER");
    engine.append_child(footer, footer_text);

    engine.append_child(root, viewport);
    engine.append_child(root, footer);
    engine.set_root(root);

    engine.render_frame(12, 4).unwrap();
    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Down,
        x: 0,
        y: 0,
        button: 0,
    });
    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Drag,
        x: 6,
        y: 2,
        button: 0,
    });
    let action = engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Up,
        x: 6,
        y: 2,
        button: 0,
    });

    assert_eq!(
        action,
        SelectionAction::CopyToClipboard(["RIGHT-0", "RIGHT-1", "RIGHT-2", "RIGHT-3"].join("\n"))
    );
}

#[test]
fn selection_drag_right_of_scroll_pane_scrolls_pane_before_selecting_outside() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(12.0), CssDimension::Length(2.0));
    root_style.display = crate::style::LayoutDisplay::Flex;
    root_style.flex_direction = LayoutFlexDirection::Row;
    let root = engine.create_element(root_style);

    let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(2.0));
    viewport_style.overflow_x = LayoutOverflow::Scroll;
    let viewport = engine.create_element(viewport_style);
    let content = engine.create_element(block_style(
        CssDimension::Length(12.0),
        CssDimension::Length(1.0),
    ));
    let content_text = engine.create_text("ABCDEFGHIJKL");
    engine.append_child(content, content_text);
    engine.append_child(viewport, content);

    let sibling = engine.create_element(block_style(
        CssDimension::Length(6.0),
        CssDimension::Length(2.0),
    ));
    let sibling_text = engine.create_text("OUT");
    engine.append_child(sibling, sibling_text);

    engine.append_child(root, viewport);
    engine.append_child(root, sibling);
    engine.set_root(root);

    engine.render_frame(12, 2).unwrap();
    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Down,
        x: 0,
        y: 0,
        button: 0,
    });
    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Drag,
        x: 8,
        y: 0,
        button: 0,
    });
    let action = engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Up,
        x: 8,
        y: 0,
        button: 0,
    });

    assert_eq!(
        action,
        SelectionAction::CopyToClipboard("ABCDEFGHIJKL".to_string())
    );
}

#[test]
fn percent_scroll_demo_keeps_widths_after_scroll_text_updates() {
    let mut engine = PaintEngine::new();

    let mut root_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
    root_style.display = crate::style::LayoutDisplay::Flex;
    root_style.flex_direction = LayoutFlexDirection::Column;
    root_style.background = Background::Black;
    let root = engine.create_element(root_style);

    let mut header_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(0.1));
    header_style.background = Background::Cyan;
    let header = engine.create_element(header_style);
    let status = engine.create_text(
        "Percent scroll demo. Resize the terminal; wheel over the panel. Ctrl-C exits.",
    );
    engine.append_child(header, status);

    let mut body_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(0.9));
    body_style.display = crate::style::LayoutDisplay::Flex;
    body_style.flex_direction = LayoutFlexDirection::Row;
    let body = engine.create_element(body_style);

    let mut viewport_style = block_style(CssDimension::Percent(1.0), CssDimension::Percent(1.0));
    viewport_style.overflow_y = LayoutOverflow::Scroll;
    viewport_style.overflow_x = LayoutOverflow::Hidden;
    viewport_style.background = Background::Blue;
    let viewport = engine.create_element(viewport_style);

    let mut content_style = block_style(CssDimension::Percent(1.0), CssDimension::Auto);
    content_style.display = crate::style::LayoutDisplay::Flex;
    content_style.flex_direction = LayoutFlexDirection::Column;
    let content = engine.create_element(content_style);
    let mut row_ids = Vec::new();
    for index in 1..=200 {
        let row =
            engine.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
        row_ids.push(row);
        let text = engine.create_text(format!(
            "percent row {index:02} - resize changes visible content"
        ));
        engine.append_child(row, text);
        engine.append_child(content, row);
    }

    engine.append_child(viewport, content);
    engine.append_child(body, viewport);
    engine.append_child(root, header);
    engine.append_child(root, body);
    engine.set_root(root);

    engine.render_frame(80, 24).unwrap();
    let viewport_node = engine.node_for(viewport).unwrap();
    let first_row_node = engine.node_for(row_ids[0]).unwrap();
    let fourth_row_node = engine.node_for(row_ids[3]).unwrap();
    let before_viewport = engine.arena.layout(viewport_node);
    assert_eq!(before_viewport.size.width, 80.0);
    assert_eq!(engine.arena.layout(first_row_node).size.width, 79.0);
    assert_eq!(engine.arena.layout(fourth_row_node).size.width, 79.0);

    let metrics = engine
        .set_scroll_offset_for_size(viewport, 0, 3, 80, 24)
        .unwrap();
    engine.set_text(
        status,
        format!(
            "scrollTop={}/{}, clientHeight={}",
            metrics.scroll_top, metrics.scroll_height, metrics.client_height
        ),
    );

    let frame = engine.render_frame(80, 24).unwrap();
    let after_viewport = engine.arena.layout(viewport_node);

    assert_eq!(after_viewport.size.width, 80.0);
    assert_eq!(engine.arena.layout(fourth_row_node).size.width, 79.0);
    let visible_row_prefix: String = (0..11)
        .map(|x| frame.cell(x, 2).unwrap().character)
        .collect();
    assert_eq!(visible_row_prefix, "percent row");
}

#[test]
fn text_mutation_recomputes_layout() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
    let text = engine.create_text("short");
    engine.append_child(root, text);
    engine.set_root(root);

    engine.render_frame(5, 5).unwrap();
    let passes = engine.layout_passes();
    engine.set_text(text, "hello world");
    engine.render_frame(5, 5).unwrap();

    assert_eq!(engine.layout_passes(), passes + 1);
}

#[test]
fn hit_testing_uses_last_rendered_regions() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
    root_style.background = Background::Blue;
    let root = engine.create_element(root_style);
    engine.set_root(root);
    engine.render_frame(4, 1).unwrap();

    assert_eq!(engine.target_at(0, 0), Some(root));
    assert_eq!(engine.target_at(4, 0), None);
}

#[test]
fn dom_ids_are_stable_and_not_reused_after_destroy() {
    let mut engine = PaintEngine::new();
    let first = engine.create_element(DivStyle::default());
    assert!(engine.destroy_node(first));

    let second = engine.create_element(DivStyle::default());

    assert_ne!(first, second);
    assert!(!engine.destroy_node(first));
    assert!(engine.set_root(second));
}

#[test]
fn destroying_subtrees_reclaims_layout_nodes_for_future_trees() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(DivStyle::default());
    for index in 0..1_000 {
        let row = engine.create_element(DivStyle::default());
        let text = engine.create_text(format!("row {index}"));
        assert!(engine.append_child(row, text));
        assert!(engine.append_child(root, row));
    }

    assert_eq!(engine.arena.stats().node_count, 2_001);
    assert!(engine.destroy_node(root));
    assert_eq!(engine.arena.stats().node_count, 0);

    let replacement_root = engine.create_element(DivStyle::default());
    for index in 0..1_000 {
        let row = engine.create_element(DivStyle::default());
        let text = engine.create_text(format!("replacement {index}"));
        assert!(engine.append_child(row, text));
        assert!(engine.append_child(replacement_root, row));
    }

    let stats = engine.arena.stats();
    assert_eq!(stats.node_count, 2_001);
    assert_eq!(stats.allocated_slot_count, 2_001);
}

#[test]
fn invalid_dom_ids_do_not_mutate_or_panic() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(2.0),
        CssDimension::Length(1.0),
    ));
    let text = engine.create_text("ok");
    let missing = DomId(99_999);
    assert!(engine.append_child(root, text));
    assert!(engine.set_root(root));

    assert!(!engine.append_child(root, missing));
    assert!(!engine.set_root(missing));
    assert!(!engine.set_text(missing, "nope"));
    assert!(!engine.set_style(missing, DivStyle::default()));
    assert_eq!(engine.scroll_metrics(missing), None);

    let frame = engine.render_frame(2, 1).unwrap();
    assert_eq!(frame.cell(0, 0).unwrap().character, 'o');
    assert_eq!(frame.cell(1, 0).unwrap().character, 'k');
}

#[test]
fn destroying_child_detaches_it_from_layout_and_hit_testing() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
    root_style.background = Background::Blue;
    let root = engine.create_element(root_style);

    let mut child_style = block_style(CssDimension::Length(1.0), CssDimension::Length(1.0));
    child_style.background = Background::Red;
    let child = engine.create_element(child_style);

    assert!(engine.append_child(root, child));
    assert!(engine.set_root(root));
    engine.render_frame(4, 1).unwrap();
    assert_eq!(engine.target_at(0, 0), Some(child));

    assert!(engine.destroy_node(child));
    engine.render_frame(4, 1).unwrap();

    assert_eq!(engine.target_at(0, 0), Some(root));
    assert!(!engine.set_root(child));
}

#[test]
fn detaching_child_detaches_without_destroying_it() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(4.0),
        CssDimension::Length(1.0),
    ));
    let child = engine.create_element(block_style(
        CssDimension::Length(1.0),
        CssDimension::Length(1.0),
    ));

    assert!(engine.append_child(root, child));
    assert!(engine.set_root(root));
    engine.render_frame(4, 1).unwrap();

    assert!(engine.detach_node(child));
    engine.render_frame(4, 1).unwrap();
    assert_eq!(engine.target_at(0, 0), Some(root));

    assert!(engine.append_child(root, child));
    engine.render_frame(4, 1).unwrap();
    assert_eq!(engine.target_at(0, 0), Some(child));
}

#[test]
fn destroying_root_clears_render_output() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(2.0),
        CssDimension::Length(1.0),
    ));
    assert!(engine.set_root(root));

    assert!(engine.render_frame(2, 1).is_some());
    assert!(engine.destroy_node(root));

    assert!(engine.render_frame(2, 1).is_none());
}

#[test]
fn selection_action_uses_current_layout_frame() {
    let (mut engine, _viewport) = scroll_engine();
    engine.render_frame(5, 1).unwrap();

    engine.handle_selection_event(SelectionMouseEvent {
        event_type: SelectionMouseEventType::Down,
        x: 0,
        y: 0,
        button: 0,
    });
    assert_eq!(
        engine.handle_selection_event(SelectionMouseEvent {
            event_type: SelectionMouseEventType::Up,
            x: 4,
            y: 0,
            button: 0,
        }),
        SelectionAction::CopyToClipboard("aaaaa".to_string())
    );
}

#[test]
fn frame_flush_diffs_against_previous_frame() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(4.0),
        CssDimension::Length(1.0),
    ));
    let text = engine.create_text("ab");
    engine.append_child(root, text);
    engine.set_root(root);

    let mut first = Vec::new();
    engine
        .flush_frame_to(4, 1, &mut first, TermProfile::NoColor, false)
        .unwrap();
    engine.set_text(text, "ac");
    let mut second = Vec::new();
    engine
        .flush_frame_to(4, 1, &mut second, TermProfile::NoColor, false)
        .unwrap();
    let second = String::from_utf8(second).unwrap();

    assert!(second.contains("\x1b[1;2Hc"));
    assert!(!second.contains("\x1b[H"));
}

#[test]
fn frame_flush_writes_and_flushes_output() {
    struct FlushProbe {
        bytes: Vec<u8>,
        flushes: usize,
    }

    impl std::io::Write for FlushProbe {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.bytes.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            self.flushes += 1;
            Ok(())
        }
    }

    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(2.0),
        CssDimension::Length(1.0),
    ));
    let text = engine.create_text("ok");
    engine.append_child(root, text);
    engine.set_root(root);

    let mut out = FlushProbe {
        bytes: Vec::new(),
        flushes: 0,
    };
    engine
        .flush_frame_to(2, 1, &mut out, TermProfile::NoColor, false)
        .unwrap();

    assert!(String::from_utf8(out.bytes).unwrap().contains("ok"));
    assert_eq!(out.flushes, 1);
}

#[test]
fn resize_recomputes_layout() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(CssDimension::Percent(1.0), CssDimension::Auto));
    let text = engine.create_text("hello world");
    engine.append_child(root, text);
    engine.set_root(root);

    engine.render_frame(5, 5).unwrap();
    let passes = engine.layout_passes();
    engine.render_frame(10, 5).unwrap();

    assert_eq!(engine.layout_passes(), passes + 1);
}

#[test]
fn invalidating_frame_forces_full_repaint() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(4.0),
        CssDimension::Length(1.0),
    ));
    let text = engine.create_text("ab");
    engine.append_child(root, text);
    engine.set_root(root);

    let mut first = Vec::new();
    engine
        .flush_frame_to(4, 1, &mut first, TermProfile::NoColor, false)
        .unwrap();
    engine.invalidate_frame();
    let mut second = Vec::new();
    engine
        .flush_frame_to(4, 1, &mut second, TermProfile::NoColor, false)
        .unwrap();
    let second = String::from_utf8(second).unwrap();

    assert!(second.contains("\x1b[H"));
}

#[test]
fn text_attribute_change_paints_without_recomputing_layout() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(4.0),
        CssDimension::Length(1.0),
    ));
    let text = engine.create_text("ok");
    engine.append_child(root, text);
    engine.set_root(root);

    engine.render_frame(4, 1).unwrap();
    let passes = engine.layout_passes();
    assert!(engine.mutate_style(root, StyleMutation::FontWeight(CssFontWeight::Bold)));
    let frame = engine.render_frame(4, 1).unwrap();

    assert_eq!(engine.layout_passes(), passes);
    assert!(frame.cell(0, 0).unwrap().bold);
}

#[test]
fn z_index_change_reorders_paint_without_recomputing_layout() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(4.0), CssDimension::Length(2.0));
    root_style.position = CssPosition::Relative;
    let root = engine.create_element(root_style);
    let mut red_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    red_style.position = CssPosition::Absolute;
    red_style.z_index = CssZIndex::Integer(1);
    red_style.background = Background::Red;
    let red = engine.create_element(red_style);
    let mut blue_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    blue_style.position = CssPosition::Absolute;
    blue_style.z_index = CssZIndex::Integer(2);
    blue_style.background = Background::Blue;
    let blue = engine.create_element(blue_style);
    engine.append_child(root, red);
    engine.append_child(root, blue);
    engine.set_root(root);

    let first = engine.render_frame(4, 2).unwrap();
    assert_eq!(first.cell(0, 0).unwrap().background, Background::Blue);
    let passes = engine.layout_passes();
    assert!(engine.mutate_style(red, StyleMutation::ZIndex(CssZIndex::Integer(3))));
    let second = engine.render_frame(4, 2).unwrap();

    assert_eq!(engine.layout_passes(), passes);
    assert_eq!(second.cell(0, 0).unwrap().background, Background::Red);
}

#[test]
fn opacity_change_repaints_without_recomputing_layout() {
    let mut engine = PaintEngine::new();
    let mut style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    style.background = Background::Red;
    let root = engine.create_element(style);
    engine.set_root(root);

    engine.render_frame(2, 1).unwrap();
    let passes = engine.layout_passes();
    assert!(engine.mutate_style(root, StyleMutation::Opacity(0.5)));
    let frame = engine.render_frame(2, 1).unwrap();

    assert_eq!(engine.layout_passes(), passes);
    assert_eq!(
        frame.cell(0, 0).unwrap().background,
        Background::Rgb(128, 0, 0)
    );
}

#[test]
fn opacity_transition_fades_in_as_a_stacking_context_without_recomputing_layout() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    root_style.background = Background::Blue;
    let root = engine.create_element(root_style);
    let mut child_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    child_style.background = Background::Red;
    child_style.opacity = 0.0;
    let child = engine.create_element(child_style.clone());
    engine.append_child(root, child);
    engine.set_root(root);
    let start = std::time::Instant::now();
    engine.render_frame_at(2, 1, start).unwrap();
    let passes = engine.layout_passes();

    engine.set_transition(
        child,
        vec![TransitionSpec {
            property: TransitionProperty::Opacity,
            duration_ms: 100,
        }],
    );
    child_style.opacity = 1.0;
    engine.set_style_at(child, child_style, start);

    assert!(engine.has_active_transitions());
    assert_eq!(
        engine.drain_transition_events(),
        vec![EngineTransitionEvent {
            event_type: TransitionEventType::Start,
            target: child,
            property: TransitionProperty::Opacity,
        }]
    );
    engine.dirtiness = Dirtiness::Clean;
    let mut output = Vec::new();
    assert!(engine
        .flush_if_dirty_to(
            2,
            1,
            &mut output,
            TermProfile::TrueColor,
            false,
            start + Duration::from_millis(50),
        )
        .unwrap());
    let midway = engine.current_frame.as_ref().unwrap();
    assert_eq!(engine.layout_passes(), passes);
    assert_eq!(
        midway.cell(0, 0).unwrap().background,
        Background::Rgb(128, 0, 128)
    );

    let finished = engine
        .render_frame_at(2, 1, start + std::time::Duration::from_millis(100))
        .unwrap();
    assert_eq!(engine.layout_passes(), passes);
    assert_eq!(finished.cell(0, 0).unwrap().background, Background::Red);
    assert!(!engine.has_active_transitions());
    assert_eq!(
        engine.drain_transition_events(),
        vec![EngineTransitionEvent {
            event_type: TransitionEventType::End,
            target: child,
            property: TransitionProperty::Opacity,
        }]
    );
}

#[test]
fn initial_opacity_does_not_transition_from_the_internal_default() {
    let mut engine = PaintEngine::new();
    let mut root_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    root_style.background = Background::Blue;
    let root = engine.create_element(root_style);
    let mut overlay_style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    overlay_style.background = Background::Black;
    let overlay = engine.create_element(overlay_style.clone());
    engine.append_child(root, overlay);
    engine.set_root(root);
    let start = std::time::Instant::now();

    engine.set_transition(
        overlay,
        vec![TransitionSpec {
            property: TransitionProperty::Opacity,
            duration_ms: 200,
        }],
    );
    overlay_style.opacity = 0.5;
    engine.set_style_at(overlay, overlay_style, start);

    let frame = engine.render_frame_at(2, 1, start).unwrap();
    assert_eq!(
        frame.cell(0, 0).unwrap().background,
        Background::Rgb(0, 0, 128)
    );
    assert!(engine.drain_transition_events().is_empty());
}

#[test]
fn color_transition_paints_without_recomputing_layout() {
    let mut engine = PaintEngine::new();
    let mut style = block_style(CssDimension::Length(4.0), CssDimension::Length(1.0));
    style.background = Background::Rgb(0, 0, 255);
    let root = engine.create_element(style.clone());
    engine.set_root(root);
    let start = std::time::Instant::now();
    engine.render_frame_at(4, 1, start).unwrap();
    let passes = engine.layout_passes();

    engine.set_transition(
        root,
        vec![TransitionSpec {
            property: TransitionProperty::BackgroundColor,
            duration_ms: 100,
        }],
    );
    style.background = Background::Rgb(0, 255, 255);
    engine.set_style_at(root, style, start);

    assert_eq!(engine.layout_passes(), passes);
    assert_eq!(
        engine.drain_transition_events(),
        vec![EngineTransitionEvent {
            event_type: TransitionEventType::Start,
            target: root,
            property: TransitionProperty::BackgroundColor,
        }]
    );

    let midway = engine
        .render_frame_at(4, 1, start + std::time::Duration::from_millis(50))
        .unwrap();
    let midway_background = midway.cell(0, 0).unwrap().background;
    assert_ne!(midway_background, Background::Rgb(0, 0, 255));
    assert_ne!(midway_background, Background::Rgb(0, 255, 255));
    assert_eq!(engine.layout_passes(), passes);

    let finished = engine
        .render_frame_at(4, 1, start + std::time::Duration::from_millis(100))
        .unwrap();
    assert_eq!(
        finished.cell(0, 0).unwrap().background,
        Background::Rgb(0, 255, 255)
    );
    assert_eq!(
        engine.drain_transition_events(),
        vec![EngineTransitionEvent {
            event_type: TransitionEventType::End,
            target: root,
            property: TransitionProperty::BackgroundColor,
        }]
    );
}

#[test]
fn truecolor_disabled_skips_transition() {
    let mut engine = PaintEngine::new();
    engine.set_truecolor_enabled(false);
    let mut style = block_style(CssDimension::Length(2.0), CssDimension::Length(1.0));
    style.color = Background::Rgb(255, 0, 0);
    let root = engine.create_element(style.clone());
    let text = engine.create_text("x");
    engine.append_child(root, text);
    engine.set_root(root);
    let start = std::time::Instant::now();
    engine.render_frame_at(2, 1, start).unwrap();

    engine.set_transition(
        root,
        vec![TransitionSpec {
            property: TransitionProperty::Color,
            duration_ms: 100,
        }],
    );
    style.color = Background::Rgb(0, 255, 0);
    engine.set_style_at(root, style, start);

    let frame = engine
        .render_frame_at(2, 1, start + std::time::Duration::from_millis(50))
        .unwrap();
    assert_eq!(
        frame.cell(0, 0).unwrap().foreground,
        Background::Rgb(0, 255, 0)
    );
    assert!(engine.drain_transition_events().is_empty());
}

#[test]
fn input_value_command_updates_textarea_for_typescript_compatibility() {
    let mut engine = PaintEngine::new();
    let root = engine.create_element(block_style(
        CssDimension::Length(8.0),
        CssDimension::Length(2.0),
    ));
    let textarea = engine.create_textarea(
        block_style(CssDimension::Length(8.0), CssDimension::Auto),
        "",
    );
    engine.append_child(root, textarea);
    engine.set_root(root);

    assert!(engine.set_input_value(textarea, "hello", 5));
    let frame = engine.render_frame(8, 2).unwrap();

    assert_eq!(frame.cell(0, 0).unwrap().character, 'h');
    assert_eq!(frame.cell(4, 0).unwrap().character, 'o');
}

#[test]
fn textarea_vertical_cursor_move_uses_soft_wrapped_rows() {
    let mut engine = PaintEngine::new();
    let textarea = engine.create_textarea(
        block_style(CssDimension::Length(6.0), CssDimension::Auto),
        "abcd efgh",
    );
    engine.set_root(textarea);
    engine.set_textarea_value(textarea, "abcd efgh", 7);
    engine.set_textarea_focused(textarea, true);

    assert_eq!(
        engine.move_textarea_cursor_vertically_for_size(textarea, -1, 6, 3),
        Some(2)
    );

    let frame = engine.render_frame(6, 3).unwrap();
    assert!(frame.cell(2, 0).unwrap().reversed);
}

#[test]
fn textarea_cursor_visual_position_query_uses_current_layout_without_relayout() {
    let mut engine = PaintEngine::new();
    let textarea = engine.create_textarea(
        block_style(CssDimension::Length(4.0), CssDimension::Auto),
        "hahahaha",
    );
    engine.set_root(textarea);
    engine.set_textarea_value(textarea, "hahahaha", 5);

    assert_eq!(
        engine.textarea_cursor_visual_position_for_size(textarea, 8, 4),
        Some((1, 1))
    );
    let layout_passes = engine.layout_passes();

    assert_eq!(
        engine.textarea_cursor_visual_position_for_size(textarea, 8, 4),
        Some((1, 1))
    );
    assert_eq!(
        engine.textarea_visual_line_range_for_size(textarea, 1, 8, 4),
        Some((4, 8))
    );
    assert_eq!(engine.layout_passes(), layout_passes);
}

#[test]
fn clicking_input_moves_cursor_to_clicked_column() {
    let mut engine = PaintEngine::new();
    let input = engine.create_input_with_id(
        DomId(1),
        block_style(CssDimension::Length(6.0), CssDimension::Length(1.0)),
        "abcdef",
    );
    engine.set_root(input);
    engine.set_input_focused(input, true);

    assert_eq!(
        engine.set_text_control_cursor_at_point_for_size(input, 3, 0, 6, 1),
        Some(3)
    );

    let frame = engine.render_frame(6, 1).unwrap();
    assert!(frame.cell(3, 0).unwrap().reversed);
}

#[test]
fn terminal_focus_changes_cursor_without_recomputing_layout() {
    let mut engine = PaintEngine::new();
    let input = engine.create_input_with_id(
        DomId(1),
        block_style(CssDimension::Length(6.0), CssDimension::Length(1.0)),
        "abcdef",
    );
    engine.set_root(input);
    engine.set_input_focused(input, true);
    engine.set_input_value(input, "abcdef", 3);

    let focused_frame = engine.render_frame(6, 1).unwrap();
    assert!(focused_frame.cell(3, 0).unwrap().reversed);
    let layout_passes = engine.layout_passes();

    engine.set_terminal_focused(false);
    let blurred_frame = engine.render_frame(6, 1).unwrap();
    assert!(!blurred_frame.cell(3, 0).unwrap().reversed);
    assert_eq!(engine.layout_passes(), layout_passes);

    engine.set_terminal_focused(true);
    let refocused_frame = engine.render_frame(6, 1).unwrap();
    assert!(refocused_frame.cell(3, 0).unwrap().reversed);
    assert_eq!(engine.layout_passes(), layout_passes);
}

#[test]
fn initially_blurred_terminal_hides_cursor_on_first_frame() {
    let mut engine = PaintEngine::new();
    let input = engine.create_input_with_id(
        DomId(1),
        block_style(CssDimension::Length(6.0), CssDimension::Length(1.0)),
        "abcdef",
    );
    engine.set_root(input);
    engine.set_input_focused(input, true);
    engine.set_input_value(input, "abcdef", 3);
    engine.set_terminal_focused(false);

    let frame = engine.render_frame(6, 1).unwrap();

    assert!(!frame.cell(3, 0).unwrap().reversed);
}

#[test]
fn clicking_textarea_uses_soft_wrapped_visual_position() {
    let mut engine = PaintEngine::new();
    let textarea = engine.create_textarea(
        block_style(CssDimension::Length(6.0), CssDimension::Auto),
        "abcd efgh",
    );
    engine.set_root(textarea);
    engine.set_textarea_focused(textarea, true);

    assert_eq!(
        engine.set_text_control_cursor_at_point_for_size(textarea, 2, 1, 6, 3),
        Some(7)
    );

    let frame = engine.render_frame(6, 3).unwrap();
    assert!(frame.cell(2, 1).unwrap().reversed);
}
