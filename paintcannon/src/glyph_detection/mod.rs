//! Font-set detection. Each polling pass shares a probe across callers.
//! Samples indicate a patched font, not full coverage
//! of every Nerd Fonts version or shaping sequence.
mod capabilities;
mod environment;
mod environment_markers;
mod legacy;
mod monitor;
mod ssh;
mod terminals;
mod tmux;
pub(crate) use capabilities::HyperlinkCapability;
pub(crate) use legacy::HasSymbolsForLegacyComputing;
pub(crate) use monitor::FontMonitor;
pub(crate) mod protocol;

use crossbeam_channel::{bounded, never, select, Receiver, Sender};
use napi::{Env, Result, Task};
use protocol::{Request, QUERY_TIMEOUT};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

#[napi_derive::napi(string_enum = "kebab-case")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NerdFontSet {
    Powerline,
    PowerlineExtended,
    FontAwesome,
    Devicons,
    MaterialDesign,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NerdFontSupport {
    pub(crate) powerline: bool,
    pub(crate) powerline_extended: bool,
    pub(crate) font_awesome: bool,
    pub(crate) devicons: bool,
    pub(crate) material_design: bool,
}

impl NerdFontSupport {
    fn uniform(supported: bool) -> Self {
        Self {
            powerline: supported,
            powerline_extended: supported,
            font_awesome: supported,
            devicons: supported,
            material_design: supported,
        }
    }

    fn supports(self, set: NerdFontSet) -> bool {
        match set {
            NerdFontSet::Powerline => self.powerline,
            NerdFontSet::PowerlineExtended => self.powerline_extended,
            NerdFontSet::FontAwesome => self.font_awesome,
            NerdFontSet::Devicons => self.devicons,
            NerdFontSet::MaterialDesign => self.material_design,
        }
    }
}

// Stable representatives from Nerd Fonts v3.3.0 glyphnames.json.
const SAMPLES: &[(NerdFontSet, &[char])] = &[
    (
        NerdFontSet::Powerline,
        &['\u{e0a0}', '\u{e0a1}', '\u{e0b0}'],
    ),
    (
        NerdFontSet::PowerlineExtended,
        &['\u{e0b8}', '\u{e0ba}', '\u{e0bc}', '\u{e0be}'],
    ),
    (
        NerdFontSet::FontAwesome,
        &['\u{f07b}', '\u{f26e}', '\u{f29b}'],
    ),
    (NerdFontSet::Devicons, &['\u{e700}', '\u{e701}', '\u{e705}']),
    (
        NerdFontSet::MaterialDesign,
        &['\u{f0001}', '\u{f01c9}', '\u{f16e0}'],
    ),
];

fn sample_characters() -> impl Iterator<Item = char> {
    SAMPLES
        .iter()
        .flat_map(|(_, samples)| samples.iter().copied())
}

pub(crate) struct Detection {
    environment: OnceLock<environment::Environment>,
    probed: OnceLock<ProbeReport>,
    stop: Receiver<()>,
}

