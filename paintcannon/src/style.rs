use csscolorparser::NAMED_COLORS;
use napi::{Error, Result};
use taffy::geometry::Point;
use taffy::prelude::*;
use taffy::style::Overflow;
use termprofile::{
    anstyle::{Color, RgbColor},
    TermProfile,
};

#[derive(Clone)]
pub(crate) struct DivStyle {
    pub(crate) display: LayoutDisplay,
    pub(crate) visibility: CssVisibility,
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
    pub(crate) padding_top: CssLengthPercentage,
    pub(crate) padding_right: CssLengthPercentage,
    pub(crate) padding_bottom: CssLengthPercentage,
    pub(crate) padding_left: CssLengthPercentage,
    pub(crate) margin_top: CssLengthPercentageAuto,
    pub(crate) margin_right: CssLengthPercentageAuto,
    pub(crate) margin_bottom: CssLengthPercentageAuto,
    pub(crate) margin_left: CssLengthPercentageAuto,
    pub(crate) width: CssDimension,
    pub(crate) height: CssDimension,
    pub(crate) min_width: CssDimension,
    pub(crate) max_width: CssDimension,
    pub(crate) min_height: CssDimension,
    pub(crate) max_height: CssDimension,
    pub(crate) grid_template_columns: Vec<CssGridTemplateTrack>,
    pub(crate) grid_template_rows: Vec<CssGridTemplateTrack>,
    pub(crate) grid_auto_columns: Vec<CssTrackSizing>,
    pub(crate) grid_auto_rows: Vec<CssTrackSizing>,
    pub(crate) grid_auto_flow: LayoutGridAutoFlow,
    pub(crate) grid_column: CssGridLine,
    pub(crate) grid_row: CssGridLine,
    pub(crate) color: Background,
    pub(crate) placeholder_color: Background,
    pub(crate) background: Background,
    pub(crate) selection_background: Option<Background>,
    pub(crate) font_weight: CssFontWeight,
    pub(crate) font_style: CssFontStyle,
    pub(crate) text_decoration_line: CssTextDecorationLine,
    pub(crate) border_top: BorderStyle,
    pub(crate) border_right: BorderStyle,
    pub(crate) border_bottom: BorderStyle,
    pub(crate) border_left: BorderStyle,
    pub(crate) border_color: Background,
    pub(crate) cursor: CursorStyle,
    pub(crate) overflow_x: LayoutOverflow,
    pub(crate) overflow_y: LayoutOverflow,
    pub(crate) scrollbar_color: ScrollbarColor,
    pub(crate) scrollbar_gutter: ScrollbarGutter,
    pub(crate) image_rendering: ImageRendering,
    pub(crate) white_space: CssWhiteSpace,
}

