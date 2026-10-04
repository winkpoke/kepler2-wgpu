use super::*;
use crate::core::error::{KeplerError, SurfaceFailure, SurfaceFailureKind};
use std::cell::Cell;
use std::collections::VecDeque;
use std::rc::Rc;

#[derive(Debug, PartialEq, Eq)]
enum Effect {
    Invalidate,
    Update,
    Render,
    Swap(u64),
    Attach,
    Resize(u32, u32),
    Enqueue(u8),
    Clear,
    Request,
    Wait(Option<Duration>),
    Exit,
}

struct RecordingHost {
    effects: Vec<Effect>,
    active: u64,
    ready: bool,
    dirty: bool,
    animating: bool,
    recovery_in: Option<Duration>,
    fail_swap: bool,
    outcomes: VecDeque<KeplerResult<FrameOutcome>>,
    clock: Rc<Cell<Duration>>,
    interaction: i32,
}

impl Default for RecordingHost {
    fn default() -> Self {
        Self {
            effects: vec![],
            active: 11,
            ready: true,
            dirty: true,
            animating: false,
            recovery_in: None,
            fail_swap: false,
            outcomes: VecDeque::new(),
            clock: Rc::new(Cell::new(Duration::ZERO)),
            interaction: 37,
        }
    }
}

impl LifecycleHost for RecordingHost {
    type WindowId = u64;
    type Graphics = u64;
    type Volume = u8;

    fn active_window_id(&self) -> u64 {
        self.active
    }
    fn surface_ready(&self) -> bool {
        self.ready
    }
    fn pending_work(&self) -> PendingWork {
        PendingWork {
            surface_ready: self.ready,
            dirty: self.dirty,
            animating: self.animating,
            recovery_in: self.recovery_in,
        }
    }
    fn invalidate_surface(&mut self) {
        self.effects.push(Effect::Invalidate);
        self.dirty = true;
    }
    fn update(&mut self) {
        self.effects.push(Effect::Update);
    }
    fn render_frame(&mut self) -> KeplerResult<FrameOutcome> {
        self.effects.push(Effect::Render);
        self.clock.set(self.clock.get() + ms(7));
        let outcome = self
            .outcomes
            .pop_front()
            .unwrap_or(Ok(FrameOutcome::Presented {
                reconfigure_pending: false,
            }));
        if let Ok(FrameOutcome::Presented {
            reconfigure_pending,
        }) = &outcome
        {
            self.dirty = *reconfigure_pending;
        }
        outcome
    }
    fn swap_graphics(&mut self, graphics: u64) -> KeplerResult<()> {
        self.effects.push(Effect::Swap(graphics));
        if self.fail_swap {
            return Err(KeplerError::Graphics(
                "injected replacement upload error".into(),
            ));
        }
        self.active = graphics;
        self.dirty = true;
        Ok(())
    }
    fn attach_fault_transport(&mut self) {
        self.effects.push(Effect::Attach);
    }
    fn resize(&mut self, width: u32, height: u32) {
        self.effects.push(Effect::Resize(width, height));
        self.ready = width > 0 && height > 0;
        self.dirty = true;
    }
    fn enqueue_volume(&mut self, volume: u8) {
        self.effects.push(Effect::Enqueue(volume));
    }
    fn clear_views(&mut self) {
        self.effects.push(Effect::Clear);
    }
    fn request_redraw(&mut self) {
        self.effects.push(Effect::Request);
    }
    fn wait(&mut self, delay: Option<Duration>) {
        self.effects.push(Effect::Wait(delay));
    }
    fn exit(&mut self) {
        self.effects.push(Effect::Exit);
    }
}

type Event = LifecycleEvent<u64, u64, u8>;

fn ms(value: u64) -> Duration {
    Duration::from_millis(value)
}

fn send(lifecycle: &mut ViewerLifecycle, host: &mut RecordingHost, event: Event) {
    let clock = Rc::clone(&host.clock);
    lifecycle.handle(event, host, || clock.get());
}

fn window(id: u64, signal: WindowSignal) -> Event {
    Event::Window { id, signal }
}

fn redraw(lifecycle: &mut ViewerLifecycle, host: &mut RecordingHost) {
    send(lifecycle, host, window(host.active, WindowSignal::Redraw));
}

fn expect_effects(host: &mut RecordingHost, expected: Vec<Effect>) {
    assert_eq!(std::mem::take(&mut host.effects), expected);
}

