# Change Log

All notable changes to this project will be documented in this file.
 
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
 
