//! Redraw policy is independent of winit/GPU handles so retries and idle behavior
//! can be verified using a deterministic clock.

use std::time::Duration;

const FRAME_INTERVAL: Duration = Duration::from_millis(16);

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum RedrawAction {
    Wait,
    WaitFor(Duration),
    Request,
}

#[derive(Default)]
pub(crate) struct RedrawScheduler {
    requested: bool,
    last_frame: Option<Duration>,
    retry_at: Option<Duration>,
}

impl RedrawScheduler {
    pub fn redraw_received(&mut self) {
        self.requested = false;
    }

    pub fn frame_succeeded(&mut self, now: Duration) {
        self.last_frame = Some(now);
        self.retry_at = None;
    }

    pub fn frame_failed(&mut self, now: Duration) {
        self.retry_at = Some(now + FRAME_INTERVAL);
    }

    pub fn schedule(
        &mut self,
        now: Duration,
        ready: bool,
        dirty: bool,
        animating: bool,
        recovery_in: Option<Duration>,
    ) -> RedrawAction {
        if !ready {
            self.requested = false;
            return RedrawAction::Wait;
        }
        if self.requested {
            return RedrawAction::Wait;
        }
        let mut delay = if dirty || self.retry_at.is_some() {
            Some(Duration::ZERO)
        } else if animating {
            Some(self.last_frame.map_or(Duration::ZERO, |last| {
                (last + FRAME_INTERVAL).saturating_sub(now)
            }))
        } else {
            None
        };
        if let Some(recovery) = recovery_in {
            delay = Some(delay.map_or(recovery, |delay| delay.min(recovery)));
        }
        let Some(mut delay) = delay else {
            return RedrawAction::Wait;
        };
        if let Some(retry_at) = self.retry_at {
            delay = delay.max(retry_at.saturating_sub(now));
        }
        if delay.is_zero() {
            self.requested = true;
            RedrawAction::Request
        } else {
            RedrawAction::WaitFor(delay)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn initial_frame_then_static_idle_and_read_only_events_wait() {
        let mut scheduler = RedrawScheduler::default();
        assert_eq!(
            scheduler.schedule(ms(0), true, true, false, None),
            RedrawAction::Request
        );
        assert_eq!(
            scheduler.schedule(ms(0), true, true, false, None),
            RedrawAction::Wait
        );
        scheduler.redraw_received();
        scheduler.frame_succeeded(ms(0));
        for time in 1..1000 {
            assert_eq!(
                scheduler.schedule(ms(time), true, false, false, None),
                RedrawAction::Wait
            );
        }
        assert_eq!(
            scheduler.schedule(ms(1), true, true, false, None),
            RedrawAction::Request
        );
    }

    #[test]
    fn animation_is_bounded_and_stops_when_disabled() {
        let mut scheduler = RedrawScheduler::default();
        scheduler.frame_succeeded(ms(0));
        assert_eq!(
            scheduler.schedule(ms(1), true, false, true, None),
            RedrawAction::WaitFor(ms(15))
        );
        assert_eq!(
            scheduler.schedule(ms(16), true, false, true, None),
            RedrawAction::Request
        );
        scheduler.redraw_received();
        scheduler.frame_succeeded(ms(16));
        assert_eq!(
            scheduler.schedule(ms(17), true, false, false, None),
            RedrawAction::Wait
        );
    }

    #[test]
    fn failed_frames_retry_without_busy_looping_or_consuming_dirty_state() {
        let mut scheduler = RedrawScheduler::default();
        scheduler.frame_failed(ms(0));
        assert_eq!(
            scheduler.schedule(ms(1), true, true, false, None),
            RedrawAction::WaitFor(ms(15))
        );
        assert_eq!(
            scheduler.schedule(ms(16), true, true, false, None),
            RedrawAction::Request
        );
        scheduler.redraw_received();
        scheduler.frame_succeeded(ms(16));
        assert_eq!(
            scheduler.schedule(ms(17), true, false, false, None),
            RedrawAction::Wait
        );
    }

    #[test]
    fn minimized_and_occluded_windows_wait_but_restore_pending_work() {
        let mut scheduler = RedrawScheduler::default();
        assert_eq!(
            scheduler.schedule(ms(0), false, true, true, None),
            RedrawAction::Wait
        );
        assert_eq!(
            scheduler.schedule(ms(100), true, true, false, None),
            RedrawAction::Request
        );
        // A pending request might not be delivered while hidden. Restore must re-request.
        assert_eq!(
            scheduler.schedule(ms(101), false, true, false, None),
            RedrawAction::Wait
        );
        assert_eq!(
            scheduler.schedule(ms(102), true, true, false, None),
            RedrawAction::Request
        );
    }

    #[test]
    fn fallback_recovery_uses_a_deadline_not_idle_frames() {
        let mut scheduler = RedrawScheduler::default();
        assert_eq!(
            scheduler.schedule(ms(0), true, false, false, Some(ms(60_000))),
            RedrawAction::WaitFor(ms(60_000))
        );
        assert_eq!(
            scheduler.schedule(ms(60_000), true, false, false, Some(ms(0))),
            RedrawAction::Request
        );
    }
}