impl Default for DivStyle {
    fn default() -> Self {
        Self {
            display: LayoutDisplay::Block,
            visibility: CssVisibility::Inherit,
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
            padding_top: CssLengthPercentage::Length(0.0),
            padding_right: CssLengthPercentage::Length(0.0),
            padding_bottom: CssLengthPercentage::Length(0.0),
            padding_left: CssLengthPercentage::Length(0.0),
            margin_top: CssLengthPercentageAuto::Length(0.0),
            margin_right: CssLengthPercentageAuto::Length(0.0),
            margin_bottom: CssLengthPercentageAuto::Length(0.0),
            margin_left: CssLengthPercentageAuto::Length(0.0),
            width: CssDimension::Auto,
            height: CssDimension::Auto,
            min_width: CssDimension::Length(0.0),
            max_width: CssDimension::Auto,
            min_height: CssDimension::Auto,
            max_height: CssDimension::Auto,
            grid_template_columns: Vec::new(),
            grid_template_rows: Vec::new(),
            grid_auto_columns: Vec::new(),
            grid_auto_rows: Vec::new(),
            grid_auto_flow: LayoutGridAutoFlow::Row,
            grid_column: CssGridLine::default(),
            grid_row: CssGridLine::default(),
            color: Background::Default,
            placeholder_color: Background::Rgb(100, 116, 139),
            background: Background::Default,
            selection_background: None,
            font_weight: CssFontWeight::Inherit,
            font_style: CssFontStyle::Inherit,
            text_decoration_line: CssTextDecorationLine::Inherit,
            border_top: BorderStyle::None,
            border_right: BorderStyle::None,
            border_bottom: BorderStyle::None,
            border_left: BorderStyle::None,
            border_color: Background::Default,
            cursor: CursorStyle::Auto,
            overflow_x: LayoutOverflow::Visible,
            overflow_y: LayoutOverflow::Visible,
            scrollbar_color: ScrollbarColor::Auto,
            scrollbar_gutter: ScrollbarGutter::Auto,
            image_rendering: ImageRendering::HalfBlock,
            white_space: CssWhiteSpace::Normal,
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
pub(crate) enum CssVisibility {
    Inherit,
    Visible,
    Hidden,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutOverflow {
    Visible,
    Hidden,
    Scroll,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollbarGutter {
    Auto,
    Stable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScrollbarColor {
    Auto,
    Colors {
        thumb: Background,
        track: Background,
    },
}

impl ScrollbarColor {
    pub(crate) fn resolve(self) -> (Background, Background) {
        match self {
            Self::Auto => (Background::White, Background::Default),
            Self::Colors { thumb, track } => (thumb, track),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ImageRendering {
    Ascii,
    HalfBlock,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CssWhiteSpace {
    Normal,
    NoWrap,
    Pre,
    PreWrap,
    PreLine,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CssFontWeight {
    Inherit,
    Normal,
    Bold,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CssFontStyle {
    Inherit,
    Normal,
    Italic,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CssTextDecorationLine {
    Inherit,
    None,
    Underline,
    LineThrough,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum BorderStyle {
    None,
    Solid,
    Double,
    Heavy,
    Rounded,
    ChunkyRounded,
    Ascii,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum CursorStyle {
    Auto,
    Alias,
    Cell,
    Copy,
    Crosshair,
    Default,
    EResize,
    EwResize,
    Grab,
    Grabbing,
    Help,
    Move,
    NResize,
    NeResize,
    NeswResize,
    NoDrop,
    NotAllowed,
    NsResize,
    NwResize,
    NwseResize,
    Pointer,
    Progress,
    SResize,
    SeResize,
    SwResize,
    Text,
    VerticalText,
    WResize,
    Wait,
    ZoomIn,
    ZoomOut,
}

#[derive(Clone, Copy)]
pub(crate) struct FlexShorthand {
    pub(crate) flex_grow: f32,
    pub(crate) flex_shrink: f32,
    pub(crate) flex_basis: CssDimension,
}

impl CursorStyle {
    pub(crate) fn osc_shape(self) -> Option<&'static str> {
        match self {
            Self::Auto => None,
            Self::Alias => Some("alias"),
            Self::Cell => Some("cell"),
            Self::Copy => Some("copy"),
            Self::Crosshair => Some("crosshair"),
            Self::Default => Some("default"),
            Self::EResize => Some("e-resize"),
            Self::EwResize => Some("ew-resize"),
            Self::Grab => Some("grab"),
            Self::Grabbing => Some("grabbing"),
            Self::Help => Some("help"),
            Self::Move => Some("move"),
            Self::NResize => Some("n-resize"),
            Self::NeResize => Some("ne-resize"),
            Self::NeswResize => Some("nesw-resize"),
            Self::NoDrop => Some("no-drop"),
            Self::NotAllowed => Some("not-allowed"),
            Self::NsResize => Some("ns-resize"),
            Self::NwResize => Some("nw-resize"),
            Self::NwseResize => Some("nwse-resize"),
            Self::Pointer => Some("pointer"),
            Self::Progress => Some("progress"),
            Self::SResize => Some("s-resize"),
            Self::SeResize => Some("se-resize"),
            Self::SwResize => Some("sw-resize"),
            Self::Text => Some("text"),
            Self::VerticalText => Some("vertical-text"),
            Self::WResize => Some("w-resize"),
            Self::Wait => Some("wait"),
            Self::ZoomIn => Some("zoom-in"),
            Self::ZoomOut => Some("zoom-out"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum ColorTransitionProperty {
    Color,
    BackgroundColor,
    BorderColor,
}

#[derive(Clone, Copy)]
pub(crate) struct TransitionSpec {
    pub(crate) property: ColorTransitionProperty,
    pub(crate) duration_ms: u64,
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
pub(crate) enum CssLengthPercentageAuto {
    Auto,
    Length(f32),
    Percent(f32),
}

impl CssLengthPercentageAuto {
    fn to_taffy(self) -> LengthPercentageAuto {
        match self {
            Self::Auto => LengthPercentageAuto::AUTO,
            Self::Length(value) => LengthPercentageAuto::length(value),
            Self::Percent(value) => LengthPercentageAuto::percent(value),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
    Rgb(u8, u8, u8),
}

impl Background {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        let value = value.trim();
        if let Some(color) = parse_hex_color(value) {
            return Some(color);
        }

        if value == "default" {
            return Some(Self::Default);
        }

        let [red, green, blue] = *NAMED_COLORS.get(value.into())?;
        Some(match (red, green, blue) {
            (0, 0, 0) => Self::Black,
            (255, 0, 0) => Self::Red,
            (0, 128, 0) => Self::Green,
            (255, 255, 0) => Self::Yellow,
            (0, 0, 255) => Self::Blue,
            (255, 0, 255) => Self::Magenta,
            (0, 255, 255) => Self::Cyan,
            (255, 255, 255) => Self::White,
            _ => Self::Rgb(red, green, blue),
        })
    }

    pub(crate) fn rgb(self) -> Option<(u8, u8, u8)> {
        match self {
            Self::Default => None,
            Self::Black => Some((0, 0, 0)),
            Self::Red => Some((255, 0, 0)),
            Self::Green => Some((0, 128, 0)),
            Self::Yellow => Some((255, 255, 0)),
            Self::Blue => Some((0, 0, 255)),
            Self::Magenta => Some((255, 0, 255)),
            Self::Cyan => Some((0, 255, 255)),
            Self::White => Some((255, 255, 255)),
            Self::Rgb(red, green, blue) => Some((red, green, blue)),
        }
    }

    pub(crate) fn ansi_bg(self, profile: TermProfile) -> String {
        ansi_color(self, profile, ColorPlane::Background)
    }

    pub(crate) fn ansi_fg(self, profile: TermProfile) -> String {
        ansi_color(self, profile, ColorPlane::Foreground)
    }
}

#[derive(Clone, Copy)]
enum ColorPlane {
    Foreground,
    Background,
}

fn ansi_color(color: Background, profile: TermProfile, plane: ColorPlane) -> String {
    if matches!(profile, TermProfile::NoTty | TermProfile::NoColor) {
        return String::new();
    }

    if color == Background::Default {
        return match plane {
            ColorPlane::Foreground => "\x1b[39m".to_string(),
            ColorPlane::Background => "\x1b[49m".to_string(),
        };
    }

    let Some((red, green, blue)) = color.rgb() else {
        return String::new();
    };

    let color = Color::Rgb(RgbColor(red, green, blue));
    let Some(color) = profile.adapt_color(color) else {
        return String::new();
    };

    match plane {
        ColorPlane::Foreground => color.render_fg().to_string(),
        ColorPlane::Background => color.render_bg().to_string(),
    }
}

fn parse_hex_color(value: &str) -> Option<Background> {
    let hex = value.strip_prefix('#')?;
    if hex.len() == 3 {
        let red = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
        let green = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
        let blue = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
        return Some(Background::Rgb(red, green, blue));
    }

    if hex.len() == 6 {
        let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
        let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
        let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
        return Some(Background::Rgb(red, green, blue));
    }

    None
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
            min_size: Size {
                width: self.min_width.to_taffy(),
                height: self.min_height.to_taffy(),
            },
            max_size: Size {
                width: self.max_width.to_taffy(),
                height: self.max_height.to_taffy(),
            },
            border: Rect {
                left: border_size(self.border_left),
                right: border_size(self.border_right),
                top: border_size(self.border_top),
                bottom: border_size(self.border_bottom),
            },
            padding: Rect {
                left: self.padding_left.to_taffy(),
                right: self.padding_right.to_taffy(),
                top: self.padding_top.to_taffy(),
                bottom: self.padding_bottom.to_taffy(),
            },
            margin: Rect {
                left: self.margin_left.to_taffy(),
                right: self.margin_right.to_taffy(),
                top: self.margin_top.to_taffy(),
                bottom: self.margin_bottom.to_taffy(),
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
                x: self.taffy_overflow_for_axis(self.overflow_x),
                y: self.taffy_overflow_for_axis(self.overflow_y),
            },
            scrollbar_width: if self.reserves_scrollbar() { 1.0 } else { 0.0 },
            ..Default::default()
        }
    }

    fn reserves_scrollbar(&self) -> bool {
        self.reserves_scrollbar_for_axis(self.overflow_x)
            || self.reserves_scrollbar_for_axis(self.overflow_y)
    }

    fn reserves_scrollbar_for_axis(&self, overflow: LayoutOverflow) -> bool {
        overflow == LayoutOverflow::Scroll
            || (self.scrollbar_gutter == ScrollbarGutter::Stable
                && overflow == LayoutOverflow::Hidden)
    }

    fn taffy_overflow_for_axis(&self, overflow: LayoutOverflow) -> Overflow {
        match overflow {
            LayoutOverflow::Visible => Overflow::Visible,
            LayoutOverflow::Hidden if self.reserves_scrollbar_for_axis(overflow) => {
                Overflow::Scroll
            }
            LayoutOverflow::Hidden => Overflow::Hidden,
            LayoutOverflow::Scroll => Overflow::Scroll,
        }
    }
}

fn border_size(style: BorderStyle) -> LengthPercentage {
    LengthPercentage::length(if style == BorderStyle::None { 0.0 } else { 1.0 })
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

pub(crate) fn parse_visibility(value: &str) -> Result<CssVisibility> {
    match value.trim() {
        "" | "inherit" => Ok(CssVisibility::Inherit),
        "visible" => Ok(CssVisibility::Visible),
        "hidden" => Ok(CssVisibility::Hidden),
        value => Err(Error::from_reason(format!(
            "unsupported visibility: {value}"
        ))),
    }
}

pub(crate) fn parse_overflow(value: &str) -> Result<LayoutOverflow> {
    match value.trim() {
        "visible" => Ok(LayoutOverflow::Visible),
        "hidden" => Ok(LayoutOverflow::Hidden),
        "scroll" => Ok(LayoutOverflow::Scroll),
        value => Err(Error::from_reason(format!("unsupported overflow: {value}"))),
    }
}

pub(crate) fn parse_scrollbar_color(value: &str) -> Result<ScrollbarColor> {
    let value = value.trim();
    if value == "auto" {
        return Ok(ScrollbarColor::Auto);
    }

    let parts = value.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 2 {
        return Err(Error::from_reason(format!(
            "unsupported scrollbar-color: {value}"
        )));
    }

    let thumb = Background::parse(parts[0]).ok_or_else(|| {
        Error::from_reason(format!("unsupported scrollbar thumb color: {}", parts[0]))
    })?;
    let track = Background::parse(parts[1]).ok_or_else(|| {
        Error::from_reason(format!("unsupported scrollbar track color: {}", parts[1]))
    })?;
    Ok(ScrollbarColor::Colors { thumb, track })
}

pub(crate) fn parse_scrollbar_gutter(value: &str) -> Result<ScrollbarGutter> {
    match value.trim() {
        "auto" => Ok(ScrollbarGutter::Auto),
        "stable" => Ok(ScrollbarGutter::Stable),
        value => Err(Error::from_reason(format!(
            "unsupported scrollbar-gutter: {value}"
        ))),
    }
}

pub(crate) fn parse_image_rendering(value: &str) -> Result<ImageRendering> {
    match value.trim() {
        "ascii" => Ok(ImageRendering::Ascii),
        "half-block" => Ok(ImageRendering::HalfBlock),
        value => Err(Error::from_reason(format!(
            "unsupported image rendering: {value}"
        ))),
    }
}

pub(crate) fn parse_border_style(value: &str) -> Result<BorderStyle> {
    match value.trim() {
        "none" => Ok(BorderStyle::None),
        "solid" => Ok(BorderStyle::Solid),
        "double" => Ok(BorderStyle::Double),
        "heavy" => Ok(BorderStyle::Heavy),
        "rounded" => Ok(BorderStyle::Rounded),
        "chunky-rounded" => Ok(BorderStyle::ChunkyRounded),
        "ascii" => Ok(BorderStyle::Ascii),
        value => Err(Error::from_reason(format!(
            "unsupported border style: {value}"
        ))),
    }
}

pub(crate) fn parse_cursor(value: &str) -> Result<CursorStyle> {
    match value.trim() {
        "auto" => Ok(CursorStyle::Auto),
        "alias" => Ok(CursorStyle::Alias),
        "cell" => Ok(CursorStyle::Cell),
        "copy" => Ok(CursorStyle::Copy),
        "crosshair" => Ok(CursorStyle::Crosshair),
        "default" => Ok(CursorStyle::Default),
        "e-resize" => Ok(CursorStyle::EResize),
        "ew-resize" => Ok(CursorStyle::EwResize),
        "grab" => Ok(CursorStyle::Grab),
        "grabbing" => Ok(CursorStyle::Grabbing),
        "help" => Ok(CursorStyle::Help),
        "move" => Ok(CursorStyle::Move),
        "n-resize" => Ok(CursorStyle::NResize),
        "ne-resize" => Ok(CursorStyle::NeResize),
        "nesw-resize" => Ok(CursorStyle::NeswResize),
        "no-drop" => Ok(CursorStyle::NoDrop),
        "not-allowed" => Ok(CursorStyle::NotAllowed),
        "ns-resize" => Ok(CursorStyle::NsResize),
        "nw-resize" => Ok(CursorStyle::NwResize),
        "nwse-resize" => Ok(CursorStyle::NwseResize),
        "pointer" => Ok(CursorStyle::Pointer),
        "progress" => Ok(CursorStyle::Progress),
        "s-resize" => Ok(CursorStyle::SResize),
        "se-resize" => Ok(CursorStyle::SeResize),
        "sw-resize" => Ok(CursorStyle::SwResize),
        "text" => Ok(CursorStyle::Text),
        "vertical-text" => Ok(CursorStyle::VerticalText),
        "w-resize" => Ok(CursorStyle::WResize),
        "wait" => Ok(CursorStyle::Wait),
        "zoom-in" => Ok(CursorStyle::ZoomIn),
        "zoom-out" => Ok(CursorStyle::ZoomOut),
        value => Err(Error::from_reason(format!("unsupported cursor: {value}"))),
    }
}

pub(crate) fn parse_transition(value: &str) -> Vec<TransitionSpec> {
    let mut transitions = Vec::new();
    for part in value.split(',') {
        let tokens = part.split_whitespace().collect::<Vec<_>>();
        if tokens.len() < 2 {
            continue;
        }

        let Some(duration_ms) = tokens.iter().find_map(|token| parse_duration_ms(token)) else {
            continue;
        };

        match tokens[0] {
            "all" => {
                transitions.push(TransitionSpec {
                    property: ColorTransitionProperty::Color,
                    duration_ms,
                });
                transitions.push(TransitionSpec {
                    property: ColorTransitionProperty::BackgroundColor,
                    duration_ms,
                });
                transitions.push(TransitionSpec {
                    property: ColorTransitionProperty::BorderColor,
                    duration_ms,
                });
            }
            "color" => transitions.push(TransitionSpec {
                property: ColorTransitionProperty::Color,
                duration_ms,
            }),
            "background" | "background-color" | "backgroundColor" => {
                transitions.push(TransitionSpec {
                    property: ColorTransitionProperty::BackgroundColor,
                    duration_ms,
                });
            }
            "border-color" | "borderColor" => transitions.push(TransitionSpec {
                property: ColorTransitionProperty::BorderColor,
                duration_ms,
            }),
            _ => {}
        }
    }

    transitions
}

fn parse_duration_ms(value: &str) -> Option<u64> {
    if let Some(ms) = value.strip_suffix("ms") {
        let duration = ms.parse::<f32>().ok()?;
        return (duration >= 0.0).then_some(duration.round() as u64);
    }

    if let Some(seconds) = value.strip_suffix('s') {
        let duration = seconds.parse::<f32>().ok()?;
        return (duration >= 0.0).then_some((duration * 1000.0).round() as u64);
    }

    None
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

pub(crate) fn parse_flex_shorthand(value: &str) -> Result<FlexShorthand> {
    let value = value.trim();
    if value == "none" {
        return Ok(FlexShorthand {
            flex_grow: 0.0,
            flex_shrink: 0.0,
            flex_basis: CssDimension::Auto,
        });
    }
    if value == "auto" {
        return Ok(FlexShorthand {
            flex_grow: 1.0,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Auto,
        });
    }
    if value == "initial" {
        return Ok(FlexShorthand {
            flex_grow: 0.0,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Auto,
        });
    }

    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [grow] => Ok(FlexShorthand {
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: 1.0,
            flex_basis: CssDimension::Length(0.0),
        }),
        [grow, shrink] => Ok(FlexShorthand {
            flex_grow: parse_non_negative_number("flex-grow", grow)?,
            flex_shrink: parse_non_negative_number("flex-shrink", shrink)?,
            flex_basis: CssDimension::Length(0.0),
        }),
        [grow, shrink, basis] => Ok(FlexShorthand {
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

pub(crate) fn parse_box_lengths(
    property: &str,
    value: &str,
) -> Result<(
    CssLengthPercentage,
    CssLengthPercentage,
    CssLengthPercentage,
    CssLengthPercentage,
)> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [all] => {
            let all = parse_length_percentage(all)?;
            Ok((all, all, all, all))
        }
        [vertical, horizontal] => {
            let vertical = parse_length_percentage(vertical)?;
            let horizontal = parse_length_percentage(horizontal)?;
            Ok((vertical, horizontal, vertical, horizontal))
        }
        [top, horizontal, bottom] => {
            let top = parse_length_percentage(top)?;
            let horizontal = parse_length_percentage(horizontal)?;
            let bottom = parse_length_percentage(bottom)?;
            Ok((top, horizontal, bottom, horizontal))
        }
        [top, right, bottom, left] => Ok((
            parse_length_percentage(top)?,
            parse_length_percentage(right)?,
            parse_length_percentage(bottom)?,
            parse_length_percentage(left)?,
        )),
        _ => Err(Error::from_reason(format!("invalid {property}: {value}"))),
    }
}

pub(crate) fn parse_margin_lengths(
    value: &str,
) -> Result<(
    CssLengthPercentageAuto,
    CssLengthPercentageAuto,
    CssLengthPercentageAuto,
    CssLengthPercentageAuto,
)> {
    let parts = value.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [all] => {
            let all = parse_length_percentage_auto(all)?;
            Ok((all, all, all, all))
        }
        [vertical, horizontal] => {
            let vertical = parse_length_percentage_auto(vertical)?;
            let horizontal = parse_length_percentage_auto(horizontal)?;
            Ok((vertical, horizontal, vertical, horizontal))
        }
        [top, horizontal, bottom] => {
            let top = parse_length_percentage_auto(top)?;
            let horizontal = parse_length_percentage_auto(horizontal)?;
            let bottom = parse_length_percentage_auto(bottom)?;
            Ok((top, horizontal, bottom, horizontal))
        }
        [top, right, bottom, left] => Ok((
            parse_length_percentage_auto(top)?,
            parse_length_percentage_auto(right)?,
            parse_length_percentage_auto(bottom)?,
            parse_length_percentage_auto(left)?,
        )),
        _ => Err(Error::from_reason(format!("invalid margin: {value}"))),
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

pub(crate) fn parse_length_percentage_auto(value: &str) -> Result<CssLengthPercentageAuto> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("auto") {
        return Ok(CssLengthPercentageAuto::Auto);
    }

    match parse_length_percentage(value)? {
        CssLengthPercentage::Length(value) => Ok(CssLengthPercentageAuto::Length(value)),
        CssLengthPercentage::Percent(value) => Ok(CssLengthPercentageAuto::Percent(value)),
    }
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

pub(crate) fn parse_white_space(value: &str) -> Result<CssWhiteSpace> {
    match value.trim() {
        "" | "normal" => Ok(CssWhiteSpace::Normal),
        "nowrap" => Ok(CssWhiteSpace::NoWrap),
        "pre" => Ok(CssWhiteSpace::Pre),
        "pre-wrap" | "preWrap" => Ok(CssWhiteSpace::PreWrap),
        "pre-line" | "preLine" => Ok(CssWhiteSpace::PreLine),
        value => Err(Error::from_reason(format!(
            "unsupported white-space: {value}"
        ))),
    }
}

pub(crate) fn parse_font_weight(value: &str) -> Result<CssFontWeight> {
    match value.trim() {
        "" | "inherit" => Ok(CssFontWeight::Inherit),
        "normal" => Ok(CssFontWeight::Normal),
        "bold" => Ok(CssFontWeight::Bold),
        value => Err(Error::from_reason(format!(
            "unsupported font-weight: {value}"
        ))),
    }
}

pub(crate) fn parse_font_style(value: &str) -> Result<CssFontStyle> {
    match value.trim() {
        "" | "inherit" => Ok(CssFontStyle::Inherit),
        "normal" => Ok(CssFontStyle::Normal),
        "italic" => Ok(CssFontStyle::Italic),
        value => Err(Error::from_reason(format!(
            "unsupported font-style: {value}"
        ))),
    }
}

pub(crate) fn parse_text_decoration_line(value: &str) -> Result<CssTextDecorationLine> {
    match value.trim() {
        "" | "inherit" => Ok(CssTextDecorationLine::Inherit),
        "none" => Ok(CssTextDecorationLine::None),
        "underline" => Ok(CssTextDecorationLine::Underline),
        "line-through" => Ok(CssTextDecorationLine::LineThrough),
        value => Err(Error::from_reason(format!(
            "unsupported text-decoration: {value}"
        ))),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_accept_css_named_colors_with_browser_rgb_values() {
        assert_eq!(
            Background::parse("gray").unwrap().rgb(),
            Some((128, 128, 128))
        );
        assert_eq!(Background::parse("green").unwrap().rgb(), Some((0, 128, 0)));
        assert_eq!(Background::parse("lime").unwrap().rgb(), Some((0, 255, 0)));
        assert_eq!(
            Background::parse("rebeccapurple").unwrap().rgb(),
            Some((102, 51, 153))
        );
        assert_eq!(
            Background::parse("LightGoldenRodYellow").unwrap().rgb(),
            Some((250, 250, 210))
        );
        assert_eq!(Background::parse("not-a-color"), None);
        assert_eq!(Background::parse("transparent"), None);
    }

    #[test]
    fn font_weight_only_accepts_terminal_backed_values() {
        assert!(matches!(
            parse_font_weight("bold").unwrap(),
            CssFontWeight::Bold
        ));
        assert!(matches!(
            parse_font_weight("normal").unwrap(),
            CssFontWeight::Normal
        ));
        assert!(parse_font_weight("700").is_err());
    }

    #[test]
    fn font_style_only_accepts_terminal_backed_values() {
        assert!(matches!(
            parse_font_style("italic").unwrap(),
            CssFontStyle::Italic
        ));
        assert!(matches!(
            parse_font_style("normal").unwrap(),
            CssFontStyle::Normal
        ));
        assert!(parse_font_style("oblique").is_err());
    }

    #[test]
    fn text_decoration_only_accepts_terminal_backed_values() {
        assert!(matches!(
            parse_text_decoration_line("underline").unwrap(),
            CssTextDecorationLine::Underline
        ));
        assert!(matches!(
            parse_text_decoration_line("none").unwrap(),
            CssTextDecorationLine::None
        ));
        assert!(matches!(
            parse_text_decoration_line("line-through").unwrap(),
            CssTextDecorationLine::LineThrough
        ));
        assert!(parse_text_decoration_line("underline dotted").is_err());
    }

    #[test]
    fn scrollbar_color_accepts_auto_or_two_terminal_colors() {
        assert!(matches!(
            parse_scrollbar_color("auto").unwrap(),
            ScrollbarColor::Auto
        ));
        assert!(matches!(
            parse_scrollbar_color("#38bdf8 #334155").unwrap(),
            ScrollbarColor::Colors {
                thumb: Background::Rgb(56, 189, 248),
                track: Background::Rgb(51, 65, 85)
            }
        ));
        assert!(parse_scrollbar_color("#38bdf8").is_err());
        assert!(matches!(
            parse_scrollbar_color("tomato darkslategray").unwrap(),
            ScrollbarColor::Colors {
                thumb: Background::Rgb(255, 99, 71),
                track: Background::Rgb(47, 79, 79)
            }
        ));
    }

    #[test]
    fn scrollbar_gutter_accepts_supported_values() {
        assert!(matches!(
            parse_scrollbar_gutter("auto").unwrap(),
            ScrollbarGutter::Auto
        ));
        assert!(matches!(
            parse_scrollbar_gutter("stable").unwrap(),
            ScrollbarGutter::Stable
        ));
        assert!(parse_scrollbar_gutter("stable both-edges").is_err());
    }
}
