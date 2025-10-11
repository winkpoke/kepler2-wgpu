Title: MIP uniforms initialization fix for MVP

Summary
- Implemented per-frame initialization of MIP uniforms in MipView::update.
- Ensures camera vectors, volume size, window/level, and view_matrix are valid defaults for MVP.
- Fix addresses blank output by providing the fragment shader with non-zero, consistent parameters.

Details
- Set camera in normalized volume space: pos (0.5, 0.5, -0.5), front (0,0,1), up (0,1,0), right (1,0,0).
- volume_size defaults to [1,1,1] (normalized). Future work: use actual voxel dimensions.
- Window/Level defaults:
  - RG8 packed volumes: window=4096, level=2048.
  - Float volumes (R16F/R32F): window=1.0, level=0.5.
- view_matrix is identity for orthographic-like traversal.
- Upload via update_uniforms before render.

Performance & Logging
- Uses DEBUG logs for update and render stages.
- No heavy TRACE logging added.

Impact
- MIP view should now show meaningful output for MVP layout.
- Preserves medical imaging accuracy constraints by avoiding perspective projection and using window/level appropriately.