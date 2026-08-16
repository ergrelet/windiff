#!/usr/bin/env python3
"""Diff two WinDiff databases for the same binary across two Windows versions.

WinDiff emits one gzip-compressed JSON file per (binary, OS) pair, named
    {binary}_{version}_{update}_{architecture}.json.gz
and an index.json.gz describing which OS versions / binaries are present.

This script does the deterministic part of a version diff so the analysis can
focus on interpretation rather than set arithmetic:

  - exports / debug symbols / modules / syscalls -> added & removed (set diff)
  - syscalls                                     -> also reports renumbering
  - reconstructed types                          -> added, removed, and for
                                                    types present in both, a
                                                    line-level diff of the
                                                    struct/enum definition so
                                                    new/removed fields and flags
                                                    are visible

Output is JSON on stdout (machine-readable, feed it back into the analysis) and
a human-readable summary on stderr.

Usage:
    windiff_diff.py <db_dir> <binary> <old_os_suffix> <new_os_suffix> [--kinds ...]

    db_dir        directory containing the *.json.gz databases (and index.json.gz)
    binary        e.g. ntoskrnl.exe, ntdll.dll, win32k.sys, ci.dll
    old/new       OS path suffix "version_update_architecture",
                  e.g. 21H2_BASE_amd64  (run with --list to see what's present)

Examples:
    windiff_diff.py ./out ntoskrnl.exe 21H2_BASE_amd64 22H2_BASE_amd64
    windiff_diff.py ./out --list
    windiff_diff.py ./out ntdll.dll 21H2_BASE_amd64 22H2_BASE_amd64 --kinds syscalls types
"""
import argparse
import difflib
import gzip
import json
import os
import re
import sys

KINDS = ["exports", "symbols", "modules", "syscalls", "types"]


def load_gz_json(path):
    with gzip.open(path, "rt", encoding="utf-8") as f:
        return json.load(f)


def db_path(db_dir, binary, suffix):
    return os.path.join(db_dir, f"{binary}_{suffix}.json.gz")


def list_databases(db_dir):
    """Print the OS versions and binaries available in db_dir."""
    index_path = os.path.join(db_dir, "index.json.gz")
    if os.path.exists(index_path):
        index = load_gz_json(index_path)
        print("OS versions (suffix = version_update_architecture):", file=sys.stderr)
        for os_entry in index.get("oses", []):
            suffix = f"{os_entry['version']}_{os_entry['update']}_{os_entry['architecture']}"
            print(f"  {suffix}", file=sys.stderr)
        print("\nBinaries:", file=sys.stderr)
        for b in index.get("binaries", []):
            print(f"  {b}", file=sys.stderr)
        return
    # Fall back to scanning the directory if there's no index.
    print("No index.json.gz; database files found:", file=sys.stderr)
    for name in sorted(os.listdir(db_dir)):
        if name.endswith(".json.gz") and name != "index.json.gz":
            print(f"  {name}", file=sys.stderr)


def diff_string_set(old_list, new_list):
    old, new = set(old_list), set(new_list)
    return {
        "added": sorted(new - old),
        "removed": sorted(old - new),
        "old_count": len(old),
        "new_count": len(new),
    }


def diff_syscalls(old_map, new_map):
    """Syscalls are {id: name}. Track added/removed names and renumbering."""
    old_names = set(old_map.values())
    new_names = set(new_map.values())
    # Map name -> id for renumbering detection.
    old_by_name = {v: k for k, v in old_map.items()}
    new_by_name = {v: k for k, v in new_map.items()}
    renumbered = []
    for name in sorted(old_names & new_names):
        if old_by_name[name] != new_by_name[name]:
            renumbered.append(
                {"name": name, "old_id": old_by_name[name], "new_id": new_by_name[name]}
            )
    return {
        "added": sorted(
            [{"id": new_by_name[n], "name": n} for n in (new_names - old_names)],
            key=lambda e: int(e["id"]) if str(e["id"]).isdigit() else e["id"],
        ),
        "removed": sorted(old_names - new_names),
        "renumbered": renumbered,
        "old_count": len(old_names),
        "new_count": len(new_names),
    }


ANON_PREFIX = "_unnamed_"
# A member declaration referencing an anonymous struct/union, e.g.
#   /* 0x09d4 */ _unnamed_0x19d4 MitigationFlags2Values;
# We capture the anon type id and the member name so we can follow the
# reference across builds (the synthetic id usually changes between builds).
ANON_MEMBER_RE = re.compile(r"\b(_unnamed_0x[0-9a-fA-F]+)\s+(\w+)")
# Any reference to an anonymous type id, used to detect lines that differ
# ONLY because the synthetic id churned (pure rebuild noise).
ANON_ID_RE = re.compile(r"_unnamed_0x[0-9a-fA-F]+")


