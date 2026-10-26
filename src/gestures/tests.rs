use super::{Command, Contact, DRAG_HOLD_MS, Point, Recognizer, Refusal, TAP_MS};

fn at(id: u32, x: f64, y: f64) -> Contact {
    Contact {
        id,
        position: Point { x, y },
    }
}

fn dragging() -> Recognizer {
    let mut recognizer = Recognizer::default();
    recognizer.update(0, &[at(1, 0.0, 0.0)]);
    assert_eq!(recognizer.update(20, &[]).commands, [Command::LeftClick]);
    recognizer.update(80, &[at(2, 0.0, 0.0)]);
    assert_eq!(
        recognizer
            .update(80 + DRAG_HOLD_MS, &[at(2, 0.0, 0.0)])
            .commands,
        [Command::LeftDown]
    );
    recognizer
}

#[test]
fn single_motion_preserves_accumulated_relative_distance() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0)]);
    assert!(r.update(1, &[at(1, 3.0, 0.0)]).commands.is_empty());
    assert_eq!(
        r.update(2, &[at(1, 8.0, 0.0)]).commands,
        [Command::Move { dx: 8.0, dy: 0.0 }]
    );
    assert_eq!(
        r.update(3, &[at(1, 9.0, -2.0)]).commands,
        [Command::Move { dx: 1.0, dy: -2.0 }]
    );
    assert!(r.update(4, &[]).commands.is_empty());
}

#[test]
fn stationary_single_tap_clicks_once_on_release() {
    let mut r = Recognizer::default();
    assert!(r.update(0, &[at(1, 0.0, 0.0)]).commands.is_empty());
    r.update(10, &[at(1, 2.0, 1.0)]);
    assert_eq!(r.update(TAP_MS, &[]).commands, [Command::LeftClick]);
    assert!(r.update(TAP_MS + 1, &[]).commands.is_empty());
}

#[test]
fn long_single_hold_is_not_a_tap_or_drag() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0)]);
    assert!(r.update(TAP_MS + 1, &[at(1, 0.0, 0.0)]).commands.is_empty());
    assert!(r.update(TAP_MS + 2, &[]).commands.is_empty());
}

#[test]
fn two_finger_tap_with_sequential_lifts_never_left_clicks() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0)]);
    r.update(10, &[at(1, 0.0, 0.0), at(2, 20.0, 0.0)]);
    assert!(r.update(20, &[at(2, 20.0, 0.0)]).commands.is_empty());
    assert_eq!(r.update(30, &[]).commands, [Command::RightClick]);
}

#[test]
fn two_finger_tap_with_simultaneous_lift_clicks_once() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0), at(2, 20.0, 0.0)]);
    assert_eq!(r.update(30, &[]).commands, [Command::RightClick]);
    assert!(r.update(31, &[]).commands.is_empty());
}

#[test]
fn scrolling_tracks_centroid_and_stable_ids_across_reordering() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0), at(2, 20.0, 0.0)]);
    assert_eq!(
        r.update(10, &[at(2, 20.0, 8.0), at(1, 0.0, 8.0)]).commands,
        [Command::Scroll { dx: 0.0, dy: 8.0 }]
    );
    assert_eq!(
        r.update(20, &[at(1, 1.0, 10.0), at(2, 21.0, 10.0)])
            .commands,
        [Command::Scroll { dx: 1.0, dy: 2.0 }]
    );
    r.update(30, &[at(2, 21.0, 10.0)]);
    assert!(r.update(40, &[]).commands.is_empty());
}

#[test]
fn opposing_movement_is_not_a_right_tap() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0), at(2, 20.0, 0.0)]);
    assert!(
        r.update(10, &[at(1, -8.0, 0.0), at(2, 28.0, 0.0)])
            .commands
            .is_empty()
    );
    assert!(r.update(20, &[]).commands.is_empty());
}

#[test]
fn moving_remaining_finger_cancels_two_finger_tap() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0), at(2, 20.0, 0.0)]);
    r.update(10, &[at(2, 20.0, 0.0)]);
    r.update(20, &[at(2, 28.0, 0.0)]);
    assert!(r.update(30, &[]).commands.is_empty());
}

#[test]
fn movement_in_the_frame_adding_a_second_finger_is_not_a_tap() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0)]);
    r.update(10, &[at(1, 8.0, 0.0), at(2, 20.0, 0.0)]);
    assert!(r.update(20, &[]).commands.is_empty());
}

#[test]
fn double_tap_hold_drags_and_releases_once() {
    let mut r = dragging();
    assert!(r.update(201, &[at(2, 0.0, 0.0)]).commands.is_empty());
    assert_eq!(
        r.update(210, &[at(2, 10.0, 0.0)]).commands,
        [Command::Move { dx: 10.0, dy: 0.0 }]
    );
    assert_eq!(r.update(220, &[]).commands, [Command::LeftUp]);
    assert!(r.update(221, &[]).commands.is_empty());
}

