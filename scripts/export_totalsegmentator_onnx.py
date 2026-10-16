#!/usr/bin/env python
# -*- coding: utf-8 -*-
"""
Export a TotalSegmentator (vertebrae) nnU-Net model to ONNX.

Why not nnunetv2.inference.model_restore?
    That module does not exist in nnunetv2 >= 2.x. TotalSegmentator
    itself loads the network through `nnUNetPredictor`, so we do the
    same and grab `predictor.network` for export.

CRITICAL — channel ordering (verified empirically, see
scripts/verify_channel_mapping.py):
    The trained vertebrae network (Dataset292) has 27 output heads.
    nnU-Net's head order is the **sorted, unique set of dataset.json
    label ids**, so channel index == label id:
        0  background
        1  sacrum
        2  S1
        3  L5
        4  L4
        5  L3
        6  L2
        7  L1
        8..26  T12..C1
    It is NOT "channel 0 = first foreground class". Any assumption that
    channel i == the i-th foreground class is off and produces a fully
    scrambled (reversed) mapping. The only correct way to pick a class
    is by its real label id.

Fix A — 9 channels + amax, exactly equivalent to the 27-class argmax:
    We keep channel 0 (background) plus the 7 spine/sacrum label ids,
    re-ordered so the ONNX output channel index (1..7) equals the kepler
    label id (1..7):  output = [bg, L1, L2, L3, L4, L5, S1, sacrum].
    All remaining channels (8..26, the T/C vertebrae) are collapsed into
    one "rest" channel via max. Because
        argmax_c v[c] == argmax( max_spine v[s], max_rest v[r] ),
    a 9-channel argmax is byte-identical to a 27-channel argmax, at ~1/3
    the memory. The Rust host writes 0 when the winner is channel 0
    (background) or the last channel (rest); otherwise it writes the
    channel index, which is the kepler label id (1..7).

Usage:
    src/server/totalsegmentator/.venv/Scripts/python.exe \
        scripts/export_totalsegmentator_onnx.py \
        --out models/totalsegmentator.onnx
"""

from __future__ import annotations

import argparse
import os
import sys

# Windows consoles default to a GBK codepage; torch's ONNX exporter prints
# emoji (✅) on success, which raises UnicodeEncodeError and aborts the run
# *before* the model is written. Force UTF-8 so the export always completes.
try:
    sys.stdout.reconfigure(encoding="utf-8")
    sys.stderr.reconfigure(encoding="utf-8")
except Exception:
    pass

import torch
import torch.nn as nn

# --- TotalSegmentator weights live here by default -----------------------
DEFAULT_RESULTS = os.path.expanduser(
    "~/.totalsegmentator/nnunet/results"
)
DEFAULT_MODEL_FOLDER = os.path.join(
    DEFAULT_RESULTS,
    "Dataset292_TotalSegmentator_part2_vertebrae_1532subj",
    "nnUNetTrainerNoMirroring__nnUNetPlans__3d_fullres",
)

# Real TotalSegmentator (Dataset292) label ids for the spine/sacrum subset,
# read from dataset.json (`labels` field). Channel i of the nnU-Net output
# equals label id i (channel 0 = background), so to select a class we index
# by its label id, NOT by its position among foreground classes:
#   id 1 = sacrum, 2 = S1, 3 = L5, 4 = L4, 5 = L3, 6 = L2, 7 = L1
#          8..26 = T12..C1 (ignored by kepler, collapsed into "rest")
#
# We export the spine classes in kepler's anatomical order so that the ONNX
# output channel index (1..7) equals the kepler label id (1..7):
#   output[1]=L1  output[2]=L2  output[3]=L3  output[4]=L4
#   output[5]=L5  output[6]=S1  output[7]=sacrum
SPINE_TS_LABEL_IDS = [7, 6, 5, 4, 3, 2, 1]   # L1,L2,L3,L4,L5,S1,sacrum

# Final ONNX output channel order (length 9). Channel 0 is background;
# channels 1..7 are the spine labels (kepler id == channel index);
# channel 8 ("rest") is the max over every other TS class -> background.
EXPORT_LABELS = [
    "background", "L1", "L2", "L3", "L4", "L5", "S1", "sacrum", "rest",
]


