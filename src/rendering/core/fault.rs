//! Private device-fault notification; the existing quit transport wakes an idle owner.

use crate::core::error::{KeplerResult, SurfaceFailure};
use std::sync::{Arc, Mutex};

#[cfg(not(target_arch = "wasm32"))]
type Wake = Arc<dyn Fn() + Send + Sync>;

#[derive(Default)]
struct FaultState {
    failure: Option<SurfaceFailure>,
    #[cfg(not(target_arch = "wasm32"))]
    wake: Option<Wake>,
}

#[derive(Clone)]
pub(crate) struct FaultNotifier {
    state: Arc<Mutex<FaultState>>,
    #[cfg(target_arch = "wasm32")]
    id: usize,
}

// Browser GPU callbacks and Winit dispatch run on the browser thread. Keeping
// its non-Send proxy here avoids unsafe Send wrappers in wgpu's Send callback.
#[cfg(target_arch = "wasm32")]
thread_local! {
    static WEB_WAKES: std::cell::RefCell<std::collections::HashMap<usize, std::rc::Rc<dyn Fn()>>> = std::cell::RefCell::new(std::collections::HashMap::new());
}

impl Default for FaultNotifier {
    fn default() -> Self {
        #[cfg(target_arch = "wasm32")]
        static NEXT_ID: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(1);
        Self {
            state: Arc::new(Mutex::new(FaultState::default())),
            #[cfg(target_arch = "wasm32")]
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        }
    }
}

impl std::fmt::Debug for FaultNotifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FaultNotifier")
            .field("failure", &self.state.lock().unwrap().failure)
            .finish()
    }
}

impl FaultNotifier {
    pub(crate) fn report(&self, failure: SurfaceFailure) {
        log::error!(
            "GPU {:?} during {}: {}. Restart the viewer (fresh page in a browser).",
            failure.kind,
            failure.operation,
            failure.message
        );
        let first = {
            let mut state = self.state.lock().unwrap();
            if state.failure.is_some() {
                false
            } else {
                state.failure = Some(failure);
                true
            }
        };
        if first {
            self.wake();
        }
    }
    pub(crate) fn check(&self) -> KeplerResult<()> {
        match self.state.lock().unwrap().failure.clone() {
            Some(failure) => Err(failure.into()),
            None => Ok(()),
        }
    }
    fn wake(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        let wake = self.state.lock().unwrap().wake.clone();
        #[cfg(target_arch = "wasm32")]
        let wake = WEB_WAKES.with(|wakes| wakes.borrow().get(&self.id).cloned());
        // Never invoke the transport while holding a notifier/registry lock.
        if let Some(wake) = wake {
            wake();
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn set_wake(&self, wake: impl Fn() + Send + Sync + 'static) {
        self.state.lock().unwrap().wake = Some(Arc::new(wake));
        if self.check().is_err() {
            self.wake();
        }
    }
    #[cfg(target_arch = "wasm32")]
    pub(crate) fn set_wake(&self, wake: impl Fn() + 'static) {
        WEB_WAKES.with(|wakes| {
            wakes.borrow_mut().insert(self.id, std::rc::Rc::new(wake));
        });
        if self.check().is_err() {
            self.wake();
        }
    }
    pub(crate) fn clear_wake(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state.lock().unwrap().wake = None;
        }
        #[cfg(target_arch = "wasm32")]
        WEB_WAKES.with(|wakes| {
            wakes.borrow_mut().remove(&self.id);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::error::{KeplerError, SurfaceFailure, SurfaceFailureKind};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn idle_fault_is_typed_persistent_and_wakes_once_without_a_render_tick() {
        for kind in [
            SurfaceFailureKind::OutOfMemory,
            SurfaceFailureKind::DeviceLost,
            SurfaceFailureKind::Validation,
        ] {
            let notifier = FaultNotifier::default();
            let wakes = Arc::new(AtomicUsize::new(0));
            let count = wakes.clone();
            notifier.set_wake(move || {
                count.fetch_add(1, Ordering::SeqCst);
            });
            notifier.report(SurfaceFailure::new(
                kind,
                "GPU callback",
                "injected Metal fault",
            ));
            notifier.report(SurfaceFailure::new(kind, "GPU callback", "duplicate"));
            assert_eq!(wakes.load(Ordering::SeqCst), 1);
            assert!(matches!(notifier.check(), Err(KeplerError::Surface(f)) if f.kind == kind));
            assert!(notifier.check().is_err());
        }
    }

    #[test]
    fn pre_run_fault_is_delivered_when_proxy_is_attached_and_detach_stops_wakes() {
        let notifier = FaultNotifier::default();
        notifier.report(SurfaceFailure::new(
            SurfaceFailureKind::OutOfMemory,
            "startup",
            "injected",
        ));
        let wakes = Arc::new(AtomicUsize::new(0));
        let count = wakes.clone();
        notifier.set_wake(move || {
            count.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(wakes.load(Ordering::SeqCst), 1);
        notifier.clear_wake();
        assert!(notifier.check().is_err());
    }
}
