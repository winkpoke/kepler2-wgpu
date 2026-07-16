"""TotalSegmentator inference wrapper.

This module is intentionally thin: it runs TotalSegmentator on an input CT
volume and post-processes the result into a **per-label spine mask** (one
byte per voxel; 0 = background, 1..7 = L1, L2, L3, L4, L5, S1, sacrum).

TotalSegmentator's ``total`` task produces ~117 anatomical structures. For
the kepler2-wgpu workflow we only care about the lumbar/sacral spine, so
we ignore every other file and merge the seven masks we keep into a single
``spine.nii.gz``. The resulting buffer matches the format expected by
``render_content::from_labels_r8`` on the Rust side (a contiguous
``uint8`` buffer of length ``width*height*depth``).

TotalSegmentator's ``python_api`` does not expose fine-grained progress
callbacks, so this module only returns when the work is done. The
surrounding FastAPI service (``app.py``) therefore steps the progress
percentage in coarse chunks (0 / 10 / 80 / 100); the Rust poller is
responsible for smoothing that into a 0..100 stream.
"""
from __future__ import annotations

import logging
from pathlib import Path
from typing import Tuple
import SimpleITK as sitk
import nibabel as nib
import numpy as np

log = logging.getLogger(__name__)

# TotalSegmentator task name. `total` produces all 117 anatomical structures
DEFAULT_TASK = "total"

# Label table returned to the Rust client.
# Only the L1-L5 + S1 + sacrum subset is kept: these are the vertebrae the
# user needs to inspect in the MPR view, and TotalSegmentator's `total` task
# happens to assign them contiguous ids 27..31 (L1..L5), 26 (S1), 25 (sacrum)
# — but we renumber them to 1..7 so the GPU/WGSL/Rust side can use a small
# fixed-size `label_colors[8]` table.
#
# Id ordering follows anatomical position (top to bottom):
#   1 L1, 2 L2, 3 L3, 4 L4, 5 L5, 6 S1, 7 sacrum
LABEL_TABLE = {
    1: "L1",
    2: "L2",
    3: "L3",
    4: "L4",
    5: "L5",
    6: "S1",
    7: "sacrum",
}

# Vertebrae files we keep from the TotalSegmentator output, in anatomical
# (top-to-bottom) order. The first tuple element is the file stem (without
# the `.nii.gz` extension), the second is the label id we write into the
# combined mask.
KEPT_VERTEBRAE: list[tuple[str, int]] = [
    ("vertebrae_L1", 1),
    ("vertebrae_L2", 2),
    ("vertebrae_L3", 3),
    ("vertebrae_L4", 4),
    ("vertebrae_L5", 5),
    ("vertebrae_S1", 6),
]
SACRUM_LABEL = 7

# Filenames to delete after merging so the output directory only contains
# the files we actually use. TotalSegmentator's `total` task produces ~117
# files (and the `vertebrae` subset alone is C1..L5 = 24 files); we keep
# only the 7 we care about.
_KEPT_STEMS = {stem for stem, _ in KEPT_VERTEBRAE} | {"sacrum"}