#[test]
fn repeated_resume_complete_os_redraw_and_clean_idle_use_live_ordering() {
    let mut lifecycle = ViewerLifecycle::default();
    let mut host = RecordingHost::default();
    send(&mut lifecycle, &mut host, Event::Resumed);
    send(&mut lifecycle, &mut host, Event::Resumed);
    expect_effects(&mut host, vec![Effect::Invalidate, Effect::Invalidate]);
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(
        &mut host,
        vec![Effect::Request, Effect::Wait(None), Effect::Wait(None)],
    );
    redraw(&mut lifecycle, &mut host);
    expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
    assert!(!host.dirty);
    for _ in 0..10 {
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        expect_effects(&mut host, vec![Effect::Wait(None)]);
    }
    // The OS can demand another complete frame even with no visual mutation.
    redraw(&mut lifecycle, &mut host);
    expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
    assert_eq!((host.active, host.interaction), (11, 37));
}

#[test]
fn unavailable_intervals_keep_pending_work_and_restore_without_acquisition() {
    for unavailable in 0..3 {
        let mut lifecycle = ViewerLifecycle::default();
        let mut host = RecordingHost::default();
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        host.effects.clear();
        match unavailable {
            0 => send(
                &mut lifecycle,
                &mut host,
                window(11, WindowSignal::Occluded(true)),
            ),
            1 => send(&mut lifecycle, &mut host, Event::Suspended),
            _ => send(
                &mut lifecycle,
                &mut host,
                window(11, WindowSignal::Resize(0, 480)),
            ),
        }
        host.effects.clear();
        redraw(&mut lifecycle, &mut host);
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        expect_effects(&mut host, vec![Effect::Wait(None)]);
        assert!(host.dirty);
        assert_eq!(host.interaction, 37);
        match unavailable {
            0 => send(
                &mut lifecycle,
                &mut host,
                window(11, WindowSignal::Occluded(false)),
            ),
            1 => send(&mut lifecycle, &mut host, Event::Resumed),
            _ => send(
                &mut lifecycle,
                &mut host,
                window(11, WindowSignal::Resize(640, 480)),
            ),
        }
        host.effects.clear();
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        expect_effects(&mut host, vec![Effect::Request, Effect::Wait(None)]);
        redraw(&mut lifecycle, &mut host);
        expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
        assert!(!host.dirty);
    }
}

#[test]
fn recoverable_results_schedule_from_frame_completion_without_consuming_pending_work() {
    let outcomes = [
        FrameOutcome::Skipped {
            recovery_action: super::super::frame::RecoveryAction::Retry,
            failure: SurfaceFailure::new(SurfaceFailureKind::Timeout, "acquire", "test"),
        },
        FrameOutcome::Skipped {
            recovery_action: super::super::frame::RecoveryAction::Probe,
            failure: SurfaceFailure::new(SurfaceFailureKind::Occluded, "acquire", "test"),
        },
        FrameOutcome::Presented {
            reconfigure_pending: true,
        },
    ];
    for outcome in outcomes {
        let mut lifecycle = ViewerLifecycle::default();
        let mut host = RecordingHost::default();
        host.outcomes.push_back(Ok(outcome));
        redraw(&mut lifecycle, &mut host);
        expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
        assert_eq!(host.clock.get(), ms(7));
        assert!(host.dirty);
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        expect_effects(&mut host, vec![Effect::Wait(Some(ms(16)))]);
        host.clock.set(ms(22));
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        expect_effects(&mut host, vec![Effect::Wait(Some(ms(1)))]);
        host.clock.set(ms(23));
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        expect_effects(&mut host, vec![Effect::Request, Effect::Wait(None)]);
        redraw(&mut lifecycle, &mut host);
        assert!(!host.dirty);
    }
}

#[test]
fn unavailable_frame_is_not_success_and_animation_recovery_and_mutation_use_live_scheduler() {
    let mut lifecycle = ViewerLifecycle::default();
    let mut host = RecordingHost::default();
    host.outcomes.push_back(Ok(FrameOutcome::Unavailable));
    redraw(&mut lifecycle, &mut host);
    assert!(host.dirty);
    host.effects.clear();
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Request, Effect::Wait(None)]);
    redraw(&mut lifecycle, &mut host);
    host.effects.clear();
    host.animating = true;
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Wait(Some(ms(16)))]);
    host.animating = false;
    host.recovery_in = Some(ms(60_000));
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Wait(Some(ms(60_000)))]);
    host.recovery_in = None;
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Wait(None)]);
    host.dirty = true;
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Request, Effect::Wait(None)]);
}

