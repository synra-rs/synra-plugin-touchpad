//! Frame admission and contact transitions, with no clocks, IO or native input of their own.

use super::{
    Command, Contact, DOUBLE_TAP_MS, DOUBLE_TAP_PX, DRAG_HOLD_MS, MOTION_PX, Mode, Recognizer,
    Refusal, Step, TAP_MS,
};

impl Recognizer {
    /// Feed a complete snapshot. Contact order is irrelevant; ids must be stable.
    ///
    /// Refused frames produce cleanup before reporting the refusal and cannot cause a click.
    pub fn update(&mut self, now_ms: u64, contacts: &[Contact]) -> Step {
        if self.last_ms.is_some_and(|last| now_ms < last) {
            return self.refuse(Refusal::ClockReversed);
        }
        self.last_ms = Some(now_ms);
        if contacts.len() > 2
            || contacts.iter().any(|contact| {
                let p = contact.position;
                !p.x.is_finite()
                    || !p.y.is_finite()
                    || p.x.abs() > 100_000.0
                    || p.y.abs() > 100_000.0
            })
            || matches!(contacts, [a, b] if a.id == b.id)
        {
            return self.refuse(Refusal::InvalidContacts);
        }
        let mut step = Step::default();
        match (self.mode, contacts) {
            (Mode::Idle, [contact]) => {
                let second = self.last_tap.take().is_some_and(|(at, position)| {
                    now_ms - at <= DOUBLE_TAP_MS
                        && contact.position.distance_squared(position) <= DOUBLE_TAP_PX.powi(2)
                });
                self.mode = Mode::One {
                    id: contact.id,
                    start: contact.position,
                    last: contact.position,
                    since: now_ms,
                    moved: false,
                    second,
                };
            }
            (Mode::Idle, [a, b]) => self.begin_two(*a, *b, now_ms, false),
            (Mode::One { .. }, _) => return self.one(now_ms, contacts),
            (Mode::Two { .. }, _) => return self.two(now_ms, contacts),
            (
                Mode::FinishingTwo {
                    remaining,
                    since,
                    tap,
                },
                [contact],
            ) if remaining.id == contact.id => {
                self.mode = Mode::FinishingTwo {
                    remaining,
                    since,
                    tap: tap
                        && remaining.position.distance_squared(contact.position)
                            < MOTION_PX.powi(2),
                };
            }
            (Mode::FinishingTwo { since, tap, .. }, []) => {
                if tap && now_ms - since <= TAP_MS {
                    step.commands.push(Command::RightClick);
                }
                self.mode = Mode::Idle;
            }
            (Mode::FinishingTwo { .. }, _) => return self.refuse(Refusal::ContactReplaced),
            (Mode::Suppressed, []) => self.mode = Mode::Idle,
            (Mode::Suppressed | Mode::Idle, _) => {}
        }
        step
    }

    fn begin_two(&mut self, a: Contact, b: Contact, since: u64, moved: bool) {
        self.last_tap = None;
        let start = if a.id < b.id { [a, b] } else { [b, a] };
        self.mode = Mode::Two {
            start,
            last: a.position.midpoint(b.position),
            since,
            moved,
        };
    }

    fn one(&mut self, now: u64, contacts: &[Contact]) -> Step {
        let Mode::One {
            id,
            start,
            last,
            since,
            moved,
            second,
        } = self.mode
        else {
            return Step::default();
        };
        let mut step = Step::default();
        match contacts {
            [contact] if contact.id == id => {
                let current = contact.position;
                let moved = moved || start.distance_squared(current) >= MOTION_PX.powi(2);
                if second && !self.left_down && (moved || now - since >= DRAG_HOLD_MS) {
                    self.left_down = true;
                    step.commands.push(Command::LeftDown);
                }
                if moved && current != last {
                    step.commands.push(Command::Move {
                        dx: current.x - last.x,
                        dy: current.y - last.y,
                    });
                }
                self.mode = Mode::One {
                    id,
                    start,
                    last: if moved { current } else { last },
                    since,
                    moved,
                    second,
                };
            }
            [] => {
                if self.left_down {
                    self.release(&mut step);
                    self.last_tap = None;
                } else if !moved && now - since <= TAP_MS {
                    step.commands.push(Command::LeftClick);
                    self.last_tap = if second { None } else { Some((now, start)) };
                } else {
                    self.last_tap = None;
                }
                self.mode = Mode::Idle;
            }
            [a, b] if a.id == id || b.id == id => {
                if self.left_down {
                    self.release(&mut step);
                    self.mode = Mode::Suppressed;
                    self.last_tap = None;
                } else {
                    let existing = if a.id == id { a } else { b };
                    let moved =
                        moved || start.distance_squared(existing.position) >= MOTION_PX.powi(2);
                    self.begin_two(*a, *b, since, moved);
                }
            }
            _ => return self.refuse(Refusal::ContactReplaced),
        }
        step
    }

    fn two(&mut self, now: u64, contacts: &[Contact]) -> Step {
        let Mode::Two {
            start: [a, b],
            last,
            since,
            moved,
        } = self.mode
        else {
            return Step::default();
        };
        let mut step = Step::default();
        match contacts {
            [first, second]
                if (first.id == a.id && second.id == b.id)
                    || (first.id == b.id && second.id == a.id) =>
            {
                let (first, second) = if first.id == a.id {
                    (first, second)
                } else {
                    (second, first)
                };
                let current = first.position.midpoint(second.position);
                let moved = moved
                    || a.position.distance_squared(first.position) >= MOTION_PX.powi(2)
                    || b.position.distance_squared(second.position) >= MOTION_PX.powi(2);
                if moved && current != last {
                    step.commands.push(Command::Scroll {
                        dx: current.x - last.x,
                        dy: current.y - last.y,
                    });
                }
                self.mode = Mode::Two {
                    start: [a, b],
                    last: if moved { current } else { last },
                    since,
                    moved,
                };
            }
            [remaining] if remaining.id == a.id || remaining.id == b.id => {
                let original = if remaining.id == a.id { a } else { b };
                self.mode = Mode::FinishingTwo {
                    remaining: *remaining,
                    since,
                    tap: !moved
                        && original.position.distance_squared(remaining.position)
                            < MOTION_PX.powi(2),
                };
            }
            [] => {
                if !moved && now - since <= TAP_MS {
                    step.commands.push(Command::RightClick);
                }
                self.mode = Mode::Idle;
            }
            _ => return self.refuse(Refusal::ContactReplaced),
        }
        step
    }
}