impl Default for Detection {
    fn default() -> Self {
        Self {
            environment: OnceLock::new(),
            probed: OnceLock::new(),
            stop: never(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct ProbeReport {
    nerd_fonts: NerdFontSupport,
    legacy_symbols: bool,
}

impl Detection {
    fn environment(&self) -> &environment::Environment {
        self.environment
            .get_or_init(environment::Environment::current)
    }

    pub(crate) fn known_nerd_fonts(&self) -> Option<NerdFontSupport> {
        let env = self.environment();
        env.nerd_font_override()
            .map(NerdFontSupport::uniform)
            .or_else(|| {
                env.capabilities()
                    .nerd_font
                    .then_some(NerdFontSupport::uniform(true))
            })
    }

    pub(crate) fn known_legacy_symbols(&self) -> Option<bool> {
        self.environment()
            .capabilities()
            .symbols_for_legacy_computing
            .then_some(true)
    }

    pub(crate) fn detect(&self, queries: Option<&Sender<Request>>) -> NerdFontSupport {
        self.known_nerd_fonts()
            .unwrap_or_else(|| self.probe(queries).nerd_fonts)
    }

    pub(crate) fn detect_legacy(&self, queries: Option<&Sender<Request>>) -> bool {
        self.known_legacy_symbols()
            .unwrap_or_else(|| self.probe(queries).legacy_symbols)
    }

    #[cfg(test)]
    fn report(
        &self,
        environment: impl FnOnce() -> environment::Environment,
        queries: Option<&Sender<Request>>,
    ) -> NerdFontSupport {
        self.environment.get_or_init(environment);
        self.detect(queries)
    }

    fn probe(&self, queries: Option<&Sender<Request>>) -> &ProbeReport {
        self.probed.get_or_init(|| {
            let known_nerd_fonts = self.known_nerd_fonts();
            let known_legacy_symbols = self.known_legacy_symbols();
            let mut characters = Vec::new();
            if known_nerd_fonts.is_none() {
                characters.extend(sample_characters());
            }
            let nerd_count = characters.len();
            if known_legacy_symbols.is_none() {
                characters.extend_from_slice(legacy::SAMPLES);
            }
            let count = characters.len();
            let (reply, response) = bounded(1);
            let deadline = Instant::now() + QUERY_TIMEOUT;
            let request = Request {
                characters,
                deadline,
                reply,
            };
            let queries = queries.filter(|_| self.environment().can_probe());
            let reported = match queries {
                Some(queries) if queries.try_send(request).is_ok() => {
                    // Allow the input thread one polling cycle to deliver partial
                    // results at the safety deadline instead of discarding them.
                    select! {
                        recv(response) -> result => result.ok(),
                        recv(self.stop) -> _ => None,
                        default(deadline.saturating_duration_since(Instant::now()) + std::time::Duration::from_millis(100)) => None,
                    }
                }
                _ => None,
            }
            .unwrap_or_else(|| vec![None; count]);
            ProbeReport {
                nerd_fonts: known_nerd_fonts
                    .unwrap_or_else(|| nerd_fonts_from_replies(&reported[..nerd_count])),
                legacy_symbols: known_legacy_symbols.unwrap_or_else(|| {
                    reported[nerd_count..]
                        .iter()
                        .all(|reply| *reply == Some(true))
                }),
            }
        })
    }
}

pub struct HasNerdFontSet {
    set: NerdFontSet,
    detection: Arc<Detection>,
    queries: Option<Sender<Request>>,
}

impl HasNerdFontSet {
    pub(crate) fn new(
        set: NerdFontSet,
        detection: Arc<Detection>,
        queries: Option<Sender<Request>>,
    ) -> Self {
        Self {
            set,
            detection,
            queries,
        }
    }
}

#[napi_derive::napi]
impl Task for HasNerdFontSet {
    type Output = bool;
    type JsValue = bool;

    fn compute(&mut self) -> Result<bool> {
        let report = self.detection.detect(self.queries.as_ref());
        Ok(report.supports(self.set))
    }

    fn resolve(&mut self, _env: Env, output: bool) -> Result<bool> {
        Ok(output)
    }
}

fn nerd_fonts_from_replies(reported: &[Option<bool>]) -> NerdFontSupport {
    let mut offset = 0;
    let mut result = NerdFontSupport::default();
    for (set, samples) in SAMPLES {
        let supported = match set {
            NerdFontSet::Powerline => &mut result.powerline,
            NerdFontSet::PowerlineExtended => &mut result.powerline_extended,
            NerdFontSet::FontAwesome => &mut result.font_awesome,
            NerdFontSet::Devicons => &mut result.devicons,
            NerdFontSet::MaterialDesign => &mut result.material_design,
        };
        *supported = reported[offset..offset + samples.len()]
            .iter()
            .all(|reply| *reply == Some(true));
        offset += samples.len();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use environment::Environment;

    #[test]
    fn native_tasks_return_the_requested_category_from_mixed_probe_results() {
        let cases = [
            (NerdFontSet::Powerline, '\u{e0a0}'),
            (NerdFontSet::PowerlineExtended, '\u{e0ba}'),
            (NerdFontSet::FontAwesome, '\u{f07b}'),
            (NerdFontSet::Devicons, '\u{e700}'),
            (NerdFontSet::MaterialDesign, '\u{f0001}'),
        ];
        for (unsupported, glyph) in cases {
            for reply in [Some(false), None] {
                let detection = Arc::new(Detection::default());
                detection
                    .environment
                    .set(Environment::from_pairs(&[]))
                    .ok()
                    .unwrap();
                let (tx, rx) = bounded::<Request>(1);
                std::thread::scope(|scope| {
                    let worker = scope.spawn(|| {
                        HasNerdFontSet::new(unsupported, detection.clone(), Some(tx.clone()))
                            .compute()
                            .unwrap()
                    });
                    let request = rx.recv_timeout(QUERY_TIMEOUT).unwrap();
                    assert!(request.characters.contains(&glyph));
                    let replies = request
                        .characters
                        .iter()
                        .map(|&character| {
                            if character == glyph {
                                reply
                            } else {
                                Some(true)
                            }
                        })
                        .collect();
                    request.reply.send(replies).unwrap();
                    assert!(!worker.join().unwrap());
                });
                for (set, _) in cases {
                    let supported = HasNerdFontSet::new(set, detection.clone(), Some(tx.clone()))
                        .compute()
                        .unwrap();
                    assert_eq!(
                        supported,
                        set != unsupported,
                        "{set:?}, unsupported: {unsupported:?}, reply: {reply:?}",
                    );
                }
                assert!(
                    rx.is_empty(),
                    "category lookups must reuse the shared probe"
                );
            }
        }
    }

    #[test]
    fn known_terminal_support_skips_the_protocol() {
        let detection = Detection::default();
        let (tx, rx) = bounded::<Request>(1);
        let (result, queried) = std::thread::scope(|scope| {
            let worker = scope.spawn(|| {
                detection.report(
                    || Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")]),
                    Some(&tx),
                )
            });
            let queried = if let Ok(request) = rx.recv_timeout(std::time::Duration::from_millis(50))
            {
                request
                    .reply
                    .send(vec![Some(false); request.characters.len()])
                    .unwrap();
                true
            } else {
                false
            };
            (worker.join().unwrap(), queried)
        });
        assert!(
            !queried,
            "known terminal support must not trigger a glyph query"
        );
        assert_eq!(result, NerdFontSupport::uniform(true));
    }

    #[test]
    fn blacklisted_terminal_detection_never_writes_glyph_queries() {
        for pairs in [
            vec![("TERM_PROGRAM", "Apple_Terminal")],
            vec![("TERM_PROGRAM", "Apple_Terminal"), ("NERD_FONT", "1")],
            vec![("TERM_PROGRAM", "Apple_Terminal"), ("NERD_FONT", "0")],
            vec![("TERM_PROGRAM", "terminology")],
            vec![("TERM", "rxvt-unicode-256color")],
        ] {
            for tmux in [false, true] {
                for _ in 0..3 {
                    let mut environment = Environment::from_pairs(&pairs);
                    if tmux {
                        let client = environment.capabilities();
                        environment = environment.with_clients(vec![client]);
                    }
                    let expected_fonts =
                        NerdFontSupport::uniform(environment.nerd_font_override().unwrap_or(false));
                    let detection = Detection {
                        environment: OnceLock::from(environment),
                        ..Detection::default()
                    };
                    let (tx, mut queries) = protocol::Queries::new();
                    assert_eq!(detection.detect(Some(&tx)), expected_fonts);
                    assert!(!detection.detect_legacy(Some(&tx)));
                    assert!(!queries.has_work());
                    let mut output = Vec::new();
                    queries.poll(true, &mut output, tmux, Instant::now());
                    assert!(output.is_empty());
                }
            }
        }
    }

    #[test]
    fn multiple_clients_cannot_be_confirmed_by_one_anonymous_glyph_reply() {
        let ghostty = Environment::from_pairs(&[("TERM_PROGRAM", "ghostty")]).capabilities();
        let apple = Environment::from_pairs(&[("TERM_PROGRAM", "Apple_Terminal")]).capabilities();
        let env = Environment::from_pairs(&[]).with_clients(vec![ghostty, apple]);
        let detection = Detection {
            environment: OnceLock::from(env),
            ..Detection::default()
        };
        let (tx, rx) = bounded(1);
        assert_eq!(detection.detect(Some(&tx)), NerdFontSupport::default());
        assert!(!detection.detect_legacy(Some(&tx)));
        assert!(rx.is_empty());
    }

    #[test]
    fn forced_nerd_fonts_still_win_with_multiple_or_no_clients() {
        for clients in [vec![], vec![Default::default(), Default::default()]] {
            for value in ["0", "1"] {
                let env =
                    Environment::from_pairs(&[("NERD_FONT", value)]).with_clients(clients.clone());
                let detection = Detection {
                    environment: OnceLock::from(env),
                    ..Detection::default()
                };
                assert_eq!(
                    detection.detect(None),
                    NerdFontSupport::uniform(value == "1")
                );
                assert!(!detection.detect_legacy(None));
            }
        }
    }

    #[test]
    fn refreshing_an_unknown_terminal_does_not_reuse_an_old_probe_result() {
        let (tx, rx) = bounded::<Request>(1);
        for reply in [true, false, true] {
            let detection = Detection {
                environment: OnceLock::from(Environment::from_pairs(&[])),
                ..Detection::default()
            };
            std::thread::scope(|scope| {
                let worker = scope.spawn(|| detection.detect(Some(&tx)));
                let request = rx.recv_timeout(QUERY_TIMEOUT).unwrap();
                request
                    .reply
                    .send(vec![Some(reply); request.characters.len()])
                    .unwrap();
                assert_eq!(worker.join().unwrap(), NerdFontSupport::uniform(reply));
            });
            assert_eq!(detection.detect_legacy(Some(&tx)), reply);
            assert!(rx.is_empty());
        }
    }

    #[test]
    fn present_override_enables_nerd_fonts_without_queries() {
        let env = Environment::from_pairs(&[("NERD_FONT", "")]);
        let (tx, rx) = bounded(1);
        assert_eq!(
            Detection::default().report(|| env, Some(&tx)),
            NerdFontSupport::uniform(true)
        );
        assert!(rx.is_empty());
    }

    #[test]
    fn zero_override_disables_even_bundled_fonts_without_queries() {
        let env = Environment::from_pairs(&[("NERD_FONT", "0"), ("TERM_PROGRAM", "ghostty")]);
        let (tx, rx) = bounded(1);
        assert_eq!(
            Detection::default().report(|| env, Some(&tx)),
            NerdFontSupport::default()
        );
        assert!(rx.is_empty());
    }

    #[test]
    fn concurrent_calls_probe_all_sets_once_and_cache_results() {
        let detection = Arc::new(Detection::default());
        detection
            .environment
            .set(Environment::from_pairs(&[]))
            .ok()
            .unwrap();
        let (tx, rx) = bounded::<Request>(1);
        let callers: Vec<_> = SAMPLES
            .iter()
            .map(|&(set, _)| {
                let detection = detection.clone();
                let tx = tx.clone();
                std::thread::spawn(move || {
                    detection
                        .report(|| Environment::from_pairs(&[]), Some(&tx))
                        .supports(set)
                })
            })
            .collect();
        let legacy_caller = {
            let detection = detection.clone();
            let tx = tx.clone();
            std::thread::spawn(move || detection.detect_legacy(Some(&tx)))
        };
        let request = rx.recv_timeout(QUERY_TIMEOUT).unwrap();
        assert_eq!(
            request.characters.len(),
            sample_characters().count() + legacy::SAMPLES.len()
        );
        request
            .reply
            .send(vec![Some(true); request.characters.len()])
            .unwrap();
        for caller in callers {
            assert!(caller.join().unwrap());
        }
        assert!(legacy_caller.join().unwrap());
        assert_eq!(
            detection.report(
                || panic!("cached checks must not query tmux again"),
                Some(&tx)
            ),
            NerdFontSupport::uniform(true)
        );
        assert!(rx.is_empty());
    }

    #[test]
    fn known_sets_return_while_unknown_sets_are_being_probed() {
        for (program, known_nerd) in [("WezTerm", true), ("kitty", false)] {
            let detection = Detection::default();
            detection
                .environment
                .set(Environment::from_pairs(&[("TERM_PROGRAM", program)]))
                .ok()
                .unwrap();
            let (tx, rx) = bounded::<Request>(1);
            std::thread::scope(|scope| {
                let worker = scope.spawn(|| {
                    if known_nerd {
                        detection.detect_legacy(Some(&tx))
                    } else {
                        detection.detect(Some(&tx)).powerline_extended
                    }
                });
                let request = rx.recv_timeout(QUERY_TIMEOUT).unwrap();
                if known_nerd {
                    assert_eq!(request.characters, legacy::SAMPLES);
                    assert_eq!(detection.detect(Some(&tx)), NerdFontSupport::uniform(true));
                } else {
                    assert_eq!(request.characters, sample_characters().collect::<Vec<_>>());
                    assert!(detection.detect_legacy(Some(&tx)));
                }
                request
                    .reply
                    .send(vec![Some(true); request.characters.len()])
                    .unwrap();
                assert!(worker.join().unwrap());
            });
            assert!(rx.is_empty());
        }
    }

    #[test]
    fn false_terminal_flags_allow_positive_protocol_replies() {
        let detection = Detection::default();
        detection
            .environment
            .set(Environment::from_pairs(&[("TERM_PROGRAM", "alacritty")]))
            .ok()
            .unwrap();
        let (tx, rx) = bounded::<Request>(1);
        std::thread::scope(|scope| {
            let worker = scope.spawn(|| detection.detect_legacy(Some(&tx)));
            let request = rx.recv_timeout(QUERY_TIMEOUT).unwrap();
            assert_eq!(
                request.characters.len(),
                sample_characters().count() + legacy::SAMPLES.len()
            );
            request
                .reply
                .send(vec![Some(true); request.characters.len()])
                .unwrap();
            assert!(worker.join().unwrap());
        });
        assert_eq!(detection.detect(Some(&tx)), NerdFontSupport::uniform(true));
        assert!(rx.is_empty());
    }

    #[test]
    fn nerd_font_overrides_are_preserved_while_legacy_symbols_are_probed() {
        for override_value in ["0", "1", ""] {
            for reply in [Some(true), Some(false), None] {
                let detection = Detection::default();
                detection
                    .environment
                    .set(Environment::from_pairs(&[("NERD_FONT", override_value)]))
                    .ok()
                    .unwrap();
                let expected = NerdFontSupport::uniform(override_value != "0");
                let (tx, rx) = bounded::<Request>(1);
                std::thread::scope(|scope| {
                    let worker = scope.spawn(|| detection.detect_legacy(Some(&tx)));
                    let request = rx.recv_timeout(QUERY_TIMEOUT).unwrap();
                    assert_eq!(request.characters, legacy::SAMPLES);
                    assert_eq!(detection.detect(Some(&tx)), expected);
                    let mut replies = vec![Some(true); legacy::SAMPLES.len()];
                    replies[1] = reply;
                    request.reply.send(replies).unwrap();
                    assert_eq!(worker.join().unwrap(), reply == Some(true));
                });
                assert_eq!(detection.probe(Some(&tx)).nerd_fonts, expected);
                assert_eq!(detection.detect(Some(&tx)), expected);
                assert!(rx.is_empty());
            }
        }
    }

    #[test]
    fn missing_sample_only_disables_its_own_set() {
        for reply in [Some(false), None] {
            let replies: Vec<_> = sample_characters()
                .map(|character| {
                    if character == '\u{e0b8}' {
                        reply
                    } else {
                        Some(true)
                    }
                })
                .collect();
            assert_eq!(
                nerd_fonts_from_replies(&replies),
                NerdFontSupport {
                    powerline: true,
                    powerline_extended: false,
                    font_awesome: true,
                    devicons: true,
                    material_design: true,
                }
            );
        }
    }

    #[test]
    fn remote_unknown_terminal_never_inspects_server_fonts() {
        let env = Environment::from_pairs(&[("SSH_TTY", "/dev/pts/1")]);
        assert_eq!(
            Detection::default().report(|| env, None),
            NerdFontSupport::default()
        );
    }

    #[test]
    fn ghostty_inside_tmux_uses_bundled_symbols_before_the_protocol() {
        let env = Environment::from_pairs(&[
            ("TERM", "tmux-256color"),
            ("TERM_PROGRAM", "tmux"),
            ("TMUX", "/tmp/tmux-1000/default,10681,0"),
        ])
        .with_clients(vec![Environment::from_pairs(&[(
            "TERM_PROGRAM",
            "ghostty",
        )])
        .capabilities()]);
        let (tx, rx) = bounded(1);
        assert_eq!(
            Detection::default().report(|| env, Some(&tx)),
            NerdFontSupport::uniform(true)
        );
        assert!(rx.is_empty());
    }

    #[test]
    fn unknown_tmux_terminal_does_not_guess_from_installed_terminal_resources() {
        let env = Environment::from_pairs(&[
            ("TERM_PROGRAM", "tmux"),
            ("TERM", "tmux-256color"),
            ("TMUX", "/tmp/tmux-1000/default,10681,0"),
        ]);
        assert_eq!(
            Detection::default().report(|| env, None),
            NerdFontSupport::default()
        );
    }
}
