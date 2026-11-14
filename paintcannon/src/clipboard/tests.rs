use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};

fn test_pastes() -> Pastes {
    Pastes {
        namespace: "pc-paste".into(),
        ..Pastes::default()
    }
}

fn packet(metadata: &str, data: &[u8]) -> Packet {
    protocol::parse_packet(
        format!("5522;type=read:{metadata};{}", STANDARD.encode(data)).as_bytes(),
    )
    .unwrap()
}

fn listing(
    pastes: &mut Pastes,
    queue: &NativeEventQueue,
    out: &mut Vec<u8>,
    types: &[u8],
    now: Instant,
) {
    pastes.receive(
        packet("status=OK:loc=primary:pw=c2VjcmV0", b""),
        queue,
        out,
        now,
    );
    pastes.receive(packet("status=DATA:mime=Lg==", types), queue, out, now);
    pastes.receive(packet("status=DONE", b""), queue, out, now);
}

#[test]
fn shutdown_resets_inherited_rich_paste_mode() {
    for tmux in [false, true] {
        let mut mode = PasteMode::new(tmux);
        let mut out = Vec::new();
        mode.query_with_tmux_setup(&mut out, || true).unwrap();
        // A previous process crashed with rich paste enabled.
        mode.report(1, &mut out).unwrap();
        assert!(mode.enabled);
        out.clear();
        mode.restore(&mut out).unwrap();
        let mut expected = Vec::new();
        write_sequence(&mut expected, protocol::DISABLE, tmux).unwrap();
        assert_eq!(out, expected, "tmux={tmux}");
        assert!(!mode.enabled);
    }
}

#[test]
fn shutdown_resets_rich_paste_even_without_a_query_reply() {
    for tmux in [false, true] {
        for query in [false, true] {
            let mut mode = PasteMode::new(tmux);
            let mut out = Vec::new();
            if query {
                mode.query_with_tmux_setup(&mut out, || true).unwrap();
            }
            let mut expected = Vec::new();
            write_sequence(&mut expected, protocol::DISABLE, tmux).unwrap();
            for _ in 0..2 {
                out.clear();
                mode.restore(&mut out).unwrap();
                assert_eq!(out, expected, "tmux={tmux}, query={query}");
                mode.report(2, &mut out).unwrap();
                assert_eq!(out, expected, "late reply re-enabled paste after shutdown");
            }
        }
    }
}

#[test]
fn shutdown_retries_a_failed_rich_paste_reset() {
    let mut mode = PasteMode::default();
    mode.query(&mut Vec::new()).unwrap();
    mode.report(2, &mut Vec::new()).unwrap();
    let mut full_buffer = &mut [][..];
    assert!(mode.restore(&mut full_buffer).is_err());
    let mut out = Vec::new();
    mode.restore(&mut out).unwrap();
    assert_eq!(out, protocol::DISABLE);
}

#[test]
fn focus_return_reenables_rich_paste_after_another_program_resets_it() {
    for tmux in [false, true] {
        let mut mode = PasteMode::new(tmux);
        let mut out = Vec::new();
        mode.query_with_tmux_setup(&mut out, || true).unwrap();
        mode.report(2, &mut out).unwrap();
        mode.set_focused(false, &mut out).unwrap();
        // Another program resets the terminal mode while we are unfocused.
        out.clear();
        mode.set_focused(true, &mut out).unwrap();
        mode.query_with_tmux_setup(&mut out, || true).unwrap();
        let mut expected = Vec::new();
        write_sequence(&mut expected, protocol::QUERY, tmux).unwrap();
        assert_eq!(
            out, expected,
            "tmux={tmux}: must query the current terminal mode"
        );
        mode.report(2, &mut out).unwrap();
        write_sequence(&mut expected, protocol::ENABLE, tmux).unwrap();
        assert_eq!(out, expected);
        assert!(mode.enabled);
        out.clear();
        // The input thread checks the focus state on every iteration.
        for _ in 0..3 {
            mode.set_focused(true, &mut out).unwrap();
            mode.query_with_tmux_setup(&mut out, || panic!("repeated query"))
                .unwrap();
        }
        assert!(out.is_empty());
    }
}

