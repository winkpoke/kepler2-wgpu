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

/// Lifecycle and frame-result policy shared by the real window owner and deterministic tests.
#[derive(Default)]
pub(crate) struct FrameOwner {
    pub(crate) redraw: RedrawScheduler,
    occluded: bool,
    suspended: bool,
    stopped: bool,
}

impl FrameOwner {
    pub(crate) fn is_active_window<T: PartialEq>(active: T, observed: T) -> bool {
        active == observed
    }
    pub(crate) fn ready(&self, surface_ready: bool) -> bool {
        surface_ready && !self.occluded && !self.suspended && !self.stopped
    }
    pub(crate) fn resumed(&mut self) {
        self.suspended = false;
        self.occluded = false;
        self.redraw.redraw_received();
    }
    pub(crate) fn suspended(&mut self) {
        self.suspended = true;
        self.redraw.redraw_received();
    }
    pub(crate) fn set_occluded(&mut self, hidden: bool) {
        self.occluded = hidden;
        self.redraw.redraw_received();
    }
    pub(crate) fn stop(&mut self) {
        self.stopped = true;
        self.redraw.redraw_received();
    }
    pub(crate) fn frame_outcome(
        &mut self,
        outcome: &crate::core::error::KeplerResult<super::frame::FrameOutcome>,
        now: Duration,
    ) {
        use super::frame::FrameOutcome;
        match outcome {
            Ok(FrameOutcome::Presented {
                reconfigure_pending,
            }) => {
                self.redraw.frame_succeeded(now);
                if *reconfigure_pending {
                    self.redraw.frame_failed(now);
                }
            }
            // Unknown OS occlusion is probed at the bounded retry cadence; known
            // hide/suspend signals still suppress all acquisitions via ready().
            Ok(FrameOutcome::Skipped { .. }) => self.redraw.frame_failed(now),
            Ok(FrameOutcome::Unavailable) => {}
            Err(_) => self.stop(),
        }
    }
    pub(crate) fn schedule(
        &mut self,
        now: Duration,
        surface_ready: bool,
        dirty: bool,
        animating: bool,
        recovery_in: Option<Duration>,
    ) -> RedrawAction {
        self.redraw.schedule(
            now,
            self.ready(surface_ready),
            dirty,
            animating,
            recovery_in,
        )
    }
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
    fn replacement_routes_only_the_active_window_and_retains_lifecycle_owner() {
        let mut owner = FrameOwner::default();
        let mut active = 11u64;
        assert!(FrameOwner::is_active_window(active, 11));
        owner.suspended();
        active = 22;
        assert!(!FrameOwner::is_active_window(active, 11));
        assert!(FrameOwner::is_active_window(active, 22));
        assert!(!owner.ready(true));
        owner.resumed();
        assert!(owner.ready(true));
    }

    #[test]
    fn owner_resume_suspend_shutdown_and_frame_outcomes_use_bounded_policy() {
        use crate::application::frame::{FrameOutcome, RecoveryAction};
        use crate::core::error::{SurfaceFailure, SurfaceFailureKind};
        let mut owner = FrameOwner::default();
        owner.resumed();
        owner.resumed();
        assert!(owner.ready(true));
        owner.suspended();
        assert!(!owner.ready(true));
        owner.resumed();
        assert!(!owner.ready(false));
        for kind in [SurfaceFailureKind::Timeout, SurfaceFailureKind::Occluded] {
            owner.frame_outcome(
                &Ok(FrameOutcome::Skipped {
                    recovery_action: if kind == SurfaceFailureKind::Occluded {
                        RecoveryAction::Probe
                    } else {
                        RecoveryAction::Retry
                    },
                    failure: SurfaceFailure::new(kind, "acquire", "test"),
                }),
                ms(0),
            );
            assert_eq!(
                owner.schedule(ms(1), true, true, false, None),
                RedrawAction::WaitFor(ms(15))
            );
            assert_eq!(
                owner.schedule(ms(16), true, true, false, None),
                RedrawAction::Request
            );
            owner.redraw.redraw_received();
        }
        owner.set_occluded(true);
        assert_eq!(
            owner.schedule(ms(17), true, true, true, None),
            RedrawAction::Wait
        );
        owner.set_occluded(false);
        assert!(owner.ready(true));
        owner.stop();
        owner.resumed();
        assert_eq!(
            owner.schedule(ms(100), true, true, true, None),
            RedrawAction::Wait
        );
    }

    #[test]
    fn fatal_draw_or_validation_error_stops_retries_and_suboptimal_requests_reconfigure() {
        use crate::application::frame::FrameOutcome;
        use crate::core::error::KeplerError;
        let mut owner = FrameOwner::default();
        owner.frame_outcome(
            &Ok(FrameOutcome::Presented {
                reconfigure_pending: true,
            }),
            ms(0),
        );
        assert_eq!(
            owner.schedule(ms(1), true, true, false, None),
            RedrawAction::WaitFor(ms(15))
        );
        owner.frame_outcome(
            &Err(KeplerError::Graphics("injected pass failure".into())),
            ms(2),
        );
        assert_eq!(
            owner.schedule(ms(100), true, true, true, Some(ms(0))),
            RedrawAction::Wait
        );
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