def is_anon(name):
    """resym names anonymous structs/unions _unnamed_0xNNNN; these synthetic
    ids change between builds and are diff noise, not real additions."""
    return name.startswith(ANON_PREFIX)


def _strip_anon_ids(line):
    """Normalize a member line by erasing anonymous type ids, so two lines that
    differ only by `_unnamed_0x2b2c` vs `_unnamed_0x2b3d` compare equal."""
    return ANON_ID_RE.sub("_unnamed_", line)


def _clean_member(raw):
    """Strip resym's `/* ... */` offset/size/BitPos comments and collapse spaces.

    Removing these comments is deliberate: it means a bit inserted mid-bitfield
    (which shifts every following BitPos/offset) doesn't masquerade as dozens of
    changed members — only the genuinely added/removed declaration shows up."""
    c = re.sub(r"/\*.*?\*/", "", raw)
    return re.sub(r"\s+", " ", c).strip()


def _member_lines(body):
    """Field/enumerator declaration lines of a type body (skip braces/headers)."""
    out = []
    for raw in body.splitlines():
        c = _clean_member(raw)
        if not c.strip("{}; ") or c.startswith(("struct", "union", "enum")):
            continue
        out.append(c)
    return out


def _body_member_delta(old_body, new_body):
    """Added/removed member declaration lines between two type bodies."""
    diff = difflib.unified_diff(_member_lines(old_body), _member_lines(new_body), lineterm="", n=0)
    added, removed = [], []
    for l in diff:
        if l.startswith("+") and not l.startswith("+++"):
            added.append(l[1:].strip())
        elif l.startswith("-") and not l.startswith("---"):
            removed.append(l[1:].strip())
    return [a for a in added if a], [r for r in removed if r]


def _anon_members(definition):
    """Map member_name -> anonymous type id for a struct/union definition."""
    return {name: anon_id for anon_id, name in ANON_MEMBER_RE.findall(definition)}


def resolve_anon_member_changes(old_types, new_types):
    """Follow anonymous struct/union members back to their named parent and diff
    their contents across builds.

    This is what surfaces new bitfield flags — e.g. a new mitigation bit added to
    `_EPROCESS::MitigationFlags2Values` lives inside an anonymous `_unnamed_0xNNNN`
    struct whose synthetic id changes between builds. Diffing by member name
    (not by id) recovers the real per-bit delta and attributes it to
    `<parent>::<member>` so the change is readable, not noise.

    Returns a list of {path, parent, member, old_type, new_type, added, removed}.
    """
    results = []
    common = (set(old_types) & set(new_types))
    # Walk every named (non-anonymous) parent; recurse through nested anon members.
    roots = sorted(n for n in common if not is_anon(n))

    def walk(old_def, new_def, path, seen):
        old_anon = _anon_members(old_def)
        new_anon = _anon_members(new_def)
        for member in sorted(set(old_anon) & set(new_anon)):
            oid, nid = old_anon[member], new_anon[member]
            ob, nb = old_types.get(oid), new_types.get(nid)
            if ob is None or nb is None:
                continue
            mpath = f"{path}::{member}"
            key = (oid, nid, mpath)
            if key in seen:
                continue
            seen.add(key)
            if _strip_anon_ids(ob) != _strip_anon_ids(nb):
                added, removed = _body_member_delta(ob, nb)
                if added or removed:
                    results.append(
                        {
                            "path": mpath,
                            "parent": path,
                            "member": member,
                            "old_type": oid,
                            "new_type": nid,
                            "added": added,
                            "removed": removed,
                        }
                    )
            walk(ob, nb, mpath, seen)  # nested anonymous struct/union

    for parent in roots:
        walk(old_types[parent], new_types[parent], parent, set())
    results.sort(key=lambda r: r["path"])
    return results