#[test]
fn tmux_mode_queries_and_changes_reach_the_outer_terminal() {
    let mut mode = PasteMode {
        tmux: true,
        ..PasteMode::default()
    };
    let mut out = Vec::new();
    mode.query_with_tmux_setup(&mut out, || true).unwrap();
    assert_eq!(out, b"\x1bPtmux;\x1b\x1b[?5522$p\x1b\\");
    out.clear();
    mode.report(2, &mut out).unwrap();
    assert_eq!(out, b"\x1bPtmux;\x1b\x1b[?5522h\x1b\\");
    assert!(mode.enabled);
    out.clear();
    mode.restore(&mut out).unwrap();
    assert_eq!(out, b"\x1bPtmux;\x1b\x1b[?5522l\x1b\\");
}

#[test]
fn tmux_clipboard_read_escapes_both_osc_and_its_terminator() {
    let mut pastes = Pastes {
        tmux: true,
        ..test_pastes()
    };
    let queue = NativeEventQueue::default();
    let mut out = Vec::new();
    listing(&mut pastes, &queue, &mut out, b"image/png", Instant::now());
    assert_eq!(out, b"\x1bPtmux;\x1b\x1b]5522;type=read:id=pc-paste-1:loc=primary:pw=c2VjcmV0:name=UGFzdGUgZXZlbnQ=;aW1hZ2UvcG5n\x1b\x1b\\\x1b\\");
}

#[test]
fn tmux_focus_loss_releases_mode_and_focus_gain_renegotiates() {
    let mut mode = PasteMode::new(true);
    let mut out = Vec::new();
    mode.query_with_tmux_setup(&mut out, || true).unwrap();
    mode.report(2, &mut out).unwrap();
    out.clear();
    mode.set_focused(false, &mut out).unwrap();
    assert_eq!(out, b"\x1bPtmux;\x1b\x1b[?5522l\x1b\\");
    assert!(!mode.enabled);
    // Blur changes activation, not the reported capability.
    assert!(mode.support().wait(Duration::ZERO));
    out.clear();
    mode.set_focused(false, &mut out).unwrap();
    mode.query_with_tmux_setup(&mut out, || panic!("query while unfocused"))
        .unwrap();
    mode.report(2, &mut out).unwrap(); // A delayed report must not re-enable it.
    assert!(out.is_empty());
    mode.set_focused(true, &mut out).unwrap();
    mode.query_with_tmux_setup(&mut out, || true).unwrap();
    mode.report(2, &mut out).unwrap();
    assert_eq!(
        out,
        b"\x1bPtmux;\x1b\x1b[?5522$p\x1b\\\x1bPtmux;\x1b\x1b[?5522h\x1b\\"
    );
    assert!(mode.enabled);
}

#[test]
fn direct_terminal_blur_keeps_mode_active_but_ignores_late_reports() {
    let mut mode = PasteMode::new(false);
    let mut out = Vec::new();
    mode.query_with_tmux_setup(&mut out, || panic!("tmux queried outside tmux"))
        .unwrap();
    mode.report(2, &mut out).unwrap();
    out.clear();
    mode.set_focused(false, &mut out).unwrap();
    mode.query(&mut out).unwrap();
    mode.report(2, &mut out).unwrap();
    assert!(out.is_empty());
    assert!(mode.enabled);
}

#[test]
fn tmux_without_confirmed_focus_events_does_not_query_or_enable_outer_paste_mode() {
    let mut mode = PasteMode::new(true);
    let mut out = Vec::new();
    mode.query_with_tmux_setup(&mut out, || false).unwrap();
    assert!(!mode.support().wait(Duration::ZERO));
    assert!(out.is_empty());
    mode.query_with_tmux_setup(&mut out, || panic!("query repeated every input tick"))
        .unwrap();
    mode.report(2, &mut out).unwrap(); // Ignore unsolicited/late capability replies.
    assert!(!mode.enabled);
    assert!(out.is_empty());
    mode.restore(&mut out).unwrap();
    assert_eq!(out, b"\x1bPtmux;\x1b\x1b[?5522l\x1b\\");
    // A resume may happen after the user changes the server option.
    mode.query_with_tmux_setup(&mut out, || true).unwrap();
    mode.report(2, &mut out).unwrap();
    assert!(mode.enabled);
}

#[test]
fn tmux_rechecks_focus_events_before_reenabling_on_focus_return() {
    let mut mode = PasteMode::new(true);
    let mut out = Vec::new();
    mode.query_with_tmux_setup(&mut out, || true).unwrap();
    mode.report(2, &mut out).unwrap();
    mode.set_focused(false, &mut out).unwrap();
    out.clear();
    mode.set_focused(true, &mut out).unwrap();
    mode.query_with_tmux_setup(&mut out, || false).unwrap();
    assert!(out.is_empty());
    assert!(!mode.enabled);
    assert!(!mode.support().wait(Duration::ZERO));
}

