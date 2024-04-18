# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Bump resym_core to version 0.4.0

## [1.3.0] - 2023-07-09

### Added

- Ability to fetch Insider Preview updates for Windows 11

### Changed

- Show OS build numbers for OS updates (instead of the KB number for KB updates)

## [1.2.1] - 2023-06-24

### Added

- Command-line option to configure concurrent downloads in `windiff_cli`

### Fixed

- Improve symbol extraction efficiency

## [1.2.0] - 2023-06-18

### Added

- Syscalls extraction (IDs and names) from `ntoskrnl.exe` and `win32k.sys`

## [1.1.0] - 2023-06-17

### Added

- Syscalls extraction (IDs and names) from `ntdll.dll` and `win32u.dll`

## [1.0.0] - 2023-05-28

### Added

- Exported symbols extraction (from PEs)
- Debug symbols extraction (from PDBs)
- Modules extraction (from PDBs)
- Types extraction (from PDBs)
- Support for amd64 and arm64 PEs