#[test]
fn moving_second_tap_presses_before_moving() {
    let mut r = Recognizer::default();
    r.update(0, &[at(1, 0.0, 0.0)]);
    r.update(20, &[]);
    r.update(30, &[at(2, 0.0, 0.0)]);
    assert_eq!(
        r.update(40, &[at(2, 8.0, 0.0)]).commands,
        [Command::LeftDown, Command::Move { dx: 8.0, dy: 0.0 }]
    );
    assert_eq!(r.update(50, &[]).commands, [Command::LeftUp]);
}

#[test]
fn distant_or_expired_second_tap_cannot_start_drag() {
    for (when, position) in [(321, 0.0), (30, 25.0)] {
        let mut r = Recognizer::default();
        r.update(0, &[at(1, 0.0, 0.0)]);
        r.update(20, &[]);
        r.update(when, &[at(2, position, 0.0)]);
        assert!(
            r.update(when + DRAG_HOLD_MS, &[at(2, position, 0.0)])
                .commands
                .is_empty()
        );
    }
}

#[test]
fn cancellation_is_idempotent_and_clears_double_tap_memory() {
    let mut r = dragging();
    assert_eq!(r.cancel().commands, [Command::LeftUp]);
    assert!(r.cancel().commands.is_empty());
    r.update(210, &[at(3, 0.0, 0.0)]);
    assert!(
        r.update(210 + DRAG_HOLD_MS, &[at(3, 0.0, 0.0)])
            .commands
            .is_empty()
    );
}

#[test]
fn second_finger_during_drag_releases_then_suppresses_until_all_lift() {
    let mut r = dragging();
    assert_eq!(
        r.update(210, &[at(2, 0.0, 0.0), at(3, 20.0, 0.0)]).commands,
        [Command::LeftUp]
    );
    assert!(r.update(220, &[at(3, 20.0, 30.0)]).commands.is_empty());
    assert!(r.update(230, &[]).commands.is_empty());
}

#[test]
fn malformed_contact_sets_release_buttons_before_refusal() {
    for contacts in [
        vec![at(2, f64::NAN, 0.0)],
        vec![at(2, 0.0, f64::INFINITY)],
        vec![at(2, 100_001.0, 0.0)],
        vec![at(2, 0.0, 0.0), at(2, 1.0, 0.0)],
        vec![at(2, 0.0, 0.0), at(3, 1.0, 0.0), at(4, 2.0, 0.0)],
    ] {
        let mut r = dragging();
        let step = r.update(210, &contacts);
        assert_eq!(step.refusal, Some(Refusal::InvalidContacts));
        assert_eq!(step.commands, [Command::LeftUp]);
        assert!(r.update(220, &[at(2, 0.0, 0.0)]).commands.is_empty());
        assert!(r.update(230, &[]).commands.is_empty());
    }
}

#[test]
fn clock_reversal_and_replaced_contact_release_drag() {
    for (now, contact, refusal) in [
        (199, at(2, 0.0, 0.0), Refusal::ClockReversed),
        (210, at(99, 0.0, 0.0), Refusal::ContactReplaced),
    ] {
        let mut r = dragging();
        let step = r.update(now, &[contact]);
        assert_eq!(step.refusal, Some(refusal));
        assert_eq!(step.commands, [Command::LeftUp]);
        assert!(r.update(220, &[]).commands.is_empty());
    }
}

#[test]
fn a_new_session_has_no_held_button_or_double_tap_history() {
    let mut r = Recognizer::default();
    assert!(r.cancel().commands.is_empty());
    r.update(0, &[at(1, 0.0, 0.0)]);
    assert!(
        r.update(DRAG_HOLD_MS, &[at(1, 0.0, 0.0)])
            .commands
            .is_empty()
    );
}

#[test]
fn short_contact_sequences_never_duplicate_or_leak_button_holds() {
    let frames = [
        vec![],
        vec![at(1, 0.0, 0.0)],
        vec![at(1, 10.0, 0.0)],
        vec![at(2, 0.0, 0.0)],
        vec![at(1, 0.0, 0.0), at(2, 20.0, 0.0)],
        vec![at(1, 0.0, 10.0), at(2, 20.0, 10.0)],
        vec![at(1, f64::NAN, 0.0)],
        vec![at(1, 0.0, 0.0), at(1, 20.0, 0.0)],
    ];
    for sequence in 0..frames.len().pow(5) {
        let mut choices = sequence;
        let mut r = Recognizer::default();
        let mut held = false;
        for frame in 0..5 {
            let step = r.update(frame * 50, &frames[choices % frames.len()]);
            choices /= frames.len();
            assert!(step.commands.len() <= 3);
            for command in step.commands {
                match command {
                    Command::LeftDown => {
                        assert!(!held, "duplicate down in sequence {sequence}");
                        held = true;
                    }
                    Command::LeftUp => {
                        assert!(held, "unmatched release in sequence {sequence}");
                        held = false;
                    }
                    Command::LeftClick | Command::RightClick => assert!(!held),
                    Command::Move { .. } | Command::Scroll { .. } => {}
                }
            }
        }
        let cleanup = r.cancel();
        assert_eq!(
            cleanup.commands,
            if held { vec![Command::LeftUp] } else { vec![] }
        );
        assert!(r.cancel().commands.is_empty());
    }
}
