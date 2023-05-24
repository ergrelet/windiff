#!/usr/bin/python3

from datetime import datetime
from typing import Any, Dict, List, Optional, Set, Tuple, TypedDict
import fire
import json
import requests
import dateparser

WINBINDEX_UPDATES_AMD64_URL = "https://winbindex.m417z.com/data/updates_last.json"

# Winbindex
WinbindexOSUpdates = Dict[str, Any]
WinbindexOSVersions = Dict[str, WinbindexOSUpdates]


# WinDiff
class WinDiffConfiguration(TypedDict):
    oses: List[Dict[str, str]]
    binaries: List[str]


def main(configuration_path: str,
         kb_date_limit: Optional[str] = None,
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
            update_windiff_config(missing_updates, json_config)
            json.dump(json_config, open(configuration_path, "w"), indent=4)
            print("Configuration file has been updated!")


def get_winbindex_available_os_versions(
        base_only: bool = True,
        kb_date_limit: Optional[datetime] = None) -> Set[Tuple[str, str, str]]:
    """
    Fetch updates from Winbindex.
    """
    response = requests.get(WINBINDEX_UPDATES_AMD64_URL)
    updates = response.json()

    return extract_winbindex_os_versions(updates, "amd64", base_only,
                                         kb_date_limit)


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
    for os_version_name, os_updates in os_versions.items():
        # "BASE" versions aren't included
        result.add((os_version_name, "BASE", architecture))
        if base_only:
            continue

        for os_update_id, os_update_info in os_updates.items():
            # Check release date if needed
            if kb_date_limit:
                release_date = dateparser.parse(os_update_info["releaseDate"])
                if release_date is not None and release_date < kb_date_limit:
                    # Release it too old, ignore
                    continue
            result.add((os_version_name, os_update_id, architecture))

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


def update_windiff_config(new_os_updates: Set[Tuple[str, str, str]],
                          json_config: WinDiffConfiguration) -> None:
    """
    Update WinDiff configuration file by adding new updates to it.
    """
    config_oses = json_config["oses"]
    config_oses.extend(
        map(
            lambda update: {
                "version": update[0],
                "update": update[1],
                "architecture": update[2]
            }, new_os_updates))


if __name__ == '__main__':
    fire.Fire(main)
