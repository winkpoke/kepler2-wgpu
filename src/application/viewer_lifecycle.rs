//! Complete lifecycle ordering shared by Winit and deterministic recording adapters.
//! App retains graphics/data ownership; frame and redraw modules retain their policies.

use super::frame::FrameOutcome;
use super::redraw::{FrameOwner, RedrawAction};
use crate::core::error::KeplerResult;
use std::time::Duration;

pub(super) enum WindowSignal {
    Redraw,
    Occluded(bool),
    Resize(u32, u32),
    Close,
}

pub(super) enum LifecycleEvent<I, G, V> {
    Window { id: I, signal: WindowSignal },
    Resumed,
    Suspended,
    AboutToWait,
    GraphicsReady(G, V),
    Quit,
}

pub(super) struct PendingWork {
    pub surface_ready: bool,
    pub dirty: bool,
    pub animating: bool,
    pub recovery_in: Option<Duration>,
}

/// Primitive borrowed application/platform effects. Sequencing belongs to `handle`.
pub(super) trait LifecycleHost {
    type WindowId: PartialEq;
    type Graphics;
    type Volume;

    fn active_window_id(&self) -> Self::WindowId;
    fn surface_ready(&self) -> bool;
    fn pending_work(&self) -> PendingWork;
    fn invalidate_surface(&mut self);
    fn update(&mut self);
    fn render_frame(&mut self) -> KeplerResult<FrameOutcome>;
    fn swap_graphics(&mut self, graphics: Self::Graphics) -> KeplerResult<()>;
    fn attach_fault_transport(&mut self);
    fn resize(&mut self, width: u32, height: u32);
    fn enqueue_volume(&mut self, volume: Self::Volume);
    fn clear_views(&mut self);
    fn request_redraw(&mut self);
    fn wait(&mut self, delay: Option<Duration>);
    fn exit(&mut self);
}

#[derive(Default)]
pub(super) struct ViewerLifecycle {
    owner: FrameOwner,
    shutting_down: bool,
}

impl ViewerLifecycle {
    pub(super) fn shutting_down(&self) -> bool {
        self.shutting_down
    }

    /// Used before App::input/native controls as well as within lifecycle dispatch.
    pub(super) fn accepts_window<I: PartialEq>(&self, active: I, observed: I) -> bool {
        !self.shutting_down && FrameOwner::is_active_window(active, observed)
    }

    pub(super) fn handle<H: LifecycleHost>(
        &mut self,
        event: LifecycleEvent<H::WindowId, H::Graphics, H::Volume>,
        host: &mut H,
        now: impl Fn() -> Duration,
    ) {
        if self.shutting_down {
            return;
        }
        match event {
            LifecycleEvent::Window { id, signal } => {
                if !self.accepts_window(host.active_window_id(), id) {
                    return;
                }
                match signal {
                    WindowSignal::Redraw => {
                        self.owner.redraw.redraw_received();
                        if !self.owner.ready(host.surface_ready()) {
                            return;
                        }
                        // An OS redraw needs a complete frame even when state is clean.
                        host.update();
                        let outcome = host.render_frame();
                        self.owner.frame_outcome(&outcome, now());
                        if let Err(error) = outcome {
                            log::error!("Rendering stopped: {error}. Restart the viewer (fresh page in a browser).");
                        }
                    }
                    WindowSignal::Occluded(hidden) => {
                        self.owner.set_occluded(hidden);
                        if !hidden {
                            host.invalidate_surface();
                        }
                    }
                    WindowSignal::Resize(width, height) => host.resize(width, height),
                    WindowSignal::Close => self.shutdown(host, false),
                }
            }
            LifecycleEvent::Resumed => {
                self.owner.resumed();
                host.invalidate_surface();
            }
            LifecycleEvent::Suspended => self.owner.suspended(),
            LifecycleEvent::AboutToWait => {
                let work = host.pending_work();
                match self.owner.schedule(
                    now(),
                    work.surface_ready,
                    work.dirty,
                    work.animating,
                    work.recovery_in,
                ) {
                    RedrawAction::Request => {
                        host.request_redraw();
                        host.wait(None);
                    }
                    RedrawAction::Wait => host.wait(None),
                    RedrawAction::WaitFor(delay) => host.wait(Some(delay)),
                }
            }
            LifecycleEvent::GraphicsReady(graphics, volume) => {
                if let Err(error) = host.swap_graphics(graphics) {
                    log::error!("Failed to swap graphics: {error}");
                    return;
                }
                host.attach_fault_transport();
                // Discard outstanding redraws for the old window without replacing the owner.
                self.owner.set_occluded(false);
                host.resize(800, 800);
                host.enqueue_volume(volume);
                log::info!("Graphics swapped in state.");
            }
            LifecycleEvent::Quit => self.shutdown(host, true),
        }
    }

    fn shutdown(&mut self, host: &mut impl LifecycleHost, clear: bool) {
        if clear {
            host.clear_views();
        }
        self.owner.stop();
        self.shutting_down = true;
        host.exit();
    }
}

#[cfg(test)]
#[path = "viewer_lifecycle_tests.rs"]
mod tests;