class SpineChannelSelect(nn.Module):
    """Wrap the nnU-Net network and emit a 9-channel spine tensor whose
    argmax is byte-identical to the full 27-class argmax.

    Output channel order: [background, L1, L2, L3, L4, L5, S1, sacrum, rest].
    Input/output: [B, C, D, H, W].
    """

    def __init__(self, net: nn.Module, spine_idx):
        super().__init__()
        self.net = net
        # Real TotalSegmentator label ids of the spine/sacrum classes,
        # in the order we want them to appear in the ONNX output
        # (channels 1..7). Channel 0 (background) is prepended below.
        self.spine_idx = list(spine_idx)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        logits = self.net(x)                      # [B, 27, D, H, W]
        keep_idx = [0] + self.spine_idx           # background + 7 spine classes
        keep = logits[:, keep_idx]                # [B, 8, ...]

        # Every channel not in `keep_idx` (8..26: the T- and C-spine classes)
        # is collapsed into a single "rest" channel via max-pooling across
        # channels. argmax over the 9 output channels then equals argmax over
        # all 27 (argmax_c v[c] == argmax( max_spine v[s], max_rest v[r] )).
        c_total = logits.shape[1]
        rest_idx = [i for i in range(c_total) if i not in keep_idx]
        rest = logits[:, rest_idx].amax(dim=1, keepdim=True)  # [B, 1, ...]

        return torch.cat([keep, rest], dim=1)     # [B, 9, D, H, W]


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--model-folder", default=DEFAULT_MODEL_FOLDER,
                    help="nnUNetTrainer output dir (contains fold_X + plans.json)")
    ap.add_argument("--checkpoint", default="checkpoint_final.pth")
    ap.add_argument("--out", default="models/totalsegmentator.onnx",
                    help="output ONNX path; filename STEM must equal the "
                         "'model' field sent to POST /api/segment")
    ap.add_argument("--patch", nargs=3, type=int, default=[128, 128, 128],
                    metavar=("D", "H", "W"),
                    help="dummy input spatial size for tracing")
    ap.add_argument("--opset", type=int, default=17)
    ap.add_argument("--device", default="cpu", choices=["cpu", "cuda"])
    args = ap.parse_args()

    if not os.path.isdir(args.model_folder):
        raise SystemExit(f"model folder not found: {args.model_folder}")

    # Sanity: the spine label ids must sit inside the network's channel range
    # and none of them may be 0 (background, which is added automatically).
    assert all(0 < i <= 26 for i in SPINE_TS_LABEL_IDS), \
        "SPINE_TS_LABEL_IDS must be real foreground label ids (1..26)"
    assert len(set(SPINE_TS_LABEL_IDS)) == len(SPINE_TS_LABEL_IDS), \
        "SPINE_TS_LABEL_IDS must be unique"

    # Late imports: only needed at runtime, not for --help
    from nnunetv2.inference.predict_from_raw_data import nnUNetPredictor

    print(f"[export] loading predictor from: {args.model_folder}")
    predictor = nnUNetPredictor(
        tile_step_size=0.5,
        use_gaussian=True,
        use_mirroring=False,   # single forward pass; deterministic for ONNX
        perform_everything_on_device=True,
        device=torch.device(args.device),  # must be torch.device, not str
        verbose=False,
    )
    predictor.initialize_from_trained_model_folder(
        args.model_folder,
        # use_folds=("all",) is buggy in nnunetv2 2.8.x: it looks for a
        # literal fold_all dir. None triggers auto_detect_available_folds
        # which correctly finds fold_0 (and any other existing folds).
        use_folds=None,
        checkpoint_name=args.checkpoint,
    )

    net = predictor.network
    net.eval()
    wrapper = SpineChannelSelect(net, SPINE_TS_LABEL_IDS).to(args.device).eval()

    d, h, w = args.patch
    dummy = torch.randn(1, 1, d, h, w, device=args.device)

    os.makedirs(os.path.dirname(os.path.abspath(args.out)), exist_ok=True)

    print(f"[export] tracing ONNX -> {args.out}  (opset {args.opset})")
    with torch.no_grad():
        torch.onnx.export(
            wrapper,
            dummy,
            args.out,
            input_names=["input"],
            output_names=["output"],
            # spatial dims are dynamic so the model accepts arbitrary
            # volume sizes (it is fully convolutional)
            dynamic_axes={
                "input": {2: "D", 3: "H", 4: "W"},
                "output": {2: "D", 3: "H", 4: "W"},
            },
            opset_version=args.opset,
            do_constant_folding=True,
            # Use the legacy TorchScript exporter (dynamo=False): it honors
            # opset_version=17 and dynamic_axes predictably, and avoids the
            # newer dynamo path that (a) auto-bumps to opset 18 and (b) trips
            # on GBK consoles. external_data keeps the large 3D-U-Net weights
            # in totalsegmentator.onnx.data (matching the existing layout).
            dynamo=False,
            external_data=True,
        )

    # --- sanity check (needs `onnx` installed) --------------------------
    result_path = os.path.join(
        os.path.dirname(os.path.abspath(__file__)),
        "export_channel_check.txt",
    )
    try:
        import onnx
        m = onnx.load(args.out)
        onnx.checker.check_model(m)
        inp = m.graph.input[0]
        out = m.graph.output[0]
        in_shape = [d.dim_value if d.dim_value > 0 else d.dim_param
                    for d in inp.type.tensor_type.shape.dim]
        out_shape = [d.dim_value if d.dim_value > 0 else d.dim_param
                     for d in out.type.tensor_type.shape.dim]
        n_out = out_shape[1]
        print(f"[export] OK  input {in_shape}  output {out_shape}")
        print(f"[export] output channel order: {EXPORT_LABELS}")
        print(f"[export] -> {n_out} channels; argmax == 27-class argmax")
        with open(result_path, "w", encoding="utf-8") as rf:
            rf.write(f"output_channels={n_out}\n")
            rf.write(f"expected=9\n")
            rf.write(f"channel_order={','.join(EXPORT_LABELS)}\n")
            rf.write(f"ok={n_out == 9}\n")
    except ImportError:
        print("[export] done (install `onnx` to run shape check)")
        with open(result_path, "w", encoding="utf-8") as rf:
            rf.write("skipped: onnx not installed\n")
    except Exception as e:
        print(f"[export] sanity check FAILED: {e}")
        with open(result_path, "w", encoding="utf-8") as rf:
            rf.write(f"failed: {e}\n")

    print(f"[export] wrote {args.out}")


if __name__ == "__main__":
    main()
