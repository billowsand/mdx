#!/usr/bin/env python3
"""Verify a file against an expected SHA-256 digest."""

from __future__ import annotations

import hashlib
from pathlib import Path
import string
import sys


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    if len(sys.argv) != 3:
        print(f"usage: {Path(sys.argv[0]).name} EXPECTED_SHA256 FILE", file=sys.stderr)
        return 2

    expected = sys.argv[1].lower()
    path = Path(sys.argv[2])
    if len(expected) != 64 or any(char not in string.hexdigits for char in expected):
        print(f"invalid SHA-256 digest: {sys.argv[1]}", file=sys.stderr)
        return 2

    actual = sha256(path)
    if actual != expected:
        print(
            f"{path}: SHA-256 mismatch (expected {expected}, got {actual})",
            file=sys.stderr,
        )
        return 1

    print(f"{path}: OK")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