def diff_types(old_types, new_types, hide_anon=True):
    """Types are {name: definition_text}. Diff definitions line by line."""
    old_keys, new_keys = set(old_types), set(new_types)
    if hide_anon:
        old_keys = {k for k in old_keys if not is_anon(k)}
        new_keys = {k for k in new_keys if not is_anon(k)}
    added = sorted(new_keys - old_keys)
    removed = sorted(old_keys - new_keys)
    modified = []
    for name in sorted(old_keys & new_keys):
        old_def = old_types[name]
        new_def = new_types[name]
        if old_def == new_def:
            continue
        old_lines = old_def.splitlines()
        new_lines = new_def.splitlines()
        diff = list(difflib.unified_diff(old_lines, new_lines, lineterm="", n=1))
        added_lines = [l[1:].strip() for l in diff if l.startswith("+") and not l.startswith("+++")]
        removed_lines = [l[1:].strip() for l in diff if l.startswith("-") and not l.startswith("---")]
        added_lines = [l for l in added_lines if l]
        removed_lines = [l for l in removed_lines if l]
        # Drop pairs that differ only by an anonymous-type id (pure rebuild churn);
        # the real change, if any, is recovered by resolve_anon_member_changes().
        anon_norm_removed = {_strip_anon_ids(l) for l in removed_lines}
        added_lines = [l for l in added_lines if _strip_anon_ids(l) not in anon_norm_removed]
        anon_norm_added = {_strip_anon_ids(l) for l in
                           [l[1:].strip() for l in diff if l.startswith("+") and not l.startswith("+++")]}
        removed_lines = [l for l in removed_lines if _strip_anon_ids(l) not in anon_norm_added]
        if not added_lines and not removed_lines:
            continue
        modified.append(
            {
                "name": name,
                "added_lines": added_lines,
                "removed_lines": removed_lines,
            }
        )
    return {
        "added": added,
        "removed": removed,
        "modified": modified,
        # Anonymous bitfield/struct member changes resolved back to <parent>::<member>.
        # Always computed (independent of hide_anon) since this is the signal that
        # hiding anonymous *names* would otherwise lose.
        "resolved_member_changes": resolve_anon_member_changes(old_types, new_types),
        "old_count": len(old_keys),
        "new_count": len(new_keys),
    }


def summarize(binary, old_suffix, new_suffix, result):
    out = sys.stderr
    print(f"\n=== {binary}: {old_suffix} -> {new_suffix} ===", file=out)
    for kind in KINDS:
        if kind not in result:
            continue
        r = result[kind]
        if kind == "syscalls":
            print(
                f"  syscalls : +{len(r['added'])} new, "
                f"{len(r.get('renumbered', []))} renumbered "
                f"({r['old_count']} -> {r['new_count']})",
                file=out,
            )
            for s in r["added"]:
                print(f"             + [{s['id']}] {s['name']}", file=out)
        elif kind == "types":
            resolved = r.get("resolved_member_changes", [])
            print(
                f"  types    : +{len(r['added'])} new, -{len(r['removed'])} removed, "
                f"~{len(r['modified'])} modified, "
                f"{len(resolved)} anon-member change(s) "
                f"({r['old_count']} -> {r['new_count']})",
                file=out,
            )
            for c in resolved:
                print(f"             ~ {c['path']}", file=out)
                for a in c["added"]:
                    print(f"                 + {a}", file=out)
                for d in c["removed"]:
                    print(f"                 - {d}", file=out)
        else:
            print(
                f"  {kind:<9}: +{len(r['added'])} added, -{len(r['removed'])} removed "
                f"({r['old_count']} -> {r['new_count']})",
                file=out,
            )


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("db_dir")
    parser.add_argument("binary", nargs="?")
    parser.add_argument("old_os_suffix", nargs="?")
    parser.add_argument("new_os_suffix", nargs="?")
    parser.add_argument("--kinds", nargs="+", choices=KINDS, default=KINDS)
    parser.add_argument(
        "--include-anon",
        action="store_true",
        help="Include _unnamed_0xNNNN anonymous types (hidden by default as diff noise)",
    )
    parser.add_argument("--list", action="store_true", help="List available OS versions and binaries")
    args = parser.parse_args()

    if args.list:
        list_databases(args.db_dir)
        return 0

    if not (args.binary and args.old_os_suffix and args.new_os_suffix):
        parser.error("binary, old_os_suffix and new_os_suffix are required (or use --list)")

    old_path = db_path(args.db_dir, args.binary, args.old_os_suffix)
    new_path = db_path(args.db_dir, args.binary, args.new_os_suffix)
    for p in (old_path, new_path):
        if not os.path.exists(p):
            print(f"error: missing database file: {p}", file=sys.stderr)
            print("Run with --list to see what's available.", file=sys.stderr)
            return 1

    old_db = load_gz_json(old_path)
    new_db = load_gz_json(new_path)

    result = {
        "binary": args.binary,
        "old_os": args.old_os_suffix,
        "new_os": args.new_os_suffix,
        "old_version": old_db.get("metadata", {}).get("version"),
        "new_version": new_db.get("metadata", {}).get("version"),
    }
    for kind in args.kinds:
        if kind == "syscalls":
            result["syscalls"] = diff_syscalls(old_db.get("syscalls", {}), new_db.get("syscalls", {}))
        elif kind == "types":
            result["types"] = diff_types(
                old_db.get("types", {}), new_db.get("types", {}), hide_anon=not args.include_anon
            )
        else:
            result[kind] = diff_string_set(old_db.get(kind, []), new_db.get(kind, []))

    summarize(args.binary, args.old_os_suffix, args.new_os_suffix, result)
    json.dump(result, sys.stdout, indent=2)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
