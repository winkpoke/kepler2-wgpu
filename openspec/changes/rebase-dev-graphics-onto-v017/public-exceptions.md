# Public graphics decisions

Implementation authorization: the user's 2026-10-05 request, "following the
design and apply it." The user requires preservation of the v0.17.0 viewer and
browser public API because downstream software depends on those controls.
Their clarification is: "they only rely on kepler's viewer and browser."
P2 and P3 were explicitly approved. These decisions supersede historical
approval records in imported documents.

## P1 — viewer/browser compatibility boundary clarified

Preserve Kepler's viewer/browser entry points, argument order, generated browser
declarations, reply shapes, and existing display/control semantics. The upstream
graphics dependency migration proceeds within that clarified boundary; this
is not a claim that wgpu 23 objects become compatible with wgpu 30 objects.
Low-level graphics consumers must follow the dependency migration below.

Align the viewer and native `wgpu30` compute alias with wgpu 30.0.1, and move
Winit from 0.29.15 to 0.30.13. Public Kepler names and field shapes remain, but
their upstream argument/field types have new crate identities. A wgpu 23
`Device`, `Queue`, `RenderPass`, or pipeline is incompatible with its wgpu 30
counterpart. Device ownership still remains independent between viewer and
server compute.

Consumer migration: align graphics dependencies, update upstream descriptor and
event-loop use, and recreate resources on the proper device. Exported Kepler
domain/browser/server APIs remain governed by the release compatibility rule.

Alternative: retain the old public graphics stack through separate owners and
an explicitly designed compatibility facade. Cross-version object casts or
sharing cannot implement that facade. This would alter the proposed migration
scope rather than provide source compatibility for free.

## P2 — surface error contracts: approved

Requested signatures, following the source migration:

| Release | Proposed |
| --- | --- |
| `Renderable::render(...) -> Result<(), wgpu::SurfaceError>` | `Renderable::render(...) -> KeplerResult<()>` |
| `App::render(&mut self) -> Result<(), wgpu::SurfaceError>` | `App::render(&mut self) -> KeplerResult<()>` |
| `KeplerError::Surface(wgpu::SurfaceError)` | `KeplerError::Surface(SurfaceFailure)` |
| `From<wgpu::SurfaceError> for KeplerError` | `From<SurfaceFailure> for KeplerError` |

wgpu 30 no longer exports `SurfaceError`; acquisition returns
`CurrentSurfaceTexture`. The source's owned `SurfaceFailure` carries `kind`,
`operation`, and `message`. Its kind includes timeout, occluded, outdated, lost,
validation, out-of-memory, device-lost, and other failures.

Consumer migration: change custom `Renderable` return types and match
`KeplerError::Surface(failure)` through `failure.kind`. Callers propagating errors
can continue to use the Kepler result alias.

Alternative to investigate if preservation is selected: keep the old wgpu 23
error type via a compatibility dependency, retain the release conversion and
public render signatures, and map modern failures at the boundary. This keeps
two upstream type families and cannot expose new failure detail through the old
enum alone. No such adapter is implemented or claimed faithful yet.

## P3 — render-pass context lifetimes: approved

Requested public shape:

```rust
// Release
PassContext<'a> { pass: &'a mut wgpu::RenderPass<'a>, /* existing fields */ }
// Proposed
PassContext<'pass, 'encoder> {
    pass: &'pass mut wgpu::RenderPass<'encoder>, /* existing fields */
}
```

The constructor retains its arguments and gains two independent lifetime
parameters. Explicit consumer annotations change from `PassContext<'a>` to
`PassContext<'pass, 'encoder>`; callback annotations with elided lifetimes can
usually retain their spelling. The descriptor uses the short borrow lifetime.

Rust 1.98.0 compile probes against wgpu 30.0.1 reject the existing callback shape
with E0597 because the pass is dropped while still borrowed. Using wgpu's safe
`forget_lifetime()` also fails; separating the lifetimes compiles. Source probes
are in [baseline/](baseline/). These probes establish the local borrow issue,
not candidate runtime correctness or proof that every possible wrapper fails.
No unsafe lifetime conversion will be used. Selecting preservation requires
further design of the public callback boundary before that phase proceeds.

## P4 — format constructors/helpers: preserve published APIs

No public exception is requested. Keep `MprRenderContext::new(device, queue)`,
`MultiMeshContext::new(device, queue)`, the basic lighting pipeline helper, and
the swapchain-format helpers with their release signatures and legacy behavior.
Add explicit-format alternatives for production owners; internal factories and
rebinding use those alternatives. Required GPU checks cover independent BGRA
and RGBA owners, format changes, and retained legacy constructor calls.

The replay retains these public constructors and adds owner-specific format paths. Release public encoding variants, metadata fields, WindowLevel
fields/bounds, browser declarations, server contracts, and package versions are
not included in any requested exception.
