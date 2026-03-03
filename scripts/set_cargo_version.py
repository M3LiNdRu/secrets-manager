#!/usr/bin/env python3

import re
import sys
from pathlib import Path


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: set_cargo_version.py <version>", file=sys.stderr)
        return 2

    version = sys.argv[1].strip()
    if not version:
        print("error: version must not be empty", file=sys.stderr)
        return 2

    cargo_toml_path = Path(__file__).resolve().parent.parent / "Cargo.toml"
    text = cargo_toml_path.read_text(encoding="utf-8")

    pattern = re.compile(r'(?m)^version\s*=\s*"[^"]+"\s*$')
    if not pattern.search(text):
        print("error: couldn't find a version = \"...\" line in Cargo.toml", file=sys.stderr)
        return 2

    new_text = pattern.sub(f'version = "{version}"', text, count=1)
    if new_text == text:
        return 0

    cargo_toml_path.write_text(new_text, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
