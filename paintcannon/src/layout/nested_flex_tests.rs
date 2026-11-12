use super::*;
use crate::style::{
    BorderStyle, CssLengthPercentage, LayoutAlignItems, LayoutFlexDirection, LayoutJustifyContent,
};

fn column() -> DivStyle {
    DivStyle {
        display: LayoutDisplay::Flex,
        flex_direction: LayoutFlexDirection::Column,
        white_space: CssWhiteSpace::PreWrap,
        ..DivStyle::default()
    }
}

fn flexible() -> DivStyle {
    DivStyle {
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: CssDimension::Length(0.0),
        min_width: CssDimension::Length(0.0),
        min_height: CssDimension::Length(0.0),
        ..column()
    }
}

#[test]
fn nested_flex_measurements_do_not_grow_exponentially() {
    let mut arena = LayoutArena::new();
    let leaf = arena.create_element(DivStyle {
        width: CssDimension::Length(10.0),
        height: CssDimension::Length(1.0),
        ..DivStyle::default()
    });
    let depth = 12;
    let mut child = leaf;
    for _ in 0..depth {
        let parent = arena.create_element(DivStyle {
            flex_shrink: 0.0,
            ..column()
        });
        arena.append_child(parent, child);
        child = parent;
    }
    let root = arena.create_element(DivStyle {
        width: CssDimension::Percent(1.0),
        height: CssDimension::Percent(1.0),
        overflow_y: LayoutOverflow::Scroll,
        scrollbar_gutter: ScrollbarGutter::Stable,
        ..column()
    });
    arena.append_child(root, child);
    arena.compute_layout(
        root,
        Size {
            width: AvailableSpace::Definite(80.0),
            height: AvailableSpace::Definite(30.0),
        },
    );
    assert_eq!(
        arena.layout(leaf).size,
        Size {
            width: 10.0,
            height: 1.0
        }
    );
    let calls = arena.profile_stats().uncached_layout_calls;
    // Allow ample room for intrinsic/final sizing passes, but not repeated
    // subtree measurements caused by hot cache entries evicting one another.
    assert!(
        calls < (depth + 2) * 100,
        "{calls} uncached layout calls for {} nodes",
        depth + 2
    );
}

fn text(arena: &mut LayoutArena, parent: NodeId, value: &str) {
    let span = arena.create_element(DivStyle {
        display: LayoutDisplay::Inline,
        ..DivStyle::default()
    });
    let text = arena.create_text(value);
    arena.append_child(span, text);
    arena.append_child(parent, span);
}

#[test]
fn textarea_newlines_shrink_nested_transcript_without_manufacturing_a_gap() {
    check_nested_transcript(false);
}

#[test]
fn zero_basis_boot_also_keeps_transcript_scroll_geometry_current() {
    check_nested_transcript(true);
}

