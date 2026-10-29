# Plan beam arrangement rendering

Approved by the owner on 2026-10-24: implement plan-wide visibility, selected-beam
BEV and independent delivery inspection. The owner explicitly directed that rendering
code belongs in Kepler. Existing BEV rendering is reused without a duplicate pipeline.

Add a bounded geometric MPR beam batch with one buffer owner per pane. The egui host
owns selection, visibility, labels, associations and layout. No fluence calculation,
delivery control, new dependency, or 3D beam path is introduced.
