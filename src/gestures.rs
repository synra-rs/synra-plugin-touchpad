//! Pure gesture recognition. Commands describe intent, not successful native input.
//!
//! Feed complete contact snapshots, including stationary frames while holding a second tap.
//! Coordinates and timestamps use one controller's CSS pixels and monotonic milliseconds.
//! Dispatch every returned command in order, including cleanup commands on refused frames.

mod recognize;

#[cfg(test)]
mod tests;

/// Maximum duration of a tap.
pub const TAP_MS: u64 = 250;
/// Maximum delay from the first release to the second contact of a double tap.
pub const DOUBLE_TAP_MS: u64 = 300;
/// Stationary hold on a second tap before starting a drag.
pub const DRAG_HOLD_MS: u64 = 120;
/// Movement needed to distinguish motion from tap jitter, in CSS pixels.
pub const MOTION_PX: f64 = 8.0;
/// Maximum separation of the two taps starting a drag, in CSS pixels.
pub const DOUBLE_TAP_PX: f64 = 24.0;

/// A point in the touch surface's coordinate system.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    /// Horizontal position in CSS pixels.
    pub x: f64,
    /// Vertical position in CSS pixels.
    pub y: f64,
}

impl Point {
    fn distance_squared(self, other: Self) -> f64 {
        (self.x - other.x).powi(2) + (self.y - other.y).powi(2)
    }

    fn midpoint(self, other: Self) -> Self {
        Self {
            x: (self.x + other.x) / 2.0,
            y: (self.y + other.y) / 2.0,
        }
    }
}

/// One contact. The controller must preserve its id until release.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact {
    /// Pointer identifier local to this touch surface.
    pub id: u32,
    /// Current position.
    pub position: Point,
}

/// An ordered intention for the future authenticated input session.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Command {
    /// Relative cursor displacement in CSS pixels, before host sensitivity scaling.
    Move { dx: f64, dy: f64 },
    /// Relative two-finger centroid displacement; scroll direction is applied by the host.
    Scroll { dx: f64, dy: f64 },
    /// A complete left-button click.
    LeftClick,
    /// A complete right-button click.
    RightClick,
    /// Begin holding the left button. Never repeated during the same drag.
    LeftDown,
    /// Release a held left button.
    LeftUp,
}

/// Why a frame was refused and the current gesture was cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// A clock value preceded the previous frame.
    ClockReversed,
    /// More than two contacts, duplicate ids, non-finite or out-of-range coordinates.
    InvalidContacts,
    /// A contact id changed without an intervening release.
    ContactReplaced,
}

/// Commands and optional refusal for one frame. Cleanup commands must still be dispatched.
#[derive(Debug, Default, PartialEq)]
pub struct Step {
    /// Ordered commands; at most three per frame.
    pub commands: Vec<Command>,
    /// A refusal ends the gesture and suppresses input until all contacts lift.
    pub refusal: Option<Refusal>,
}

#[derive(Debug, Clone, Copy, Default)]
enum Mode {
    #[default]
    Idle,
    One {
        id: u32,
        start: Point,
        last: Point,
        since: u64,
        moved: bool,
        second: bool,
    },
    Two {
        start: [Contact; 2],
        last: Point,
        since: u64,
        moved: bool,
    },
    FinishingTwo {
        remaining: Contact,
        since: u64,
        tap: bool,
    },
    Suppressed,
}

/// Runtime-only recognizer. Recreate it for a new controller session; never persist button state.
#[derive(Debug, Default)]
pub struct Recognizer {
    mode: Mode,
    last_ms: Option<u64>,
    last_tap: Option<(u64, Point)>,
    left_down: bool,
}

impl Recognizer {
    /// Cancel after backgrounding, target change, disconnection, revocation or pointer cancellation.
    ///
    /// This produces a release intention. The receiving host also needs an independent lease failsafe.
    pub fn cancel(&mut self) -> Step {
        self.mode = Mode::Idle;
        self.last_tap = None;
        let mut step = Step::default();
        self.release(&mut step);
        step
    }

    fn release(&mut self, step: &mut Step) {
        if self.left_down {
            self.left_down = false;
            step.commands.push(Command::LeftUp);
        }
    }

    fn refuse(&mut self, refusal: Refusal) -> Step {
        let mut step = self.cancel();
        self.mode = Mode::Suppressed;
        step.refusal = Some(refusal);
        step
    }
}
