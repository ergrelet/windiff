# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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