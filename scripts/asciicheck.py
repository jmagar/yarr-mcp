#!/usr/bin/env python3
"""Check files for non-ASCII characters.

Use --fix to replace common smart punctuation with ASCII equivalents.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

SUBSTITUTIONS: dict[int, str] = {
    0x00A0: " ",  # non-breaking space
    0x2011: "-",  # non-breaking hyphen
    0x2013: "-",  # en dash
    0x2014: "-",  # em dash
    0x2018: "'",  # left single quote
    0x2019: "'",  # right single quote
    0x201C: '"',  # left double quote
    0x201D: '"',  # right double quote
    0x2026: "...",  # ellipsis
    0x202F: " ",  # narrow non-breaking space
}

ALLOWED_UNICODE_CODEPOINTS: set[int] = {
    0x00A7,  # section sign in pattern references
    0x00D7,  # multiplication sign in dimensions
    0x2014,  # em dash used in docs/comments
    0x2013,  # en dash used in docs/comments
    0x2026,  # ellipsis used in CLI output/docs
    0x20AC,  # euro sign used in token/cost examples
    0x2192,  # right arrow used in docs/comments
    0x2190,  # left arrow used in diagrams
    0x2193,  # down arrow used in diagrams
    0x2194,  # left-right arrow used in docs
    0x2248,  # approximately equal sign used in token/cost examples
    0x2264,  # less-than-or-equal sign used in docs
    0x2265,  # greater-than-or-equal sign used in docs
    0x2500,  # box drawing chars used in section dividers/trees
    0x2501,
    0x2502,
    0x250C,
    0x2510,
    0x2514,
    0x2518,
    0x251C,
    0x26A0,  # warning sign used in CLI output/docs
    0x2713,  # check mark used in CLI output/docs
    0x2717,  # cross mark used in CLI output/docs
    0xFE0F,  # variation selector used with warning sign
}


def main() -> int:
    parser = argparse.ArgumentParser(description="Check files for non-ASCII characters.")
    parser.add_argument(
        "--fix",
        action="store_true",
        help="Rewrite files, replacing common non-ASCII characters with ASCII equivalents.",
    )
    parser.add_argument("files", nargs="+", help="Files to check.")
    args = parser.parse_args()

    has_errors = False
    for filename in args.files:
        has_errors |= lint_utf8_ascii(Path(filename), fix=args.fix)
    return 1 if has_errors else 0


def lint_utf8_ascii(filename: Path, fix: bool) -> bool:
    """Return True if an error was printed."""
    try:
        raw = filename.read_bytes()
        text = raw.decode("utf-8")
    except UnicodeDecodeError as exc:
        print(f"{filename}: UTF-8 decoding error:")
        print(f"  byte offset: {exc.start}")
        print(f"  reason: {exc.reason}")
        partial = raw[: exc.start]
        line = partial.count(b"\n") + 1
        col = exc.start - (partial.rfind(b"\n") if b"\n" in partial else -1)
        print(f"  location: line {line}, column {col}")
        return True

    errors: list[tuple[int, int, str, int]] = []
    for lineno, line in enumerate(text.splitlines(keepends=True), 1):
        for colno, char in enumerate(line, 1):
            codepoint = ord(char)
            if char in ("\n", "\r", "\t"):
                continue
            if not (0x20 <= codepoint <= 0x7E) and codepoint not in ALLOWED_UNICODE_CODEPOINTS:
                errors.append((lineno, colno, char, codepoint))

    if errors:
        print(f"{filename}:")
        for lineno, colno, char, codepoint in errors:
            safe_char = repr(char)[1:-1]
            print(
                f"  line {lineno}, column {colno}: U+{codepoint:04X} ({safe_char})"
            )

    if errors and fix:
        new_contents = "".join(SUBSTITUTIONS.get(ord(char), char) for char in text)
        replacements = sum(1 for char in text if ord(char) in SUBSTITUTIONS)
        filename.write_text(new_contents, encoding="utf-8")
        print(f"  fixed {replacements} replaceable character(s)")

    return bool(errors)


if __name__ == "__main__":
    sys.exit(main())
