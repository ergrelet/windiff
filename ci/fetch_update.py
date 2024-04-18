#!/usr/bin/python3

from datetime import datetime
from typing import Any, Dict, List, Optional, Set, Tuple, TypedDict
import fire
import json
import requests
import dateparser

WINBINDEX_UPDATES_AMD64_URL = "https://winbindex.m417z.com/data/updates_last.json"
WINBINDEX_UPDATES_ARM64_URL = "https://m417z.com/winbindex-data-arm64/updates_last.json"
WINBINDEX_UPDATES_INSIDER_URL = "https://m417z.com/winbindex-data-insider/updates_last.json"

# Winbindex
WinbindexOSUpdates = Dict[str, Any]
WinbindexOSVersions = Dict[str, WinbindexOSUpdates]


# WinDiff
class WinDiffConfiguration(TypedDict):
    oses: List[Dict[str, str]]
    binaries: List[str]


def main(configuration_path: str,
         kb_date_limit: Optional[str] = None,
         replace_configuration: bool = False,
         dry_run: bool = False) -> None:
    """
    Main routine of the script.
    This script is used to update WinDiff configuration files with new updates
    fetched from Winbindex.
    """
    if kb_date_limit is not None:
        # Fetch base versions and KB updates up to a certain date
        available_os_versions = get_winbindex_available_os_versions(
            False, dateparser.parse(kb_date_limit))
    else:
        # Fetch only "base" versions
        available_os_versions = get_winbindex_available_os_versions()

    # Load "current" WinDiff configuration
    json_config = json.load(open(configuration_path))
    configured_os_versions = get_configured_os_versions(json_config)

    missing_updates = available_os_versions.difference(configured_os_versions)
    new_update_count = len(missing_updates)
    if new_update_count > 0:
        print(f"Found {new_update_count} new update(s) on Winbindex!")
        if not dry_run:
            if replace_configuration:
                replace_windiff_config(available_os_versions, json_config)
            else:
                update_windiff_config(missing_updates, json_config)
            json.dump(json_config, open(configuration_path, "w"), indent=4)
            print("Configuration file has been updated!")


def get_winbindex_available_os_versions(
        base_only: bool = True,
        kb_date_limit: Optional[datetime] = None) -> Set[Tuple[str, str, str]]:
    """
    Fetch updates from Winbindex.
    """
    updates_json_amd64 = requests.get(WINBINDEX_UPDATES_AMD64_URL).json()
    updates_json_arm64 = requests.get(WINBINDEX_UPDATES_ARM64_URL).json()
    updates_json_insider = requests.get(WINBINDEX_UPDATES_INSIDER_URL).json()

    updates_amd64 = extract_winbindex_os_versions(updates_json_amd64, "amd64",
                                                  base_only, kb_date_limit)
    updates_arm64 = extract_winbindex_os_versions(updates_json_arm64, "arm64",
                                                  base_only, kb_date_limit)
    updates_insider = extract_winbindex_os_insider_versions(
        updates_json_insider, ["amd64", "arm64"], kb_date_limit)

    return updates_amd64.union(updates_arm64).union(updates_insider)


def extract_winbindex_os_versions(
        os_versions: WinbindexOSVersions,
        architecture: str,
        base_only: bool = True,
        kb_date_limit: Optional[datetime] = None) -> Set[Tuple[str, str, str]]:
    """
    Parse Winbindex's 'updates.json' files and return the set of updates for
    all Windows versions.
    """
    result: Set[Tuple[str, str, str]] = set()
    
    # Determine "BASE" versions as they aren't included
    for os_version_name, os_updates in os_versions.items():
        result.add((os_version_name, "BASE", architecture))

        for os_update_id, os_update_info in os_updates.items():
            other_windows_versions = os_update_info.get("otherWindowsVersions")
            if other_windows_versions is not None:
                for windows_version_name in other_windows_versions:
                    result.add((windows_version_name, "BASE", architecture))

    if base_only:
        return result

    for os_version_name, os_updates in os_versions.items():
        for os_update_id, os_update_info in os_updates.items():
            # Check release date if needed
            if kb_date_limit:
                release_date = dateparser.parse(os_update_info["releaseDate"])
                if release_date is not None and release_date < kb_date_limit:
                    # Release is too old, ignore
                    continue
            result.add((os_version_name, os_update_id, architecture))

    return result


def extract_winbindex_os_insider_versions(
        os_versions: WinbindexOSVersions,
        architectures: List[str],
        date_limit: Optional[datetime] = None) -> Set[Tuple[str, str, str]]:
    """
    Parse Winbindex's insider 'updates.json' files and return the set of insider
    updates for Windows 11.
    """
    result: Set[Tuple[str, str, str]] = set()

    insider_builds = os_versions["builds"]
    for build_guid, build_info in insider_builds.items():
        if date_limit:
            release_date = datetime.fromtimestamp(build_info["created"])
            if release_date is not None and release_date < date_limit:
                # Release is too old, ignore
                continue

        build_arch = build_info["arch"]
        # Note(ergrelet): only keep Windows 11 Insider previews for now
        if build_arch in architectures and \
           build_info["title"].startswith("Windows 11 Insider Preview"):
            # Note(ergrelet): "11-Insider" is the OS name used by `windiff_cli`
            # to design Windows 11 insider preview builds
            result.add(("11-Insider", build_guid, build_arch))

    return result


def get_configured_os_versions(
        json_config: WinDiffConfiguration) -> Set[Tuple[str, str, str]]:
    """
    Return the set of updates tracked by WinDiff, given its configuration file.
    """
    config_oses = json_config["oses"]
    return set(
        map(lambda os: (os["version"], os["update"], os["architecture"]),
            config_oses))


def replace_windiff_config(os_updates: Set[Tuple[str, str, str]],
                           json_config: WinDiffConfiguration) -> None:
    """
    Update WinDiff configuration file by replacing the existing update list.
    """
    # Replace previous configuration
    json_config["oses"].clear()
    update_windiff_config(os_updates, json_config)


def update_windiff_config(new_os_updates: Set[Tuple[str, str, str]],
                          json_config: WinDiffConfiguration) -> None:
    """
    Update WinDiff configuration file by adding new updates to it.
    """
    config_oses = json_config["oses"]
    # Extend previous configuration
    config_oses.extend(
        map(
            lambda update: {
                "version": update[0],
                "update": update[1],
                "architecture": update[2]
            }, new_os_updates))


if __name__ == '__main__':
    fire.Fire(main)