#[test]
fn fatal_render_stop_survives_resume_and_replacement_until_explicit_quit() {
    let mut lifecycle = ViewerLifecycle::default();
    let mut host = RecordingHost::default();
    host.outcomes
        .push_back(Err(KeplerError::Graphics("injected draw failure".into())));
    redraw(&mut lifecycle, &mut host);
    expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
    assert!(host.dirty);
    assert!(!lifecycle.shutting_down());
    send(&mut lifecycle, &mut host, Event::Resumed);
    send(&mut lifecycle, &mut host, Event::GraphicsReady(22, 3));
    host.effects.clear();
    redraw(&mut lifecycle, &mut host);
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Wait(None)]);
    send(&mut lifecycle, &mut host, Event::Quit);
    expect_effects(&mut host, vec![Effect::Clear, Effect::Exit]);
    assert!(lifecycle.shutting_down());
    for event in [
        Event::Quit,
        Event::Resumed,
        Event::AboutToWait,
        Event::GraphicsReady(33, 7),
    ] {
        send(&mut lifecycle, &mut host, event);
    }
    redraw(&mut lifecycle, &mut host);
    assert!(host.effects.is_empty());
    assert!(!lifecycle.accepts_window(22, 22));
}

#[test]
fn quit_clears_but_window_close_retains_layout_and_exits_only_once() {
    for quit in [false, true] {
        let mut lifecycle = ViewerLifecycle::default();
        let mut host = RecordingHost::default();
        let event = if quit {
            Event::Quit
        } else {
            window(11, WindowSignal::Close)
        };
        send(&mut lifecycle, &mut host, event);
        let expected = if quit {
            vec![Effect::Clear, Effect::Exit]
        } else {
            vec![Effect::Exit]
        };
        expect_effects(&mut host, expected);
        send(&mut lifecycle, &mut host, Event::Quit);
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        assert!(host.effects.is_empty());
    }
}

#[test]
fn replacement_orders_effects_routes_active_window_and_retains_suspension() {
    for suspended in [false, true] {
        let mut lifecycle = ViewerLifecycle::default();
        let mut host = RecordingHost::default();
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        if suspended {
            send(&mut lifecycle, &mut host, Event::Suspended);
        }
        host.effects.clear();
        send(&mut lifecycle, &mut host, Event::GraphicsReady(22, 9));
        expect_effects(
            &mut host,
            vec![
                Effect::Swap(22),
                Effect::Attach,
                Effect::Resize(800, 800),
                Effect::Enqueue(9),
            ],
        );
        assert_eq!((host.active, host.interaction), (22, 37));
        for signal in [
            WindowSignal::Redraw,
            WindowSignal::Close,
            WindowSignal::Occluded(true),
            WindowSignal::Resize(0, 0),
        ] {
            send(&mut lifecycle, &mut host, window(11, signal));
        }
        assert!(host.effects.is_empty());
        assert!(host.ready);
        send(&mut lifecycle, &mut host, Event::AboutToWait);
        if suspended {
            expect_effects(&mut host, vec![Effect::Wait(None)]);
            redraw(&mut lifecycle, &mut host);
            assert!(host.effects.is_empty());
            send(&mut lifecycle, &mut host, Event::Resumed);
            host.effects.clear();
            send(&mut lifecycle, &mut host, Event::AboutToWait);
        }
        expect_effects(&mut host, vec![Effect::Request, Effect::Wait(None)]);
        redraw(&mut lifecycle, &mut host);
        expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
    }
}

#[test]
fn failed_replacement_leaves_active_routing_and_outstanding_request_intact() {
    let mut lifecycle = ViewerLifecycle::default();
    let mut host = RecordingHost::default();
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    host.effects.clear();
    host.fail_swap = true;
    send(&mut lifecycle, &mut host, Event::GraphicsReady(22, 9));
    expect_effects(&mut host, vec![Effect::Swap(22)]);
    assert_eq!((host.active, host.interaction), (11, 37));
    send(&mut lifecycle, &mut host, window(22, WindowSignal::Close));
    assert!(host.effects.is_empty());
    send(&mut lifecycle, &mut host, Event::AboutToWait);
    expect_effects(&mut host, vec![Effect::Wait(None)]);
    redraw(&mut lifecycle, &mut host);
    expect_effects(&mut host, vec![Effect::Update, Effect::Render]);
    assert!(!lifecycle.shutting_down());
}
