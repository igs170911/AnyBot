#!/usr/bin/env python3
"""Write a 1024x1024 solid-color PNG with one letter. No external assets."""

import struct
import zlib
from pathlib import Path


def chunk(tag: bytes, data: bytes) -> bytes:
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)


def png(width: int, height: int, paint) -> bytes:
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        for x in range(width):
            raw.extend(paint(x, y))
    return b"".join(
        [
            b"\x89PNG\r\n\x1a\n",
            chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)),
            chunk(b"IDAT", zlib.compress(bytes(raw), 9)),
            chunk(b"IEND", b""),
        ]
    )


def letter_a(x: int, y: int, size: int) -> bool:
    # A block letter inside a 1024 canvas. Coordinates are pixels.
    left = size * 28 // 100
    right = size * 72 // 100
    top = size * 18 // 100
    bottom = size * 82 // 100
    mid = size * 52 // 100
    thick = size * 10 // 100
    if y < top or y > bottom or x < left or x > right:
        return False
    span = right - left
    rise = bottom - top
    center = left + span * (bottom - y) // rise
    half = thick // 2 + (bottom - y) * thick // (rise * 3)
    on_stroke = abs(x - center) <= half or abs(x - (left + right - center)) <= half
    on_bar = mid - thick // 2 <= y <= mid + thick // 2 and left + thick <= x <= right - thick
    return on_stroke or on_bar


def paint(x: int, y: int) -> bytes:
    if letter_a(x, y, 1024):
        return b"\xff\xff\xff"
    return b"\x1f\x6f\x4a"


def main() -> None:
    out = Path(__file__).resolve().parents[2] / "target" / "placeholder-icon.png"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_bytes(png(1024, 1024, paint))
    print(out)


if __name__ == "__main__":
    main()