#[test]
fn queries_support_without_reading_the_clipboard_and_restores_mode() {
    for status in 0..=4 {
        let mut mode = PasteMode::default();
        let mut out = Vec::new();
        mode.query(&mut out).unwrap();
        mode.query(&mut out).unwrap();
        assert_eq!(out, protocol::QUERY);
        out.clear();
        mode.report(status, &mut out).unwrap();
        assert_eq!(mode.enabled, matches!(status, 1..=3));
        assert_eq!(mode.support().wait(Duration::ZERO), matches!(status, 1..=3));
        assert_eq!(out, if status == 2 { protocol::ENABLE } else { b"" });
        out.clear();
        mode.restore(&mut out).unwrap();
        assert_eq!(out, protocol::DISABLE);
        assert!(!mode.enabled);
        out.clear();
        mode.report(2, &mut out).unwrap(); // Late response while suspended.
        assert!(out.is_empty());
        mode.query(&mut out).unwrap(); // Resume re-negotiates.
        assert_eq!(out, protocol::QUERY);
    }
}

#[test]
fn support_waits_for_the_native_reply_and_is_shared_across_callers() {
    let mut mode = PasteMode::default();
    mode.query(&mut Vec::new()).unwrap();
    let support = mode.support();
    let (started_tx, started_rx) = std::sync::mpsc::channel();
    let (result_tx, result_rx) = std::sync::mpsc::channel();
    let waiter = std::thread::spawn(move || {
        started_tx.send(()).unwrap();
        result_tx
            .send(support.wait(Duration::from_secs(1)))
            .unwrap();
    });
    started_rx.recv().unwrap();
    assert!(matches!(
        result_rx.recv_timeout(Duration::from_millis(10)),
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    mode.report(2, &mut Vec::new()).unwrap();
    assert!(result_rx.recv_timeout(Duration::from_secs(1)).unwrap());
    waiter.join().unwrap();
    assert!(mode.support().wait(Duration::ZERO));
}

#[test]
fn unanswered_support_query_fails_closed_but_late_replies_can_update_it() {
    let mut mode = PasteMode::default();
    mode.query(&mut Vec::new()).unwrap();
    let support = mode.support();
    assert!(!support.wait(Duration::ZERO));
    assert!(!support.wait(Duration::ZERO));
    mode.report(2, &mut Vec::new()).unwrap();
    assert!(support.wait(Duration::ZERO));
}

#[test]
fn releasing_terminal_resolves_pending_support_and_resume_updates_the_same_handle() {
    let mut mode = PasteMode::default();
    mode.query(&mut Vec::new()).unwrap();
    let support = mode.support();
    mode.restore(&mut Vec::new()).unwrap();
    assert!(!support.wait(Duration::ZERO));
    mode.query(&mut Vec::new()).unwrap();
    mode.report(2, &mut Vec::new()).unwrap();
    assert!(support.wait(Duration::ZERO));
    mode.restore(&mut Vec::new()).unwrap();
    assert!(!support.wait(Duration::ZERO));
}

#[test]
fn output_failures_and_missing_input_do_not_claim_rich_paste_support() {
    struct BrokenOutput;
    impl std::io::Write for BrokenOutput {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut mode = PasteMode::default();
    assert!(mode.query(&mut BrokenOutput).is_err());
    assert!(!mode.support().wait(Duration::ZERO));
    mode.query(&mut Vec::new()).unwrap();
    assert!(mode.report(2, &mut BrokenOutput).is_err());
    assert!(!mode.support().wait(Duration::ZERO));
    let mut out = Vec::new();
    mode.restore(&mut out).unwrap();
    assert_eq!(out, protocol::DISABLE);
    assert!(!napi::Task::compute(&mut RichPasteSupport { support: None }).unwrap());
}

#[test]
fn image_paste_uses_token_and_location_and_preserves_native_event_order() {
    let mut pastes = test_pastes();
    let queue = NativeEventQueue::default();
    let mut out = Vec::new();
    let now = Instant::now();
    queue.push(NativeEvent::paste("before".into()));
    listing(
        &mut pastes,
        &queue,
        &mut out,
        b" image/jpeg\timage/png\ntext/plain\n",
        now,
    );
    assert_eq!(out, b"\x1b]5522;type=read:id=pc-paste-1:loc=primary:pw=c2VjcmV0:name=UGFzdGUgZXZlbnQ=;aW1hZ2UvcG5n\x1b\\");
    queue.push(NativeEvent::paste("after".into()));
    queue.push(NativeEvent::focus(
        crate::native_event::TerminalFocusEvent {
            r#type: "blur".into(),
        },
    ));
    let events = queue.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].paste.as_deref(), Some("before"));
    pastes.receive(
        packet("id=pc-paste-1:status=OK", b""),
        &queue,
        &mut out,
        now,
    );
    let first = vec![0xa5; 4096];
    pastes.receive(
        packet("id=pc-paste-1:status=DATA:mime=aW1hZ2UvcG5n", &first),
        &queue,
        &mut out,
        now,
    );
    assert!(queue.drain().is_empty());
    pastes.receive(
        packet("id=pc-paste-1:status=DATA:mime=aW1hZ2UvcG5n", &[0, 0xff]),
        &queue,
        &mut out,
        now,
    );
    pastes.receive(
        packet("id=pc-paste-1:status=DONE", b""),
        &queue,
        &mut out,
        now,
    );
    let events = queue.drain();
    assert_eq!(events.len(), 3);
    let image = events[0].paste_image.as_ref().unwrap();
    assert_eq!(image.mime_type, "image/png");
    assert_eq!(&image.data[..4096], first);
    assert_eq!(&image.data[4096..], &[0, 0xff]);
    assert_eq!(events[0].paste.as_deref(), Some(""));
    assert_eq!(events[1].paste.as_deref(), Some("after"));
    assert_eq!(events[2].kind, "focus");
}

#[test]
fn overlapping_pastes_keep_their_original_order_even_if_replies_do_not() {
    let mut pastes = test_pastes();
    let queue = NativeEventQueue::default();
    let mut out = Vec::new();
    let now = Instant::now();
    listing(&mut pastes, &queue, &mut out, b"text/plain", now);
    queue.push(NativeEvent::paste("between".into()));
    listing(&mut pastes, &queue, &mut out, b"text/plain", now);
    for id in [2, 1] {
        pastes.receive(
            packet(&format!("id=pc-paste-{id}:status=OK"), b""),
            &queue,
            &mut out,
            now,
        );
        pastes.receive(
            packet(
                &format!("id=pc-paste-{id}:status=DATA:mime=dGV4dC9wbGFpbg=="),
                id.to_string().as_bytes(),
            ),
            &queue,
            &mut out,
            now,
        );
        pastes.receive(
            packet(&format!("id=pc-paste-{id}:status=DONE"), b""),
            &queue,
            &mut out,
            now,
        );
        if id == 2 {
            assert!(queue.drain().is_empty());
        }
    }
    assert_eq!(
        queue
            .drain()
            .iter()
            .map(|event| event.paste.as_deref().unwrap())
            .collect::<Vec<_>>(),
        ["1", "between", "2"]
    );
}

#[test]
fn denied_invalid_and_stalled_reads_release_later_events_without_partial_images() {
    for ending in [
        "EPERM",
        "EBUSY",
        "ENOSYS",
        "invalid",
        "timeout",
        "suspend",
        "wrong-mime",
    ] {
        let mut pastes = test_pastes();
        let queue = NativeEventQueue::default();
        let mut out = Vec::new();
        let now = Instant::now();
        listing(&mut pastes, &queue, &mut out, b"image/png", now);
        queue.push(NativeEvent::paste("after".into()));
        pastes.receive(
            packet("id=pc-paste-1:status=OK", b""),
            &queue,
            &mut out,
            now,
        );
        pastes.receive(
            packet("id=pc-paste-1:status=DATA:mime=aW1hZ2UvcG5n", b"partial"),
            &queue,
            &mut out,
            now,
        );
        match ending {
            "timeout" => {
                assert!(pastes.expire(&queue, now + IDLE_TIMEOUT, true));
            }
            "suspend" => {
                assert!(pastes.expire(&queue, now, false));
            }
            "invalid" => pastes.receive(
                protocol::parse_packet(
                    b"5522;type=read:id=pc-paste-1:status=DATA:mime=aW1hZ2UvcG5n;not base64",
                )
                .unwrap(),
                &queue,
                &mut out,
                now,
            ),
            "wrong-mime" => pastes.receive(
                packet("id=pc-paste-1:status=DATA:mime=dGV4dC9wbGFpbg==", b"wrong"),
                &queue,
                &mut out,
                now,
            ),
            status => pastes.receive(
                packet(&format!("id=pc-paste-1:status={status}"), b""),
                &queue,
                &mut out,
                now,
            ),
        }
        // A late completion must not deliver a truncated image.
        pastes.receive(
            packet("id=pc-paste-1:status=DONE", b""),
            &queue,
            &mut out,
            now,
        );
        let events = queue.drain();
        assert_eq!(events.len(), 1, "{ending}");
        assert_eq!(events[0].paste.as_deref(), Some("after"));
        assert!(pastes.transfers.is_empty());
    }
}

#[test]
fn incomplete_mime_listing_expires_and_unsupported_types_do_not_read() {
    let mut pastes = test_pastes();
    let queue = NativeEventQueue::default();
    let mut out = Vec::new();
    let now = Instant::now();
    pastes.receive(packet("status=OK", b""), &queue, &mut out, now);
    queue.push(NativeEvent::paste("after".into()));
    assert!(queue.drain().is_empty());
    assert!(pastes.expire(&queue, now + IDLE_TIMEOUT, true));
    assert_eq!(queue.drain().len(), 1);
    listing(
        &mut pastes,
        &queue,
        &mut out,
        b"application/pdf text/html",
        now,
    );
    assert!(out.is_empty());
    assert!(queue.drain().is_empty());
    assert!(pastes.transfers.is_empty());
}

#[test]
fn text_paste_with_no_token_still_uses_only_the_advertised_mime_type() {
    let mut pastes = test_pastes();
    let queue = NativeEventQueue::default();
    let mut out = Vec::new();
    let now = Instant::now();
    pastes.receive(packet("status=OK", b""), &queue, &mut out, now);
    pastes.receive(
        packet("status=DATA:mime=Lg==", b"text/plain;charset=utf-8\n"),
        &queue,
        &mut out,
        now,
    );
    pastes.receive(packet("status=DONE", b""), &queue, &mut out, now);
    let request = String::from_utf8(out).unwrap();
    assert!(!request.contains("pw="));
    assert!(!request.contains("loc="));
    assert!(request.contains(&STANDARD.encode("text/plain;charset=utf-8")));
}

#[test]
fn base64_validation_is_strict_and_packet_size_is_bounded() {
    for data in ["YQ", "YQ=", "Y Q==", "YQ==\n", "YR=="] {
        let reply = protocol::parse_packet(
            format!("5522;type=read:status=DATA:mime=Lg==;{data}").as_bytes(),
        )
        .unwrap();
        assert!(reply.data.is_none(), "{data}");
    }
    assert!(packet("status=DATA:mime=Lg==", &vec![0; 4097])
        .data
        .is_none());
}

#[test]
fn replies_from_other_instances_cannot_complete_a_paste() {
    let mut pastes = test_pastes();
    let queue = NativeEventQueue::default();
    let now = Instant::now();
    let mut out = Vec::new();
    listing(&mut pastes, &queue, &mut out, b"image/png", now);
    queue.push(NativeEvent::paste("after".into()));
    for status in ["OK", "DATA:mime=aW1hZ2UvcG5n", "DONE"] {
        pastes.receive(
            packet(
                &format!("id=another-instance-1:status={status}"),
                b"unrelated",
            ),
            &queue,
            &mut out,
            now,
        );
    }
    assert!(queue.drain().is_empty());
    assert_eq!(pastes.transfers.len(), 1);
    assert!(pastes.expire(&queue, now + IDLE_TIMEOUT, true));
    let events = queue.drain();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].paste.as_deref(), Some("after"));
}

#[test]
fn excessive_image_data_is_cancelled_without_exposing_partial_contents() {
    let mut pastes = test_pastes();
    let queue = NativeEventQueue::default();
    let mut out = Vec::new();
    let now = Instant::now();
    listing(&mut pastes, &queue, &mut out, b"image/png", now);
    pastes.receive(
        packet("id=pc-paste-1:status=OK", b""),
        &queue,
        &mut out,
        now,
    );
    pastes
        .transfers
        .get_mut("pc-paste-1")
        .unwrap()
        .data
        .resize(MAX_BYTES, 0);
    pastes.receive(
        packet("id=pc-paste-1:status=DATA:mime=aW1hZ2UvcG5n", b"x"),
        &queue,
        &mut out,
        now,
    );
    assert!(pastes.transfers.is_empty());
    assert!(queue.drain().is_empty());
}
