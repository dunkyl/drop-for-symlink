# Drop for Symlink

Windows shell extension to easily create symbolic links.

## Installation

TODO: Installer

## Installation from Source

```sh
cargo build
# (as admin):
RegSvr32 target/debug/drop_for_symlink.dll
```

Note that Explorer may retain handles to extension modules for arbitrarily long periods of time, preventing re-builds after its been loaded at least once. Restart Windows Explorer from the Task Manager to force it to release the handle.

## Usage

Right-click-drag one or more items from one location to another, then select `Create Symlink(s)` from the menu.

## Uninstall

```sh
# (as admin):
RegSvr32 /u target/debug/drop_for_symlink.dll
```