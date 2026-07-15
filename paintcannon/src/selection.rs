use crate::frame::{Frame, Selection, SelectionPoint};

const MIN_SELECTION_DRAG_DELTA_CELLS: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelectionMouseEvent {
    pub(crate) event_type: SelectionMouseEventType,
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) button: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionMouseEventType {
    Down,
    Drag,
    Scroll,
    Up,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SelectionAction {
    None,
    Redraw,
    CopyToClipboard(String),
}

#[derive(Default)]
pub(crate) struct SelectionState {
    selection: Option<ActiveSelection>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveSelection {
    anchor: SelectionPoint,
    focus: SelectionPoint,
    selecting: bool,
    moved: bool,
    origin_x: u32,
    origin_y: u32,
    last_x: u32,
    last_y: u32,
}

impl SelectionState {
    pub(crate) fn is_selecting(&self) -> bool {
        self.selection.is_some_and(|selection| selection.selecting)
    }

    pub(crate) fn active_selection(&self) -> Option<Selection> {
        self.selection
            .filter(|selection| selection.moved)
            .map(|selection| Selection {
                anchor: selection.anchor,
                focus: selection.focus,
            })
    }

    pub(crate) fn handle_event(
        &mut self,
        event: SelectionMouseEvent,
        frame: Option<&Frame>,
    ) -> SelectionAction {
        match event.event_type {
            SelectionMouseEventType::Down => {
                let Some(point) =
                    frame.and_then(|frame| frame.selection_point_for(event.x, event.y))
                else {
                    self.selection = None;
                    return SelectionAction::Redraw;
                };

                self.selection = Some(ActiveSelection {
                    anchor: point,
                    focus: point,
                    selecting: true,
                    moved: false,
                    origin_x: event.x,
                    origin_y: event.y,
                    last_x: event.x,
                    last_y: event.y,
                });
                SelectionAction::Redraw
            }
            SelectionMouseEventType::Drag => {
                if event.button != 0 {
                    return SelectionAction::None;
                }
                let Some(point) =
                    frame.and_then(|frame| frame.selection_point_for(event.x, event.y))
                else {
                    return SelectionAction::None;
                };
                let Some(selection) = self
                    .selection
                    .as_mut()
                    .filter(|selection| selection.selecting)
                else {
                    return SelectionAction::None;
                };

                let reached_threshold = selection.moved
                    || selection_drag_delta_reached(
                        selection.origin_x,
                        selection.origin_y,
                        event.x,
                        event.y,
                    );
                let changed = reached_threshold && selection.focus != point;
                selection.moved = selection.moved || changed;
                if reached_threshold {
                    selection.focus = point;
                }
                selection.last_x = event.x;
                selection.last_y = event.y;
                if changed {
                    SelectionAction::Redraw
                } else {
                    SelectionAction::None
                }
            }
            SelectionMouseEventType::Scroll => {
                let Some(selection) = self
                    .selection
                    .as_mut()
                    .filter(|selection| selection.selecting)
                else {
                    return SelectionAction::None;
                };
                selection.last_x = event.x;
                selection.last_y = event.y;
                let mut changed = false;
                let reached_threshold = selection.moved
                    || selection_drag_delta_reached(
                        selection.origin_x,
                        selection.origin_y,
                        event.x,
                        event.y,
                    );
                if reached_threshold {
                    if let Some(point) =
                        frame.and_then(|frame| frame.selection_point_for(event.x, event.y))
                    {
                        changed = selection.focus != point;
                        selection.moved = selection.moved || changed;
                        selection.focus = point;
                    }
                }
                if changed {
                    SelectionAction::Redraw
                } else {
                    SelectionAction::None
                }
            }
            SelectionMouseEventType::Up => {
                if event.button != 0 {
                    return SelectionAction::None;
                }

                let point = frame.and_then(|frame| frame.selection_point_for(event.x, event.y));
                let mut should_copy = false;
                if let Some(selection) = self
                    .selection
                    .as_mut()
                    .filter(|selection| selection.selecting)
                {
                    if let Some(point) = point {
                        let reached_threshold = selection.moved
                            || selection_drag_delta_reached(
                                selection.origin_x,
                                selection.origin_y,
                                event.x,
                                event.y,
                            );
                        if reached_threshold {
                            selection.moved = selection.moved || selection.focus != point;
                            selection.focus = point;
                        }
                    }
                    selection.last_x = event.x;
                    selection.last_y = event.y;
                    selection.selecting = false;
                    should_copy = selection.moved;
                }

                let selected_text = should_copy.then(|| {
                    frame.and_then(|frame| {
                        self.active_selection()
                            .as_ref()
                            .and_then(|selection| frame.selected_text(selection))
                    })
                });
                self.selection = None;

                match selected_text.flatten() {
                    Some(text) => SelectionAction::CopyToClipboard(text),
                    None => SelectionAction::Redraw,
                }
            }
        }
    }

    pub(crate) fn refresh_focus_from_last_pointer(&mut self, frame: &Frame) -> bool {
        let Some(selection) = self
            .selection
            .as_mut()
            .filter(|selection| selection.selecting)
        else {
            return false;
        };
        let Some(point) = frame.selection_point_for(selection.last_x, selection.last_y) else {
            return false;
        };
        if !selection.moved
            && !selection_drag_delta_reached(
                selection.origin_x,
                selection.origin_y,
                selection.last_x,
                selection.last_y,
            )
        {
            return false;
        }

        let changed = selection.focus != point;
        selection.moved = selection.moved || changed;
        selection.focus = point;
        changed
    }
}

fn selection_drag_delta_reached(origin_x: u32, origin_y: u32, x: u32, y: u32) -> bool {
    origin_x.abs_diff(x).max(origin_y.abs_diff(y)) >= MIN_SELECTION_DRAG_DELTA_CELLS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{ClipBounds, GlyphStyle};
    use crate::layout::LayoutArena;
    use crate::paint::paint_arena;
    use crate::style::{CssDimension, DivStyle, LayoutFlexDirection, LayoutOverflow};
    use taffy::{AvailableSpace, Size};

    fn text_frame(text: &str) -> Frame {
        let mut frame = Frame::new(text.len(), 1, false);
        for (index, character) in text.chars().enumerate() {
            frame.write_glyph(
                index as i32,
                0,
                character,
                1,
                GlyphStyle::default(),
                ClipBounds::unbounded(),
            );
        }
        frame
    }

    fn block_style(width: CssDimension, height: CssDimension) -> DivStyle {
        DivStyle {
            width,
            height,
            ..DivStyle::default()
        }
    }

    fn scroll_text_arena() -> (LayoutArena, taffy::NodeId) {
        let mut arena = LayoutArena::new();
        let mut viewport_style = block_style(CssDimension::Length(6.0), CssDimension::Length(1.0));
        viewport_style.overflow_y = LayoutOverflow::Scroll;
        let viewport = arena.create_element(viewport_style);
        let mut content_style = block_style(CssDimension::Length(5.0), CssDimension::Auto);
        content_style.display = crate::style::LayoutDisplay::Flex;
        content_style.flex_direction = LayoutFlexDirection::Column;
        let content = arena.create_element(content_style);

        for text in ["aaaaa", "bbbbb"] {
            let row =
                arena.create_element(block_style(CssDimension::Length(5.0), CssDimension::Auto));
            let text = arena.create_text(text);
            arena.append_child(row, text);
            arena.append_child(content, row);
        }
        arena.append_child(viewport, content);
        arena.compute_layout(
            viewport,
            Size {
                width: AvailableSpace::Definite(6.0),
                height: AvailableSpace::Definite(1.0),
            },
        );

        (arena, viewport)
    }

    #[test]
    fn drag_then_up_returns_copy_action_and_clears_selection() {
        let frame = text_frame("hello");
        let mut state = SelectionState::default();

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Down,
                    x: 1,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::Redraw
        );
        assert_eq!(state.active_selection(), None);
        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Drag,
                    x: 3,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::Redraw
        );
        assert_eq!(
            state.active_selection(),
            Some(Selection {
                anchor: SelectionPoint { order: 1 },
                focus: SelectionPoint { order: 3 },
            })
        );
        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Up,
                    x: 3,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::CopyToClipboard("ell".to_string())
        );
        assert_eq!(state.active_selection(), None);
    }

    #[test]
    fn drag_within_same_cell_does_not_show_or_copy_selection() {
        let frame = text_frame("hello");
        let mut state = SelectionState::default();

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Down,
                    x: 1,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::Redraw
        );
        assert_eq!(state.active_selection(), None);

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Drag,
                    x: 1,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::None
        );
        assert_eq!(state.active_selection(), None);

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Up,
                    x: 1,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::Redraw
        );
        assert_eq!(state.active_selection(), None);
    }

    #[test]
    fn one_cell_drag_starts_selection() {
        let frame = text_frame("hello");
        let mut state = SelectionState::default();

        state.handle_event(
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Down,
                x: 1,
                y: 0,
                button: 0,
            },
            Some(&frame),
        );

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Drag,
                    x: 2,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::Redraw
        );
        assert_eq!(
            state.active_selection(),
            Some(Selection {
                anchor: SelectionPoint { order: 1 },
                focus: SelectionPoint { order: 2 },
            })
        );

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Up,
                    x: 2,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::CopyToClipboard("el".to_string())
        );
        assert_eq!(state.active_selection(), None);
    }

    #[test]
    fn click_without_drag_only_redraws_and_clears() {
        let frame = text_frame("hello");
        let mut state = SelectionState::default();

        state.handle_event(
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Down,
                x: 1,
                y: 0,
                button: 0,
            },
            Some(&frame),
        );

        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Up,
                    x: 1,
                    y: 0,
                    button: 0,
                },
                Some(&frame),
            ),
            SelectionAction::Redraw
        );
        assert_eq!(state.active_selection(), None);
    }

    #[test]
    fn refresh_focus_uses_last_pointer_after_scroll_repaint() {
        let old_frame = text_frame("abcd");
        let new_frame = text_frame("wxyz");
        let mut state = SelectionState::default();

        state.handle_event(
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Down,
                x: 0,
                y: 0,
                button: 0,
            },
            Some(&old_frame),
        );
        state.handle_event(
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Scroll,
                x: 3,
                y: 0,
                button: 0,
            },
            Some(&old_frame),
        );

        assert!(!state.refresh_focus_from_last_pointer(&new_frame));
        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Up,
                    x: 3,
                    y: 0,
                    button: 0,
                },
                Some(&new_frame),
            ),
            SelectionAction::CopyToClipboard("wxyz".to_string())
        );
    }

    #[test]
    fn layout_clipping_limits_visible_selection_points() {
        let (arena, viewport) = scroll_text_arena();
        let output = paint_arena(&arena, viewport, 5, 2, false);

        assert_eq!(
            output.frame.selection_point_for(0, 0),
            Some(SelectionPoint { order: 0 })
        );
        assert_eq!(output.frame.selection_point_for(0, 1), None);
    }

    #[test]
    fn selecting_while_scrolling_keeps_original_hidden_anchor() {
        let (mut arena, viewport) = scroll_text_arena();
        let before_scroll = paint_arena(&arena, viewport, 5, 1, true);
        let mut state = SelectionState::default();

        state.handle_event(
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Down,
                x: 0,
                y: 0,
                button: 0,
            },
            Some(&before_scroll.frame),
        );
        state.handle_event(
            SelectionMouseEvent {
                event_type: SelectionMouseEventType::Scroll,
                x: 4,
                y: 0,
                button: 0,
            },
            Some(&before_scroll.frame),
        );

        arena.set_scroll_offset(viewport, 0, 1);
        let after_scroll = paint_arena(&arena, viewport, 5, 1, true);
        assert!(state.refresh_focus_from_last_pointer(&after_scroll.frame));
        assert_eq!(
            state.handle_event(
                SelectionMouseEvent {
                    event_type: SelectionMouseEventType::Up,
                    x: 4,
                    y: 0,
                    button: 0,
                },
                Some(&after_scroll.frame),
            ),
            SelectionAction::CopyToClipboard("aaaaa\nbbbbb".to_string())
        );
    }
}
