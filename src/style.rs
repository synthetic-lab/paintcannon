use napi::{Error, Result};
use taffy::geometry::Point;
use taffy::prelude::*;
use taffy::style::Overflow;

use crate::renderer::RenderCommand;

#[derive(Clone)]
pub(crate) struct DivStyle {
    pub(crate) display: LayoutDisplay,
    pub(crate) flex_direction: LayoutFlexDirection,
    pub(crate) flex_wrap: LayoutFlexWrap,
    pub(crate) flex_basis: CssDimension,
    pub(crate) flex_grow: f32,
    pub(crate) flex_shrink: f32,
    pub(crate) justify_content: Option<LayoutJustifyContent>,
    pub(crate) align_items: Option<LayoutAlignItems>,
    pub(crate) align_self: Option<LayoutAlignItems>,
    pub(crate) align_content: Option<LayoutJustifyContent>,
    pub(crate) justify_items: Option<LayoutAlignItems>,
    pub(crate) justify_self: Option<LayoutAlignItems>,
    pub(crate) row_gap: CssLengthPercentage,
    pub(crate) column_gap: CssLengthPercentage,
    pub(crate) width: CssDimension,
    pub(crate) height: CssDimension,
    pub(crate) grid_template_columns: Vec<CssGridTemplateTrack>,
    pub(crate) grid_template_rows: Vec<CssGridTemplateTrack>,
    pub(crate) grid_auto_columns: Vec<CssTrackSizing>,
    pub(crate) grid_auto_rows: Vec<CssTrackSizing>,
    pub(crate) grid_auto_flow: LayoutGridAutoFlow,
    pub(crate) grid_column: CssGridLine,
    pub(crate) grid_row: CssGridLine,
    pub(crate) background: Background,
    pub(crate) overflow: LayoutOverflow,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            display: LayoutDisplay::Block,
            flex_direction: LayoutFlexDirection::Row,
            flex_wrap: LayoutFlexWrap::NoWrap,
            flex_basis: CssDimension::Auto,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            justify_content: None,
            align_items: None,
            align_self: None,
            align_content: None,
            justify_items: None,
            justify_self: None,
            row_gap: CssLengthPercentage::Length(0.0),
            column_gap: CssLengthPercentage::Length(0.0),
            width: CssDimension::Auto,
            height: CssDimension::Auto,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_auto_columns: Vec::new(),
            grid_auto_rows: Vec::new(),
            grid_auto_flow: LayoutGridAutoFlow::Row,
            grid_column: CssGridLine::default(),
            grid_row: CssGridLine::default(),
            background: Background::Default,
            overflow: LayoutOverflow::Visible,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutDisplay {
    Inline,
    Block,
    Flex,
    Grid,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutOverflow {
    Visible,
    Hidden,
}

#[derive(Clone, Copy)]
pub(crate) enum LayoutFlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Copy)]
pub(crate) enum LayoutFlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Clone, Copy)]
pub(crate) enum LayoutJustifyContent {
    Start,
    FlexStart,
    Center,
    End,
    FlexEnd,
    Stretch,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

#[derive(Clone, Copy)]
pub(crate) enum LayoutAlignItems {
    Start,
    FlexStart,
    Center,
    End,
    FlexEnd,
    Baseline,
    Stretch,
}

#[derive(Clone, Copy)]
pub(crate) enum CssDimension {
    Auto,
    Length(f32),
    Percent(f32),
}

impl CssDimension {
    pub(crate) fn to_taffy(self) -> Dimension {
        match self {
            Self::Auto => Dimension::AUTO,
            Self::Length(value) => Dimension::length(value),
            Self::Percent(value) => Dimension::percent(value),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CssLengthPercentage {
    Length(f32),
    Percent(f32),
}

impl CssLengthPercentage {
    fn to_taffy(self) -> LengthPercentage {
        match self {
            Self::Length(value) => LengthPercentage::length(value),
            Self::Percent(value) => LengthPercentage::percent(value),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CssTrackSizing {
    Auto,
    Length(f32),
    Percent(f32),
    Fr(f32),
    MinContent,
    MaxContent,
}

impl CssTrackSizing {
    fn to_taffy(self) -> TrackSizingFunction {
        match self {
            Self::Auto => TrackSizingFunction::AUTO,
            Self::Length(value) => TrackSizingFunction::from_length(value),
            Self::Percent(value) => TrackSizingFunction::from_percent(value),
            Self::Fr(value) => TrackSizingFunction::from_fr(value),
            Self::MinContent => TrackSizingFunction::MIN_CONTENT,
            Self::MaxContent => TrackSizingFunction::MAX_CONTENT,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CssGridTemplateTrack {
    Single(CssTrackSizing),
}

impl CssGridTemplateTrack {
    fn to_taffy(self) -> GridTemplateComponent<String> {
        match self {
            Self::Single(track) => GridTemplateComponent::Single(track.to_taffy()),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum LayoutGridAutoFlow {
    Row,
    Column,
    RowDense,
    ColumnDense,
}

impl LayoutGridAutoFlow {
    fn to_taffy(self) -> GridAutoFlow {
        match self {
            Self::Row => GridAutoFlow::Row,
            Self::Column => GridAutoFlow::Column,
            Self::RowDense => GridAutoFlow::RowDense,
            Self::ColumnDense => GridAutoFlow::ColumnDense,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum CssGridPlacement {
    Auto,
    Line(i16),
    Span(u16),
}

impl CssGridPlacement {
    fn to_taffy(self) -> GridPlacement {
        match self {
            Self::Auto => GridPlacement::Auto,
            Self::Line(value) => GridPlacement::Line(value.into()),
            Self::Span(value) => GridPlacement::Span(value),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CssGridLine {
    pub(crate) start: CssGridPlacement,
    pub(crate) end: CssGridPlacement,
}

impl Default for CssGridLine {
    fn default() -> Self {
        Self {
            start: CssGridPlacement::Auto,
            end: CssGridPlacement::Auto,
        }
    }
}

impl CssGridLine {
    fn to_taffy(self) -> Line<GridPlacement> {
        Line {
            start: self.start.to_taffy(),
            end: self.end.to_taffy(),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Background {
    Default,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Background {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "default" => Some(Self::Default),
            "black" => Some(Self::Black),
            "red" => Some(Self::Red),
            "green" => Some(Self::Green),
            "yellow" => Some(Self::Yellow),
            "blue" => Some(Self::Blue),
            "magenta" => Some(Self::Magenta),
            "cyan" => Some(Self::Cyan),
            "white" => Some(Self::White),
            _ => None,
        }
    }

    pub(crate) fn ansi_bg(self) -> &'static str {
        match self {
            Self::Default => "\x1b[49m",
            Self::Black => "\x1b[40m",
            Self::Red => "\x1b[41m",
            Self::Green => "\x1b[42m",
            Self::Yellow => "\x1b[43m",
            Self::Blue => "\x1b[44m",
            Self::Magenta => "\x1b[45m",
            Self::Cyan => "\x1b[46m",
            Self::White => "\x1b[47m",
        }
    }
}

impl DivStyle {
    pub(crate) fn to_taffy(&self) -> Style {
        Style {
            display: match self.display {
                LayoutDisplay::Inline => Display::Block,
                LayoutDisplay::Block => Display::Block,
                LayoutDisplay::Flex => Display::Flex,
                LayoutDisplay::Grid => Display::Grid,
            },
            flex_direction: match self.flex_direction {
                LayoutFlexDirection::Row => FlexDirection::Row,
                LayoutFlexDirection::Column => FlexDirection::Column,
                LayoutFlexDirection::RowReverse => FlexDirection::RowReverse,
                LayoutFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
            },
            flex_wrap: match self.flex_wrap {
                LayoutFlexWrap::NoWrap => FlexWrap::NoWrap,
                LayoutFlexWrap::Wrap => FlexWrap::Wrap,
                LayoutFlexWrap::WrapReverse => FlexWrap::WrapReverse,
            },
            flex_basis: self.flex_basis.to_taffy(),
            flex_grow: self.flex_grow,
            flex_shrink: self.flex_shrink,
            justify_content: self.justify_content.map(|value| match value {
                LayoutJustifyContent::Start => JustifyContent::Start,
                LayoutJustifyContent::FlexStart => JustifyContent::FlexStart,
                LayoutJustifyContent::Center => JustifyContent::Center,
                LayoutJustifyContent::End => JustifyContent::End,
                LayoutJustifyContent::FlexEnd => JustifyContent::FlexEnd,
                LayoutJustifyContent::Stretch => JustifyContent::Stretch,
                LayoutJustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
                LayoutJustifyContent::SpaceAround => JustifyContent::SpaceAround,
                LayoutJustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
            }),
            align_items: self.align_items.map(layout_align_items_to_taffy),
            align_self: self.align_self.map(layout_align_items_to_taffy),
            align_content: self.align_content.map(|value| match value {
                LayoutJustifyContent::Start => AlignContent::Start,
                LayoutJustifyContent::FlexStart => AlignContent::FlexStart,
                LayoutJustifyContent::Center => AlignContent::Center,
                LayoutJustifyContent::End => AlignContent::End,
                LayoutJustifyContent::FlexEnd => AlignContent::FlexEnd,
                LayoutJustifyContent::Stretch => AlignContent::Stretch,
                LayoutJustifyContent::SpaceBetween => AlignContent::SpaceBetween,
                LayoutJustifyContent::SpaceAround => AlignContent::SpaceAround,
                LayoutJustifyContent::SpaceEvenly => AlignContent::SpaceEvenly,
            }),
            justify_items: self.justify_items.map(layout_align_items_to_taffy),
            justify_self: self.justify_self.map(layout_align_items_to_taffy),
            gap: Size {
                width: self.column_gap.to_taffy(),
                height: self.row_gap.to_taffy(),
            },
            size: Size {
                width: self.width.to_taffy(),
                height: self.height.to_taffy(),
            },
            grid_template_columns: self
                .grid_template_columns
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_template_rows: self
                .grid_template_rows
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_auto_columns: self
                .grid_auto_columns
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_auto_rows: self
                .grid_auto_rows
                .iter()
                .map(|track| track.to_taffy())
                .collect(),
            grid_auto_flow: self.grid_auto_flow.to_taffy(),
            grid_column: self.grid_column.to_taffy(),
            grid_row: self.grid_row.to_taffy(),
            overflow: Point {
                x: match self.overflow {
                    LayoutOverflow::Visible => Overflow::Visible,
                    LayoutOverflow::Hidden => Overflow::Hidden,
                },
                y: match self.overflow {
                    LayoutOverflow::Visible => Overflow::Visible,
                    LayoutOverflow::Hidden => Overflow::Hidden,
                },
            },
            ..Default::default()
        }
    }
}

fn layout_align_items_to_taffy(value: LayoutAlignItems) -> AlignItems {
    match value {
        LayoutAlignItems::Start => AlignItems::Start,
        LayoutAlignItems::FlexStart => AlignItems::FlexStart,
        LayoutAlignItems::Center => AlignItems::Center,
        LayoutAlignItems::End => AlignItems::End,
        LayoutAlignItems::FlexEnd => AlignItems::FlexEnd,
        LayoutAlignItems::Baseline => AlignItems::Baseline,
        LayoutAlignItems::Stretch => AlignItems::Stretch,
    }
}

pub(crate) fn parse_display(value: &str) -> Result<LayoutDisplay> {
    match value.trim() {
        "inline" => Ok(LayoutDisplay::Inline),
        "block" => Ok(LayoutDisplay::Block),
        "flex" | "flexbox" => Ok(LayoutDisplay::Flex),
        "grid" => Ok(LayoutDisplay::Grid),
        value => Err(Error::from_reason(format!("unsupported display: {value}"))),
    }
}

pub(crate) fn parse_overflow(value: &str) -> Result<LayoutOverflow> {
    match value.trim() {
        "visible" => Ok(LayoutOverflow::Visible),
        "hidden" => Ok(LayoutOverflow::Hidden),
        value => Err(Error::from_reason(format!("unsupported overflow: {value}"))),
    }
}

pub(crate) fn parse_flex_direction(value: &str) -> Result<LayoutFlexDirection> {
    match value.trim() {
        "row" => Ok(LayoutFlexDirection::Row),
        "column" => Ok(LayoutFlexDirection::Column),
        "row-reverse" => Ok(LayoutFlexDirection::RowReverse),
        "column-reverse" => Ok(LayoutFlexDirection::ColumnReverse),
        value => Err(Error::from_reason(format!(
            "unsupported flex direction: {value}"
        ))),
    }
}

pub(crate) fn parse_flex_wrap(value: &str) -> Result<LayoutFlexWrap> {
    match value.trim() {
        "nowrap" => Ok(LayoutFlexWrap::NoWrap),
        "wrap" => Ok(LayoutFlexWrap::Wrap),
        "wrap-reverse" => Ok(LayoutFlexWrap::WrapReverse),
        value => Err(Error::from_reason(format!(
            "unsupported flex-wrap: {value}"
        ))),
    }
}

pub(crate) fn parse_flex_flow(value: &str) -> Result<(LayoutFlexDirection, LayoutFlexWrap)> {
    let mut direction = None;
    let mut flex_wrap = None;

    for part in value.split_whitespace() {
        if direction.is_none() {
            if let Ok(parsed) = parse_flex_direction(part) {
                direction = Some(parsed);
                continue;
            }
        }

        if flex_wrap.is_none() {
            if let Ok(parsed) = parse_flex_wrap(part) {
                flex_wrap = Some(parsed);
                continue;
            }
        }

        return Err(Error::from_reason(format!(
            "unsupported flex-flow: {value}"
        )));
    }

    Ok((
        direction.unwrap_or(LayoutFlexDirection::Row),
        flex_wrap.unwrap_or(LayoutFlexWrap::NoWrap),
    ))
}

pub(crate) fn parse_flex_shorthand(id: u32, value: &str) -> Result<RenderCommand> {
    let value = value.trim();
    if value == "none" {
        return Ok(RenderCommand::SetFlex {
            id,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            flex_basis: CssDimension::Auto,
        });
    }
    if value == "auto" {
        return Ok(RenderCommand::SetFlex {
            id,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Auto,
        });
    }
    if value == "initial" {
        return Ok(RenderCommand::SetFlex {
            id,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Auto,
        });
    }

    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [grow] => Ok(RenderCommand::SetFlex {
            id,
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Length(0.0),
        }),
        [grow, shrink] => Ok(RenderCommand::SetFlex {
            id,
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: parse_non_negative_number("flex-shrink", shrink)?,
            flex_basis: CssDimension::Length(0.0),
        }),
        [grow, shrink, basis] => Ok(RenderCommand::SetFlex {
            id,
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: parse_non_negative_number("flex-shrink", shrink)?,
            flex_basis: parse_dimension(basis)?,
        }),
        _ => Err(Error::from_reason(format!(
            "unsupported flex shorthand: {value}"
        ))),
    }
}

pub(crate) fn parse_justify_content(value: &str) -> Result<LayoutJustifyContent> {
    match value.trim() {
        "start" => Ok(LayoutJustifyContent::Start),
        "flex-start" => Ok(LayoutJustifyContent::FlexStart),
        "center" => Ok(LayoutJustifyContent::Center),
        "end" => Ok(LayoutJustifyContent::End),
        "flex-end" => Ok(LayoutJustifyContent::FlexEnd),
        "stretch" => Ok(LayoutJustifyContent::Stretch),
        "space-between" => Ok(LayoutJustifyContent::SpaceBetween),
        "space-around" => Ok(LayoutJustifyContent::SpaceAround),
        "space-evenly" => Ok(LayoutJustifyContent::SpaceEvenly),
        value => Err(Error::from_reason(format!(
            "unsupported justify-content: {value}"
        ))),
    }
}

pub(crate) fn parse_align_items(value: &str) -> Result<LayoutAlignItems> {
    match value.trim() {
        "start" => Ok(LayoutAlignItems::Start),
        "flex-start" => Ok(LayoutAlignItems::FlexStart),
        "center" => Ok(LayoutAlignItems::Center),
        "end" => Ok(LayoutAlignItems::End),
        "flex-end" => Ok(LayoutAlignItems::FlexEnd),
        "baseline" => Ok(LayoutAlignItems::Baseline),
        "stretch" => Ok(LayoutAlignItems::Stretch),
        value => Err(Error::from_reason(format!(
            "unsupported align-items: {value}"
        ))),
    }
}

pub(crate) fn parse_non_negative_number(property: &str, value: &str) -> Result<f32> {
    let parsed = value
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid {property}: {value}")))?;

    if parsed.is_sign_negative() {
        Err(Error::from_reason(format!(
            "{property} must be non-negative: {value}"
        )))
    } else {
        Ok(parsed)
    }
}

pub(crate) fn parse_gap(value: &str) -> Result<(CssLengthPercentage, CssLengthPercentage)> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [both] => {
            let parsed = parse_length_percentage(both)?;
            Ok((parsed, parsed))
        }
        [row, column] => Ok((
            parse_length_percentage(row)?,
            parse_length_percentage(column)?,
        )),
        _ => Err(Error::from_reason(format!("invalid gap: {value}"))),
    }
}

pub(crate) fn parse_length_percentage(value: &str) -> Result<CssLengthPercentage> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid percentage: {value}")))?;
        return Ok(CssLengthPercentage::Percent(percent / 100.0));
    }

    let number = value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid length: {value}")))?;
    Ok(CssLengthPercentage::Length(number))
}

pub(crate) fn parse_dimension(value: &str) -> Result<CssDimension> {
    let value = value.trim();

    if value == "auto" || value.is_empty() {
        return Ok(CssDimension::Auto);
    }

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid percentage dimension: {value}")))?;
        return Ok(CssDimension::Percent(percent / 100.0));
    }

    let number = value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid dimension: {value}")))?;

    Ok(CssDimension::Length(number))
}

pub(crate) fn parse_grid_template_tracks(value: &str) -> Result<Vec<CssGridTemplateTrack>> {
    parse_grid_auto_tracks(value).map(|tracks| {
        tracks
            .into_iter()
            .map(CssGridTemplateTrack::Single)
            .collect()
    })
}

pub(crate) fn parse_grid_auto_tracks(value: &str) -> Result<Vec<CssTrackSizing>> {
    let tracks = value
        .split_whitespace()
        .map(parse_track_sizing)
        .collect::<Result<Vec<_>>>()?;

    if tracks.is_empty() {
        Err(Error::from_reason("grid track list cannot be empty"))
    } else {
        Ok(tracks)
    }
}

fn parse_track_sizing(value: &str) -> Result<CssTrackSizing> {
    let value = value.trim();
    match value {
        "auto" => return Ok(CssTrackSizing::Auto),
        "min-content" => return Ok(CssTrackSizing::MinContent),
        "max-content" => return Ok(CssTrackSizing::MaxContent),
        _ => {}
    }

    if let Some(fr) = value.strip_suffix("fr") {
        let fr = fr
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid fr track size: {value}")))?;
        return Ok(CssTrackSizing::Fr(fr));
    }

    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent
            .trim()
            .parse::<f32>()
            .map_err(|_| Error::from_reason(format!("invalid percentage track size: {value}")))?;
        return Ok(CssTrackSizing::Percent(percent / 100.0));
    }

    let number = value
        .strip_suffix("px")
        .unwrap_or(value)
        .trim()
        .parse::<f32>()
        .map_err(|_| Error::from_reason(format!("invalid track size: {value}")))?;
    Ok(CssTrackSizing::Length(number))
}

pub(crate) fn parse_grid_auto_flow(value: &str) -> Result<LayoutGridAutoFlow> {
    let mut axis = None;
    let mut dense = false;

    for part in value.split_whitespace() {
        match part {
            "row" => axis = Some("row"),
            "column" => axis = Some("column"),
            "dense" => dense = true,
            _ => {
                return Err(Error::from_reason(format!(
                    "unsupported grid-auto-flow: {value}"
                )))
            }
        }
    }

    match (axis, dense) {
        (Some("row") | None, false) => Ok(LayoutGridAutoFlow::Row),
        (Some("column"), false) => Ok(LayoutGridAutoFlow::Column),
        (Some("row") | None, true) => Ok(LayoutGridAutoFlow::RowDense),
        (Some("column"), true) => Ok(LayoutGridAutoFlow::ColumnDense),
        _ => Err(Error::from_reason(format!(
            "unsupported grid-auto-flow: {value}"
        ))),
    }
}

pub(crate) fn parse_grid_line(value: &str) -> Result<CssGridLine> {
    let parts = value.split('/').map(str::trim).collect::<Vec<_>>();
    match parts.as_slice() {
        [single] => Ok(CssGridLine {
            start: parse_grid_placement(single)?,
            end: CssGridPlacement::Auto,
        }),
        [start, end] => Ok(CssGridLine {
            start: parse_grid_placement(start)?,
            end: parse_grid_placement(end)?,
        }),
        _ => Err(Error::from_reason(format!(
            "invalid grid placement: {value}"
        ))),
    }
}

pub(crate) fn parse_grid_placement(value: &str) -> Result<CssGridPlacement> {
    let value = value.trim();
    if value == "auto" {
        return Ok(CssGridPlacement::Auto);
    }

    if let Some(span) = value.strip_prefix("span ") {
        let span = span
            .trim()
            .parse::<u16>()
            .map_err(|_| Error::from_reason(format!("invalid grid span: {value}")))?;
        return Ok(CssGridPlacement::Span(span));
    }

    let line = value
        .parse::<i16>()
        .map_err(|_| Error::from_reason(format!("invalid grid line: {value}")))?;
    Ok(CssGridPlacement::Line(line))
}