def run_totalsegmentator(
    input_path: str,
    output_dir: str,
    task: str = DEFAULT_TASK,
    fast: bool = False,
) -> None:
    """Run TotalSegmentator on ``input_path`` and write the per-label spine mask.

    Parameters
    ----------
    input_path : str
        Path to a volume that TotalSegmentator can read. Typically an
        ``.mha`` file saved by the Rust server under
        ``KEPLER_SERIES_DIR/<series_id>.mha``.
    output_dir : str
        Output directory. TotalSegmentator will write per-class
        ``.nii.gz`` files here; this function merges the seven we keep
        (L1..L5, S1, sacrum) into a single ``spine.nii.gz`` containing the
        multi-label mask and deletes every other file.
    task : str
        TotalSegmentator task name. Default ``total``.
    fast : bool
        Forwarded to TotalSegmentator's ``fast`` flag. ``True`` halves
        the resolution and is a good default for prototyping.
    """
    # Import inside the function so the module can be imported (for tests
    # / type checking) without the heavy PyTorch dependency installed.
    from totalsegmentator.python_api import totalsegmentator

    out = Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)

    # TotalSegmentator's python_api requires NIfTI input. Convert .mha
    # (or any SimpleITK-readable format) to .nii.gz in a temp location.
    input_p = Path(input_path)
    if input_p.suffix == ".mha" or input_p.suffix == ".mhd":
        nii_input = out / "input.nii.gz"
        log.info("Converting %s -> %s for TotalSegmentator", input_path, nii_input)
        img = sitk.ReadImage(str(input_p))
        sitk.WriteImage(img, str(nii_input))
        ts_input = str(nii_input)
    else:
        ts_input = input_path

    log.info("Running TotalSegmentator task=%s fast=%s on %s -> %s",
             task, fast, ts_input, out)
    totalsegmentator(ts_input, str(out), task=task, fast=fast)

    # Find an existing kept-vertebra file to recover the affine/header.
    # (TotalSegmentator only writes the files for structures it actually
    # detects — e.g. an L1-only volume won't have C1..T12.)
    template_path: Path | None = None
    for stem, _ in KEPT_VERTEBRAE:
        candidate = out / f"{stem}.nii.gz"
        if candidate.exists():
            template_path = candidate
            break
    if template_path is None:
        sacrum_fallback = out / "sacrum.nii.gz"
        if sacrum_fallback.exists():
            template_path = sacrum_fallback
        else:
            raise FileNotFoundError(
                f"TotalSegmentator produced none of the kept vertebrae (or "
                f"sacrum) in {out}. task={task} may be wrong for this "
                f"volume (e.g. wrong modality or anatomy not in the FOV)."
            )

    # Merge only the vertebrae we care about (L1..L5, S1). Other files
    # (C1..C7, T1..T12) are ignored even if TotalSegmentator produced them.
    log.info("Combining vertebra + sacrum files into a single spine mask")
    first = nib.load(str(template_path))
    combined = np.zeros(first.shape, dtype=np.uint8)

    for stem, label_id in KEPT_VERTEBRAE:
        vf = out / f"{stem}.nii.gz"
        if not vf.exists():
            log.warning("Expected vertebra file missing: %s", vf)
            continue
        data = nib.load(str(vf)).get_fdata()
        # Write the (anatomical order) label id; must match LABEL_TABLE.
        combined[data > 0] = label_id

    sacrum = out / "sacrum.nii.gz"
    if sacrum.exists():
        sacrum_data = nib.load(str(sacrum)).get_fdata()
        # sacrum is label 7 (after L1..S1 = 1..6). Must match LABEL_TABLE.
        combined[sacrum_data > 0] = SACRUM_LABEL

    out_path = out / "spine.nii.gz"
    nib.save(
        nib.Nifti1Image(combined, first.affine, first.header),
        str(out_path),
    )
    log.info("Wrote collapsed spine mask (%d labels) to %s",
             len(LABEL_TABLE), out_path)

    # Clean up: remove all non-vertebrae NIfTI files and the converted input
    # to save disk space (total task produces ~117 files). Also drop the
    # vertebra masks we don't use (C1..C7, T1..T12) so the output folder
    # only contains the 7 labels plus the combined spine.nii.gz.
    keep = {"spine.nii.gz", "input.nii.gz", "sacrum.nii.gz"} | {f"{stem}.nii.gz" for stem in _KEPT_STEMS}
    removed = 0
    for f in out.glob("*.nii.gz"):
        if f.name not in keep:
            f.unlink()
            removed += 1
    if removed:
        log.info("Cleaned up %d unused files from %s", removed, out)


def extract_mask_bytes(output_dir: str) -> bytes:
    """Read ``spine.nii.gz`` and return the raw uint8 mask bytes.

    The returned buffer is a contiguous row-major ``width * height * depth``
    array of **label ids** (0 = background, 1..N = vertebra/sacrum). This
    is the format expected by ``AiService::download`` and
    ``RenderContent::from_labels_r8`` on the Rust side: the GPU uploads the
    buffer as an ``R8Uint`` 3D texture and the WGSL shader indexes into a
    ``label_table`` uniform to colour each label.

    Nibabel returns data in (X, Y, Z) order. The Rust CTVolume and the WGSL
    texture sampler expect (Z, Y, X) row-major (byte offset
    ``= z*X*Y + y*X + x``). Transpose the nibabel array into that layout
    on the way out so the per-voxel label matches the same voxel the CT
    volume samples.
    """
    spine = Path(output_dir) / "spine.nii.gz"
    if not spine.exists():
        raise FileNotFoundError(f"spine.nii.gz not found in {output_dir}")
    img = nib.load(str(spine))
    data = img.get_fdata().astype(np.uint8)
    # Nibabel shape is (X, Y, Z). Transpose to (Z, Y, X) for the GPU.
    data = np.transpose(data, (2, 1, 0))
    return np.ascontiguousarray(data).tobytes()


def mask_dimensions(output_dir: str) -> Tuple[int, int, int]:
    """Return ``(width, height, depth)`` of the spine mask, or ``(0,0,0)`` if
    the file does not exist.

    The NIfTI file is (X, Y, Z); we return it transposed to (Z, Y, X) which
    is the (width, height, depth) order the WGSL ``texture_3d`` and the
    Rust ``RenderContent::from_labels_r8`` expect.
    """
    spine = Path(output_dir) / "spine.nii.gz"
    if not spine.exists():
        return (0, 0, 0)
    img = nib.load(str(spine))
    x, y, z = img.shape
    return (z, y, x)
