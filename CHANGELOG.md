# Change Log

All notable changes to this project will be documented in this file.

##  [0.1.x] - 2024-09-26

### Breaking Changes

- The `Arena` no longer exposes clients array to users of the Arena, instead
  the `Arena` now exposes a `clients` method that returns a reference to the
  clients array.

## [0.1.x] - 2024-09-15

### Added

Adds the ability to view local game on a global server live

## [0.1.9] - 2024-09-14

### Added

Pins a specific version of tungstenite to avoid breaking changes,
clients should can the  `tungstenite` version exposed by the latest `splendor_arena` crate

 
## [0.1.8] - 2024-09-03
 
### Added

- Added the ability to git the frontend from either `static_files/` or `splendor/`  

## [0.1.7] - 2024-09-03
 
### Fixes

- Fixed a bug where the static files were not being served correctly 

## [0.1.6] - 2024-09-03
 
### Breaking Changes

- Interface changed for public facing `Arena::launch` method, which
now accepts a no paramters, instead, use the `Arena::ArenaBuilder` to
construct the `Arena` object with the desired configuration.

### Other Changes

- The ability to specify which path has static files has been added
to `Arena::ArenaBuilder` with the `static_files` method.
 
## [0.1.4] - 2024-09-01 
 
 
### Breaking Changes

- Interface changed for public facing `Arena::launch` method, which
now accepts a new parameter to optionally specify the location of a python 3
interpreter.
 
