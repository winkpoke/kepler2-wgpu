# Kepler viewer

Language for the arrangement and interaction of medical-image views.

## Language

**View layout**:
The arrangement of MPR, MIP, or mesh views displayed together in the viewer.
_Avoid_: Render mode, view state

**View-layout transition**:
A change from the current displayed view layout to a requested view layout.
_Avoid_: Mesh toggle, mode switch when referring only to layout

**View interaction state**:
The current slice position, pan, zoom, window/level, and rotation settings of a view, where those settings apply.
_Avoid_: Saved states when the intended meaning is orientation selection

**View-slot orientation**:
The MPR orientation selected for a position within a view layout.
_Avoid_: Saved state
