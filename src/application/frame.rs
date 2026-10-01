//! Internal complete-frame transaction; public callers retain unit success.

use crate::core::error::{KeplerError, KeplerResult, SurfaceFailure, SurfaceFailureKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RecoveryAction {
    Retry,
    Probe,
    Reconfigure,
    Recreate,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrameOutcome {
    Presented {
        reconfigure_pending: bool,
    },
    Skipped {
        recovery_action: RecoveryAction,
        failure: SurfaceFailure,
    },
    Unavailable,
}

impl FrameOutcome {
    pub(crate) fn into_public_result(self) -> KeplerResult<()> {
        match self {
            Self::Presented { .. } | Self::Unavailable => Ok(()),
            Self::Skipped { failure, .. } => Err(failure.into()),
        }
    }
}

pub(crate) enum Acquisition<T> {
    Usable { texture: T, suboptimal: bool },
    Failed(SurfaceFailureKind),
}

impl Acquisition<wgpu::SurfaceTexture> {
    pub(crate) fn from_surface(value: wgpu::CurrentSurfaceTexture) -> Self {
        use wgpu::CurrentSurfaceTexture as Current;
        match value {
            Current::Success(texture) => Self::Usable {
                texture,
                suboptimal: false,
            },
            Current::Suboptimal(texture) => Self::Usable {
                texture,
                suboptimal: true,
            },
            Current::Timeout => Self::Failed(SurfaceFailureKind::Timeout),
            Current::Occluded => Self::Failed(SurfaceFailureKind::Occluded),
            Current::Outdated => Self::Failed(SurfaceFailureKind::Outdated),
            Current::Lost => Self::Failed(SurfaceFailureKind::Lost),
            Current::Validation => Self::Failed(SurfaceFailureKind::Validation),
        }
    }
}

pub(crate) trait FrameDriver {
    type Texture;
    fn check_fault(&self) -> KeplerResult<()> {
        Ok(())
    }
    fn ready(&self) -> bool;
    fn reconfigure_pending(&self) -> bool;
    fn set_reconfigure_pending(&mut self, pending: bool);
    fn recover(&mut self, action: RecoveryAction) -> KeplerResult<()>;
    fn acquire(&mut self) -> Acquisition<Self::Texture>;
    fn draw_and_present(&mut self, texture: Self::Texture) -> KeplerResult<()>;
    fn mark_presented(&mut self);
    fn acquisition_failure(&self, kind: SurfaceFailureKind) -> SurfaceFailure;
}

pub(crate) fn pass_failure(error: impl std::fmt::Display) -> KeplerError {
    KeplerError::Graphics(format!("execute complete frame passes: {error}"))
}

/// The same transaction is used by App and injected regression drivers.
pub(crate) fn execute(driver: &mut impl FrameDriver) -> KeplerResult<FrameOutcome> {
    driver.check_fault()?;
    if !driver.ready() {
        return Ok(FrameOutcome::Unavailable);
    }
    if driver.reconfigure_pending() {
        driver.recover(RecoveryAction::Reconfigure)?;
        driver.set_reconfigure_pending(false);
    }
    let acquisition = driver.acquire();
    driver.check_fault()?;
    match acquisition {
        Acquisition::Usable {
            texture,
            suboptimal,
        } => {
            driver.draw_and_present(texture)?;
            driver.mark_presented();
            driver.set_reconfigure_pending(suboptimal);
            Ok(FrameOutcome::Presented {
                reconfigure_pending: suboptimal,
            })
        }
        Acquisition::Failed(kind) => {
            let action = match kind {
                SurfaceFailureKind::Timeout => RecoveryAction::Retry,
                SurfaceFailureKind::Occluded => RecoveryAction::Probe,
                SurfaceFailureKind::Outdated => RecoveryAction::Reconfigure,
                SurfaceFailureKind::Lost => RecoveryAction::Recreate,
                _ => return Err(driver.acquisition_failure(kind).into()),
            };
            if matches!(
                action,
                RecoveryAction::Reconfigure | RecoveryAction::Recreate
            ) {
                driver.recover(action)?;
            }
            Ok(FrameOutcome::Skipped {
                recovery_action: action,
                failure: driver.acquisition_failure(kind),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::{KeplerError, SurfaceFailureKind};

    struct Fake {
        acquisition: Option<Acquisition<()>>,
        pending: bool,
        dirty: bool,
        ready: bool,
        fail_draw: bool,
        fail_recovery: bool,
        draws: usize,
        presentations: usize,
        configurations: usize,
        recreations: usize,
    }

    impl Fake {
        fn new(acquisition: Acquisition<()>) -> Self {
            Self {
                acquisition: Some(acquisition),
                pending: false,
                dirty: true,
                ready: true,
                fail_draw: false,
                fail_recovery: false,
                draws: 0,
                presentations: 0,
                configurations: 0,
                recreations: 0,
            }
        }
    }

    impl FrameDriver for Fake {
        type Texture = ();
        fn ready(&self) -> bool {
            self.ready
        }
        fn reconfigure_pending(&self) -> bool {
            self.pending
        }
        fn set_reconfigure_pending(&mut self, pending: bool) {
            self.pending = pending;
        }
        fn recover(&mut self, action: RecoveryAction) -> KeplerResult<()> {
            if self.fail_recovery {
                return Err(KeplerError::Graphics(
                    "configure selected backend failed".into(),
                ));
            }
            match action {
                RecoveryAction::Reconfigure => self.configurations += 1,
                RecoveryAction::Recreate => self.recreations += 1,
                _ => {}
            }
            Ok(())
        }
        fn acquire(&mut self) -> Acquisition<()> {
            self.acquisition.take().unwrap()
        }
        fn draw_and_present(&mut self, _: ()) -> KeplerResult<()> {
            self.draws += 1;
            if self.fail_draw {
                return Err(pass_failure("injected slice pass failure"));
            }
            self.presentations += 1;
            Ok(())
        }
        fn mark_presented(&mut self) {
            self.dirty = false;
        }
        fn acquisition_failure(&self, kind: SurfaceFailureKind) -> SurfaceFailure {
            SurfaceFailure::new(
                kind,
                "acquire current frame (test backend)",
                "no usable frame",
            )
        }
    }

    #[test]
    fn success_presents_once_and_public_success_is_unit() {
        let mut driver = Fake::new(Acquisition::Usable {
            texture: (),
            suboptimal: false,
        });
        let outcome = execute(&mut driver).unwrap();
        assert_eq!(
            outcome,
            FrameOutcome::Presented {
                reconfigure_pending: false
            }
        );
        let _: () = outcome.into_public_result().unwrap();
        assert_eq!((driver.draws, driver.presentations), (1, 1));
        assert!(!driver.dirty);
    }

    #[test]
    fn suboptimal_reconfiguration_survives_public_wrapper_and_later_mutations() {
        let mut driver = Fake::new(Acquisition::Usable {
            texture: (),
            suboptimal: true,
        });
        execute(&mut driver).unwrap().into_public_result().unwrap();
        assert!(driver.pending);
        driver.dirty = true;
        driver.acquisition = Some(Acquisition::Failed(SurfaceFailureKind::Timeout));
        execute(&mut driver).unwrap();
        assert_eq!(driver.configurations, 1);
        assert!(driver.dirty);
        assert!(!driver.pending);
    }

    #[test]
    fn failed_draw_does_not_present_or_consume_pending_work_or_recreate_surface() {
        let mut driver = Fake::new(Acquisition::Usable {
            texture: (),
            suboptimal: false,
        });
        driver.fail_draw = true;
        let error = execute(&mut driver).unwrap_err();
        assert!(matches!(error, KeplerError::Graphics(_)));
        assert!(error.to_string().contains("slice pass failure"));
        assert!(driver.dirty);
        assert_eq!((driver.presentations, driver.recreations), (0, 0));
    }

    #[test]
    fn skipped_acquisitions_keep_dirty_work_and_owned_classification() {
        for (kind, action, configurations, recreations) in [
            (SurfaceFailureKind::Timeout, RecoveryAction::Retry, 0, 0),
            (SurfaceFailureKind::Occluded, RecoveryAction::Probe, 0, 0),
            (
                SurfaceFailureKind::Outdated,
                RecoveryAction::Reconfigure,
                1,
                0,
            ),
            (SurfaceFailureKind::Lost, RecoveryAction::Recreate, 0, 1),
        ] {
            let mut driver = Fake::new(Acquisition::Failed(kind));
            let outcome = execute(&mut driver).unwrap();
            assert!(
                matches!(&outcome, FrameOutcome::Skipped { recovery_action, .. } if *recovery_action == action)
            );
            assert!(
                matches!(outcome.into_public_result(), Err(KeplerError::Surface(f)) if f.kind == kind)
            );
            assert!(driver.dirty);
            assert_eq!((driver.draws, driver.presentations), (0, 0));
            assert_eq!(
                (driver.configurations, driver.recreations),
                (configurations, recreations)
            );
        }
    }

    #[test]
    fn zero_size_is_successful_noop_without_acquisition_or_recovery() {
        let mut driver = Fake::new(Acquisition::Failed(SurfaceFailureKind::Lost));
        driver.ready = false;
        driver.pending = true;
        execute(&mut driver).unwrap().into_public_result().unwrap();
        assert!(driver.acquisition.is_some());
        assert!(driver.pending && driver.dirty);
        assert_eq!((driver.configurations, driver.recreations), (0, 0));
    }

    #[test]
    fn validation_and_recovery_failure_are_fatal_not_surface_retry_success() {
        let mut driver = Fake::new(Acquisition::Failed(SurfaceFailureKind::Validation));
        assert!(
            matches!(execute(&mut driver), Err(KeplerError::Surface(f)) if f.kind == SurfaceFailureKind::Validation)
        );
        assert_eq!(driver.recreations, 0);
        driver.acquisition = Some(Acquisition::Failed(SurfaceFailureKind::Lost));
        driver.fail_recovery = true;
        assert!(execute(&mut driver)
            .unwrap_err()
            .to_string()
            .contains("selected backend"));
        assert!(driver.dirty);
    }
}
