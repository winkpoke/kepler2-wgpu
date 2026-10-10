# Design

## Context

See [proposal](proposal.md) for scope. `ViewerHandler` owns an initialized `App`, proxy, clock, `FrameOwner`, and shutdown flag. `frame::execute` and `RedrawScheduler` already earn their depth and remain unchanged. `App::input` is called only for the active window before lifecycle/input handling. Native `run_app`, browser `spawn_app`, queued controls, and the GPU-ready constructor remain unchanged.

## Goals / Non-Goals

Goals: concentrate lifecycle ordering behind a private event-handling interface; test the live operation with recording effects and deterministic time; remove the AppView factory-before-rebuild precondition.

Non-goals: move general controls or bootstrap into a new owner, alter frame outcomes or retry timing, implement GPU/panic recovery, change signed encoding or shaders, or claim pre-run control delivery/browser acceptance from deterministic tests.

## Decisions

1. A private `ViewerLifecycle` owns the existing `FrameOwner` and explicit shutdown flag. It borrows an adapter for application operations and window/event-loop effects. Winit translates supported events; a recording adapter supplies frame/replacement results and effect observations. Both cross the same event-handling seam. Active-window filtering and `App::input` consumption remain before native controls; supported lifecycle events also validate the active ID through the shared operation. Keeping pure policy alone would leave the same calling-order gap; replacing the frame transaction would duplicate tested logic.
2. Redraw acknowledges an outstanding request before readiness, then updates and renders once. Outcome time is sampled after rendering. OS redraws draw even when clean. Hidden/suspended/zero-size/stopped viewers do not update/render. The existing scheduler supplies bounded retry, animation, and recovery deadlines; request precedes selecting Wait.
3. Successful replacement orders swap, fault-transport attachment, old-window request acknowledgement/unocclusion, 800x800 resize, and one supplied-volume enqueue. A returned swap error stops all subsequent effects. The same lifecycle owner survives replacement, preserving suspended/stopped state. No rollback beyond existing swap behavior is added.
4. Fatal frame errors stop automatic scheduling, with existing logging, but do not exit or prohibit later controls. Explicit Quit clears layout then stops/exits; Close/Escape stop/exit without clearing. Explicit shutdown guards all later dispatch. Resume cannot clear fatal stop.
5. AppView's bulk rebinding operation accepts one graphics configuration, selects/refreshes its compatible factory internally, then rebuilds existing views from that factory's shared MPR context. App continues to prepare replacement volume content before mutation and owns graphics/depth resources. Standalone view methods remain public-compatible. Same-format resize and compatible factory reuse remain unchanged; device change reuploads, format-only change keeps textures, and CPU state survives.

## Risks / Trade-offs

- Adapter plumbing could merely relocate sequencing: production and tests must call the complete lifecycle operation, with primitive adapter effects and observed ordering.
- Winit translation or input consumption could drift: retain existing active-ID/input gate, use production translation tests where practical, and separately run native smoke; deterministic tests do not prove OS callbacks.
- A factory/context mismatch could survive: migrate the existing GPU rebinding test to the complete operation and retain shared-context, CPU-state, pending-frame, pixel, and two-device assertions.
- Browser automation is unavailable in this session: keep acceptance unchecked and record the actual blocker; packaging/native results do not substitute.

## Migration Plan

Introduce the private sequence and recording regressions, route current Winit dispatch, document ownership, then consolidate AppView rebinding and migrate its existing GPU tests. Run locked native/WASM/packaging, existing frame/policy/resource/encoding tests, docs/spec/maintenance checks, scoped formatting, and full quality gates. Use a synthetic native harness; retain browser acceptance as a separate task. Rollback the internal refactor together with its caller changes; no data or public interface migration is needed.
