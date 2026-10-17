"""从用户提供的原图提取 FocalRoom 图案，不重绘或改变原始比例。"""

import argparse
import hashlib
import json
from pathlib import Path

import numpy as np
from PIL import Image
from scipy import ndimage


def extract(source: Path, destination: Path) -> dict:
    if hashlib.sha256(source.read_bytes()).hexdigest() != "91c84b24305a32ad2f3beaf80fc985ab97ab5e4ff887cfef027c5623802aa414":
        raise ValueError("输入不是已核验的原始 logo，请勿套用固定提取区域")
    image = Image.open(source).convert("RGB")
    if image.size != (1254, 1254):
        raise ValueError("本脚本仅适用于已核验的 1254×1254 原始 logo")

    # 限定上方图案区域，文字完全排除。按白色距离隔离九个图形部件。
    rgb = np.asarray(image.crop((420, 340, 835, 738)), dtype=np.float64)
    labels, _ = ndimage.label(255 - rgb.min(axis=2) > 14)
    areas = np.bincount(labels.ravel())
    areas[0] = 0
    components = np.flatnonzero(areas > 100)
    if len(components) != 9:
        raise ValueError("图案应包含九个独立部件，请检查输入原图")
    mask = ndimage.binary_fill_holes(np.isin(labels, components))

    # 内部像素原样保留；仅在轮廓窄带估计 alpha，解除白底预乘。
    core = ndimage.binary_erosion(mask, iterations=2)
    support = ndimage.binary_dilation(mask, iterations=2)
    _, nearest = ndimage.distance_transform_edt(~core, return_indices=True)
    foreground = rgb[tuple(nearest)]
    contrast = 255 - foreground
    denominator = np.maximum(np.sum(contrast * contrast, axis=2), 1)
    alpha = np.clip(np.sum((255 - rgb) * contrast, axis=2) / denominator, 0, 1)
    alpha[core] = 1
    alpha[~support] = 0
    alpha[alpha < 0.035] = 0
    # 窄带中可能残留纸纹的独立像素，仅保留与九个主体相连的区域。
    alpha_labels, _ = ndimage.label(alpha > 0)
    alpha_areas = np.bincount(alpha_labels.ravel())
    alpha_areas[0] = 0
    subjects = np.flatnonzero(alpha_areas > 1000)
    if len(subjects) != 9:
        raise ValueError("透明轮廓缺少图案部件")
    alpha[~np.isin(alpha_labels, subjects)] = 0
    recovered = np.clip(255 - (255 - rgb) / np.maximum(alpha[..., None], 1 / 255), 0, 255)
    rgba = np.dstack((np.rint(recovered), np.rint(alpha * 255))).astype(np.uint8)
    rgba[rgba[..., 3] == 0, :3] = 0
    cutout = Image.fromarray(rgba)
    bounds = cutout.getbbox()
    if bounds is None:
        raise ValueError("抠图结果为空")
    cutout = cutout.crop(bounds)

    # 保持原始像素尺寸，居中留出约 9% 的透明边距，供 UI 和图标复用。
    canvas = Image.new("RGBA", (480, 480))
    canvas.paste(cutout, ((480 - cutout.width) // 2, (480 - cutout.height) // 2))
    destination.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(destination)
    return {
        "source_sha256": hashlib.sha256(source.read_bytes()).hexdigest(),
        "output_sha256": hashlib.sha256(destination.read_bytes()).hexdigest(),
        "size": list(canvas.size),
        "mode": canvas.mode,
        "components": len(components),
        "artwork_size": list(cutout.size),
        "opaque_source_pixels_preserved": bool(np.array_equal(rgba[core, :3], rgb[core].astype(np.uint8))),
    }


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("source", type=Path)
    parser.add_argument("destination", type=Path)
    args = parser.parse_args()
    print(json.dumps(extract(args.source, args.destination), ensure_ascii=False, indent=2))
