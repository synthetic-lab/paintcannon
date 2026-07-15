use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use taffy::NodeId;

use crate::style::{Background, TransitionProperty, TransitionSpec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TransitionEvent {
    pub(crate) event_type: TransitionEventType,
    pub(crate) target: NodeId,
    pub(crate) property: TransitionProperty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TransitionEventType {
    Start,
    End,
}

#[derive(Default)]
pub(crate) struct TransitionState {
    specs: HashMap<NodeId, HashMap<TransitionProperty, Duration>>,
    active: HashMap<TransitionKey, ActiveTransition>,
    events: VecDeque<TransitionEvent>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TransitionKey {
    node: NodeId,
    property: TransitionProperty,
}

#[derive(Clone, Copy, Debug)]
enum ActiveTransition {
    Color(ActiveColorTransition),
    Opacity(ActiveOpacityTransition),
}

#[derive(Clone, Copy, Debug)]
struct ActiveColorTransition {
    from: Background,
    to: Background,
    started_at: Instant,
    duration: Duration,
}

#[derive(Clone, Copy, Debug)]
struct ActiveOpacityTransition {
    from: f32,
    to: f32,
    started_at: Instant,
    duration: Duration,
}

impl TransitionState {
    pub(crate) fn set_specs(&mut self, node: NodeId, transitions: Vec<TransitionSpec>) {
        let specs = transitions
            .into_iter()
            .map(|transition| {
                (
                    transition.property,
                    Duration::from_millis(transition.duration_ms),
                )
            })
            .collect::<HashMap<_, _>>();

        if specs.is_empty() {
            self.specs.remove(&node);
        } else {
            self.specs.insert(node, specs);
        }
    }

    pub(crate) fn clear_node(&mut self, node: NodeId) {
        self.specs.remove(&node);
        self.active.retain(|key, _| key.node != node);
        self.events.retain(|event| event.target != node);
    }

    pub(crate) fn style_color_changed(
        &mut self,
        node: NodeId,
        property: TransitionProperty,
        previous_style_color: Background,
        next_style_color: Background,
        now: Instant,
        truecolor_enabled: bool,
    ) {
        if previous_style_color == next_style_color {
            return;
        }

        let key = TransitionKey { node, property };
        let duration = self
            .specs
            .get(&node)
            .and_then(|specs| specs.get(&property))
            .copied();

        if !truecolor_enabled {
            self.active.remove(&key);
            return;
        }

        let Some(duration) = duration else {
            self.active.remove(&key);
            return;
        };

        if previous_style_color.rgb().is_none() || next_style_color.rgb().is_none() {
            self.active.remove(&key);
            return;
        }

        let from = self
            .active
            .get(&key)
            .and_then(|transition| transition.color_at(now))
            .unwrap_or(previous_style_color);

        self.active.insert(
            key,
            ActiveTransition::Color(ActiveColorTransition {
                from,
                to: next_style_color,
                started_at: now,
                duration,
            }),
        );
        self.events.push_back(TransitionEvent {
            event_type: TransitionEventType::Start,
            target: node,
            property,
        });
    }

    pub(crate) fn paint_color(
        &self,
        node: NodeId,
        property: TransitionProperty,
        style_color: Background,
        now: Instant,
        truecolor_enabled: bool,
    ) -> Background {
        if !truecolor_enabled {
            return style_color;
        }

        self.active
            .get(&TransitionKey { node, property })
            .and_then(|transition| transition.color_at(now))
            .unwrap_or(style_color)
    }

    pub(crate) fn style_opacity_changed(
        &mut self,
        node: NodeId,
        previous_opacity: f32,
        next_opacity: f32,
        now: Instant,
        truecolor_enabled: bool,
    ) {
        if previous_opacity == next_opacity {
            return;
        }

        let property = TransitionProperty::Opacity;
        let key = TransitionKey { node, property };
        let duration = self
            .specs
            .get(&node)
            .and_then(|specs| specs.get(&property))
            .copied();

        if !truecolor_enabled {
            self.active.remove(&key);
            return;
        }

        let Some(duration) = duration else {
            self.active.remove(&key);
            return;
        };
        let from = self
            .active
            .get(&key)
            .and_then(|transition| transition.opacity_at(now))
            .unwrap_or(previous_opacity);

        self.active.insert(
            key,
            ActiveTransition::Opacity(ActiveOpacityTransition {
                from,
                to: next_opacity,
                started_at: now,
                duration,
            }),
        );
        self.events.push_back(TransitionEvent {
            event_type: TransitionEventType::Start,
            target: node,
            property,
        });
    }

    pub(crate) fn paint_opacity(
        &self,
        node: NodeId,
        style_opacity: f32,
        now: Instant,
        truecolor_enabled: bool,
    ) -> f32 {
        if !truecolor_enabled {
            return style_opacity;
        }

        self.active
            .get(&TransitionKey {
                node,
                property: TransitionProperty::Opacity,
            })
            .and_then(|transition| transition.opacity_at(now))
            .unwrap_or(style_opacity)
    }

    pub(crate) fn has_active_opacity(&self, node: NodeId) -> bool {
        matches!(
            self.active.get(&TransitionKey {
                node,
                property: TransitionProperty::Opacity,
            }),
            Some(ActiveTransition::Opacity(_))
        )
    }

    pub(crate) fn finish_completed(&mut self, now: Instant) -> Vec<(NodeId, TransitionProperty)> {
        let completed = self
            .active
            .iter()
            .filter_map(|(key, transition)| transition.is_complete(now).then_some(*key))
            .collect::<Vec<_>>();

        for key in &completed {
            self.active.remove(key);
            self.events.push_back(TransitionEvent {
                event_type: TransitionEventType::End,
                target: key.node,
                property: key.property,
            });
        }
        completed
            .into_iter()
            .map(|key| (key.node, key.property))
            .collect()
    }

    #[cfg(test)]
    pub(crate) fn has_active(&self) -> bool {
        !self.active.is_empty()
    }

    pub(crate) fn drain_events(&mut self) -> Vec<TransitionEvent> {
        self.events.drain(..).collect()
    }
}

impl ActiveColorTransition {
    fn color_at(self, now: Instant) -> Background {
        if self.duration.is_zero() {
            return self.to;
        }
        let progress =
            now.duration_since(self.started_at).as_secs_f32() / self.duration.as_secs_f32();
        interpolate_background(self.from, self.to, progress.clamp(0.0, 1.0)).unwrap_or(self.to)
    }

    fn is_complete(self, now: Instant) -> bool {
        now.duration_since(self.started_at) >= self.duration
    }
}

impl ActiveOpacityTransition {
    fn opacity_at(self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return self.to;
        }
        let progress =
            now.duration_since(self.started_at).as_secs_f32() / self.duration.as_secs_f32();
        interpolate_float(self.from, self.to, progress.clamp(0.0, 1.0))
    }

    fn is_complete(self, now: Instant) -> bool {
        now.duration_since(self.started_at) >= self.duration
    }
}

impl ActiveTransition {
    fn color_at(self, now: Instant) -> Option<Background> {
        match self {
            Self::Color(transition) => Some(transition.color_at(now)),
            Self::Opacity(_) => None,
        }
    }

    fn opacity_at(self, now: Instant) -> Option<f32> {
        match self {
            Self::Opacity(transition) => Some(transition.opacity_at(now)),
            Self::Color(_) => None,
        }
    }

    fn is_complete(self, now: Instant) -> bool {
        match self {
            Self::Color(transition) => transition.is_complete(now),
            Self::Opacity(transition) => transition.is_complete(now),
        }
    }
}

fn interpolate_background(from: Background, to: Background, progress: f32) -> Option<Background> {
    let from = Oklab::from_rgb(from.rgb()?);
    let to = Oklab::from_rgb(to.rgb()?);
    Some(
        Oklab {
            l: interpolate_float(from.l, to.l, progress),
            a: interpolate_float(from.a, to.a, progress),
            b: interpolate_float(from.b, to.b, progress),
        }
        .to_background(),
    )
}

fn interpolate_float(from: f32, to: f32, progress: f32) -> f32 {
    from + (to - from) * progress
}

#[derive(Clone, Copy)]
struct Oklab {
    l: f32,
    a: f32,
    b: f32,
}

impl Oklab {
    fn from_rgb((red, green, blue): (u8, u8, u8)) -> Self {
        let red = srgb_to_linear(red);
        let green = srgb_to_linear(green);
        let blue = srgb_to_linear(blue);

        let l = 0.412_221_46 * red + 0.536_332_55 * green + 0.051_445_995 * blue;
        let m = 0.211_903_5 * red + 0.680_699_5 * green + 0.107_396_96 * blue;
        let s = 0.088_302_46 * red + 0.281_718_85 * green + 0.629_978_7 * blue;

        let l = l.cbrt();
        let m = m.cbrt();
        let s = s.cbrt();

        Self {
            l: 0.210_454_26 * l + 0.793_617_8 * m - 0.004_072_047 * s,
            a: 1.977_998_5 * l - 2.428_592_2 * m + 0.450_593_7 * s,
            b: 0.025_904_037 * l + 0.782_771_77 * m - 0.808_675_77 * s,
        }
    }

    fn to_background(self) -> Background {
        let l = self.l + 0.396_337_78 * self.a + 0.215_803_76 * self.b;
        let m = self.l - 0.105_561_346 * self.a - 0.063_854_17 * self.b;
        let s = self.l - 0.089_484_18 * self.a - 1.291_485_5 * self.b;

        let l = l * l * l;
        let m = m * m * m;
        let s = s * s * s;

        let red = 4.076_741_7 * l - 3.307_711_6 * m + 0.230_969_94 * s;
        let green = -1.268_438 * l + 2.609_757_4 * m - 0.341_319_38 * s;
        let blue = -0.004_196_086_3 * l - 0.703_418_6 * m + 1.707_614_7 * s;

        Background::Rgb(
            linear_to_srgb(red),
            linear_to_srgb(green),
            linear_to_srgb(blue),
        )
    }
}

fn srgb_to_linear(value: u8) -> f32 {
    let value = f32::from(value) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

fn linear_to_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let value = if value <= 0.003_130_8 {
        value * 12.92
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    (value * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(index: usize) -> NodeId {
        NodeId::from(index)
    }

    #[test]
    fn starts_and_ends_color_transition() {
        let mut transitions = TransitionState::default();
        let start = Instant::now();
        transitions.set_specs(
            node(1),
            vec![TransitionSpec {
                property: TransitionProperty::BackgroundColor,
                duration_ms: 100,
            }],
        );

        transitions.style_color_changed(
            node(1),
            TransitionProperty::BackgroundColor,
            Background::Rgb(0, 0, 255),
            Background::Rgb(0, 255, 255),
            start,
            true,
        );

        assert!(transitions.has_active());
        assert_eq!(
            transitions.drain_events(),
            vec![TransitionEvent {
                event_type: TransitionEventType::Start,
                target: node(1),
                property: TransitionProperty::BackgroundColor,
            }]
        );

        let midway = transitions.paint_color(
            node(1),
            TransitionProperty::BackgroundColor,
            Background::Rgb(0, 255, 255),
            start + Duration::from_millis(50),
            true,
        );
        assert!(matches!(midway, Background::Rgb(_, _, _)));
        assert_ne!(midway, Background::Rgb(0, 0, 255));
        assert_ne!(midway, Background::Rgb(0, 255, 255));

        transitions.finish_completed(start + Duration::from_millis(100));
        assert!(!transitions.has_active());
        assert_eq!(
            transitions.drain_events(),
            vec![TransitionEvent {
                event_type: TransitionEventType::End,
                target: node(1),
                property: TransitionProperty::BackgroundColor,
            }]
        );
    }

    #[test]
    fn disables_transitions_without_truecolor() {
        let mut transitions = TransitionState::default();
        let start = Instant::now();
        transitions.set_specs(
            node(1),
            vec![TransitionSpec {
                property: TransitionProperty::Color,
                duration_ms: 100,
            }],
        );

        transitions.style_color_changed(
            node(1),
            TransitionProperty::Color,
            Background::Rgb(255, 0, 0),
            Background::Rgb(0, 255, 0),
            start,
            false,
        );

        assert!(!transitions.has_active());
        assert!(transitions.drain_events().is_empty());
        assert_eq!(
            transitions.paint_color(
                node(1),
                TransitionProperty::Color,
                Background::Rgb(0, 255, 0),
                start + Duration::from_millis(50),
                false,
            ),
            Background::Rgb(0, 255, 0)
        );
    }

    #[test]
    fn starts_paints_and_ends_opacity_transition() {
        let mut transitions = TransitionState::default();
        let start = Instant::now();
        transitions.set_specs(
            node(1),
            vec![TransitionSpec {
                property: TransitionProperty::Opacity,
                duration_ms: 100,
            }],
        );

        transitions.style_opacity_changed(node(1), 1.0, 0.25, start, true);

        assert!(transitions.has_active_opacity(node(1)));
        assert_eq!(
            transitions.drain_events(),
            vec![TransitionEvent {
                event_type: TransitionEventType::Start,
                target: node(1),
                property: TransitionProperty::Opacity,
            }]
        );
        assert_eq!(
            transitions.paint_opacity(node(1), 0.25, start + Duration::from_millis(50), true,),
            0.625,
        );

        assert_eq!(
            transitions.finish_completed(start + Duration::from_millis(100)),
            vec![(node(1), TransitionProperty::Opacity)],
        );
        assert!(!transitions.has_active());
        assert_eq!(
            transitions.drain_events(),
            vec![TransitionEvent {
                event_type: TransitionEventType::End,
                target: node(1),
                property: TransitionProperty::Opacity,
            }]
        );
    }

    #[test]
    fn disables_opacity_transitions_without_truecolor() {
        let mut transitions = TransitionState::default();
        let start = Instant::now();
        transitions.set_specs(
            node(1),
            vec![TransitionSpec {
                property: TransitionProperty::Opacity,
                duration_ms: 100,
            }],
        );

        transitions.style_opacity_changed(node(1), 1.0, 0.25, start, false);

        assert!(!transitions.has_active());
        assert!(transitions.drain_events().is_empty());
        assert_eq!(
            transitions.paint_opacity(node(1), 0.25, start + Duration::from_millis(50), false,),
            0.25,
        );
    }
}