fn check_nested_transcript(zero_basis_boot: bool) {
    let mut arena = LayoutArena::new();
    let root = arena.create_element(DivStyle {
        width: CssDimension::Percent(1.0),
        height: CssDimension::Percent(1.0),
        padding_top: CssLengthPercentage::Length(1.0),
        padding_right: CssLengthPercentage::Length(1.0),
        padding_bottom: CssLengthPercentage::Length(1.0),
        padding_left: CssLengthPercentage::Length(1.0),
        ..column()
    });
    let shell = arena.create_element(DivStyle {
        border_top: BorderStyle::ChunkyRounded,
        border_right: BorderStyle::ChunkyRounded,
        border_bottom: BorderStyle::ChunkyRounded,
        border_left: BorderStyle::ChunkyRounded,
        ..flexible()
    });
    let transcript = arena.create_element(DivStyle {
        overflow_y: LayoutOverflow::Scroll,
        scrollbar_gutter: ScrollbarGutter::Stable,
        overflow_anchor: CssOverflowAnchor::None,
        ..flexible()
    });
    let inner = arena.create_element(DivStyle {
        min_height: CssDimension::Percent(1.0),
        flex_shrink: 0.0,
        overflow_wrap: CssOverflowWrap::Anywhere,
        ..column()
    });
    let boot = arena.create_element(DivStyle {
        width: CssDimension::Percent(1.0),
        flex_grow: 1.0,
        flex_shrink: 1.0,
        flex_basis: if zero_basis_boot {
            CssDimension::Length(0.0)
        } else {
            CssDimension::Auto
        },
        min_height: if zero_basis_boot {
            CssDimension::Length(0.0)
        } else {
            CssDimension::Auto
        },
        align_items: Some(LayoutAlignItems::Center),
        justify_content: Some(LayoutJustifyContent::Center),
        margin_top: CssLengthPercentageAuto::Length(1.0),
        margin_bottom: CssLengthPercentageAuto::Length(1.0),
        ..column()
    });
    text(
        &mut arena,
        boot,
        "OCTO\nVersion: 0.0.46\nConfigured skills: example",
    );
    let slogan = arena.create_element(DivStyle {
        flex_direction: LayoutFlexDirection::Row,
        margin_left: CssLengthPercentageAuto::Length(1.0),
        margin_top: CssLengthPercentageAuto::Length(1.0),
        ..column()
    });
    text(
        &mut arena,
        slogan,
        "Octo is your friend. Tell Octo what you want to do.",
    );
    let history = arena.create_element(column());
    let bottom = arena.create_element(DivStyle {
        width: CssDimension::Percent(1.0),
        ..column()
    });
    let input_wrapper = arena.create_element(DivStyle {
        width: CssDimension::Percent(1.0),
        min_width: CssDimension::Length(0.0),
        border_right: BorderStyle::ChunkyBestEffort,
        border_bottom: BorderStyle::ChunkyBestEffort,
        border_left: BorderStyle::ChunkyBestEffort,
        ..column()
    });
    let textarea = arena.create_textarea(
        DivStyle {
            width: CssDimension::Percent(1.0),
            min_width: CssDimension::Length(0.0),
            min_height: CssDimension::Length(1.0),
            flex_grow: 1.0,
            border_top: BorderStyle::ChunkyBestEffort,
            border_right: BorderStyle::ChunkyBestEffort,
            border_bottom: BorderStyle::ChunkyBestEffort,
            border_left: BorderStyle::ChunkyBestEffort,
            ..column()
        },
        "x",
    );
    arena.append_child(root, shell);
    arena.append_child(shell, transcript);
    arena.append_child(shell, bottom);
    arena.append_child(transcript, inner);
    arena.append_child(inner, boot);
    arena.append_child(inner, slogan);
    arena.append_child(inner, history);
    arena.append_child(bottom, input_wrapper);
    arena.append_child(input_wrapper, textarea);

    // The only mutation is in the textarea, outside the transcript subtree.
    // Never scroll: even at offset zero, scrollHeight must follow the resized layout.
    for newlines in (0..=8).chain((0..8).rev()) {
        let value = format!("x{}", "\n".repeat(newlines));
        arena.set_textarea_value(textarea, &value, value.len() as u32);
        arena.compute_layout(
            root,
            Size {
                width: AvailableSpace::Definite(80.0),
                height: AvailableSpace::Definite(40.0),
            },
        );
        let metrics = arena.scroll_metrics(transcript).unwrap();
        let textarea_layout = arena.layout(textarea);
        let bottom_layout = arena.layout(bottom);
        let transcript_layout = arena.layout(transcript);
        let inner_layout = arena.layout(inner);
        let boot_layout = arena.layout(boot);
        let slogan_layout = arena.layout(slogan);
        let gap =
            metrics.client_height as f32 - (slogan_layout.location.y + slogan_layout.size.height);
        let rect = |node| {
            let layout = arena.layout(node);
            (
                layout.location.x,
                layout.location.y,
                layout.size.width,
                layout.size.height,
            )
        };
        eprintln!("newlines={newlines} textarea={:?} content_h={} bottom={:?} transcript={:?} metrics={metrics:?} inner={:?} boot={:?} slogan={:?} gap={gap} overflow(inner,boot)=({:?},{:?})",
            rect(textarea), arena.layout(textarea).content_box_size().height,
            rect(bottom), rect(transcript), rect(inner), rect(boot), rect(slogan),
            arena.visible_overflow_size(inner), arena.visible_overflow_size(boot));
        assert_eq!(metrics.scroll_top, 0, "newlines={newlines}");
        assert_eq!(textarea_layout.size.height, 3.0 + newlines as f32);
        assert_eq!(
            textarea_layout.content_box_size().height,
            1.0 + newlines as f32
        );
        assert_eq!(bottom_layout.size.height, 4.0 + newlines as f32);
        assert_eq!(
            metrics.client_height as f32 + bottom_layout.size.height,
            36.0
        );
        assert_eq!(transcript_layout.size.height, metrics.client_height as f32);
        assert_eq!(transcript_layout.location.y, 1.0);
        assert_eq!(
            bottom_layout.location.y,
            transcript_layout.location.y + transcript_layout.size.height
        );
        assert_eq!(
            metrics.scroll_height, metrics.client_height,
            "newlines={newlines}"
        );
        assert_eq!(inner_layout.location.y, 0.0);
        assert_eq!(
            inner_layout.size.height, metrics.client_height as f32,
            "newlines={newlines}"
        );
        assert_eq!(
            boot_layout.size.height,
            inner_layout.size.height - 4.0,
            "newlines={newlines}"
        );
        assert_eq!(
            arena.visible_overflow_size(inner).height,
            inner_layout.size.height
        );
        assert_eq!(
            arena.visible_overflow_size(boot).height,
            boot_layout.size.height
        );
        assert_eq!(gap, 0.0, "newlines={newlines}, slogan={slogan_layout:?}");
    }
}
