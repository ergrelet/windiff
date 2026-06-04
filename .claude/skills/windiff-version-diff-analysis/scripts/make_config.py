#!/usr/bin/env python3
"""Build a minimal WinDiff CLI config for diffing two Windows versions.

WinDiff's config (see windiff_cli/src/configuration.rs) is:
    { "oses": [ {version, update, architecture}, ... ],
      "binaries": { "<name>": { "extracted_information": [FLAGS...] }, ... } }

This emits such a config restricted to the OS versions and binaries you want to
compare, so the CLI run downloads only what the diff needs.

Usage:
    make_config.py --os "VERSION:UPDATE:ARCH" --os "VERSION:UPDATE:ARCH" \
                   --binary ntoskrnl.exe [--binary ntdll.dll ...] \
                   [--info EXPORTS DEBUG_SYMBOLS MODULES TYPES SYSCALLS]

Example:
    make_config.py --os "21H2:BASE:amd64" --os "11-24H2:KB5074105:amd64" \
                   --binary ntoskrnl.exe --binary ntdll.dll --binary ci.dll
"""
import argparse
import json
import sys

ALL_INFO = ["EXPORTS", "DEBUG_SYMBOLS", "MODULES", "TYPES", "SYSCALLS"]
VALID_ARCH = {"i386", "wow64", "amd64", "arm", "arm64"}


def parse_os(spec):
    parts = spec.split(":")
    if len(parts) != 3:
        sys.exit(f"error: --os must be VERSION:UPDATE:ARCH, got {spec!r}")
    version, update, arch = parts
    if arch not in VALID_ARCH:
        sys.exit(f"error: arch must be one of {sorted(VALID_ARCH)}, got {arch!r}")
    return {"version": version, "update": update, "architecture": arch}


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--os", action="append", required=True, metavar="VERSION:UPDATE:ARCH")
    parser.add_argument("--binary", action="append", required=True, metavar="NAME")
    parser.add_argument(
        "--info",
        nargs="+",
        choices=ALL_INFO,
        default=ALL_INFO,
        help="Which data kinds to extract per binary (default: all)",
    )
    args = parser.parse_args()

    if len(args.os) < 2:
        sys.exit("error: pass --os at least twice (the two versions to diff)")

    config = {
        "oses": [parse_os(s) for s in args.os],
        "binaries": {name: {"extracted_information": args.info} for name in args.binary},
    }
    json.dump(config, sys.stdout, indent=4)
    print()


if __name__ == "__main__":
    main()
