# Introduction

The goal of this PoC is to build out an architecture allowing for capture and offline checking of ABI information for ELF build artefacts.

## Definitions

### `AbiCapture` struct

The lowest abstraction level is the `AbiCapture` struct.

These structs capture the essential[1] dynamic linking related information we care about for a single ELF file as 
stand-alone data unit, including the kind of ELF object (shared object or executable) and its dynamic linking related symbols.

This means that each `AbiCapture` struct can own the underlying ELF file whilst parsing it, but still be independent of 
the underlying ELF file afterwards.

In particular, the `AbiCapture` struct contains lists of both exported and imported symbols from the .dynsym section and 
its associated string table.

Most likely, the AbiCapture data will live as separate files and be indexed separately on the binary vessel repo side,
because they are ABI artefacts and therefore outputs from the recipe + build profile (and its associated build-time
`system-model`).

[1]: "Essential" means "necessary and sufficient information for the purpose of our desired analysis capability".


### `AbiHash` struct

Every `AbiCapture` struct has an associated `AbiHash` struct. This struct shares (and uses as its key for lookups) 
either the underlying DT_SONAME string XOR the underlying executable name string. This also implies that the `AbiHash` 
struct needs to encode the kind of ELF object it is representing.

The `AbiHash` struct then hashes the `AbiCapture` struct's exported symbols to a single hash value. This works a
little like a build-id, but only for exported symbols. This exported symbol hash will come in useful for the
final `AbiReport below.

Each `AbiHash` struct can therefore only be generated _after_ the associated `AbiCapture` struct has been created.

Like the `AbiCapture` data, the `AbiHash` data will also live as separate files and be indexed separately on the binary
vessel repo side.


### `AbiReport` struct

This struct is an amalgam of `AbiCapture` and associated `AbiHash` structs from the build artefacts under analysis.

However, for each executable and shared object it "owns", it also contains copies of the `AbiHash` export hashes from each of 
the relevant build dependencies for the given executable or shared object. These `AbiHash` export hashes can then each be
resolved against their parent `AbiReport`'s `AbiHash` export hashes contents.

This means that each AerynOS recipe will have an `AbiReport` (captured on the binary side by boulder at build time and
stored in the vessel repo) that lists the impoted `AbiHash` states of each associated build dependency artefact, which was
dynamically linked against at build time.

These `AbiHash` structs will obviously need to be resolvable to each of their generating recipes, for the purposes of 
determining whether rebuilds are necessary.

It is possible that it might be expedient to also save the actual artefact version / PkgID for each build dep (next to 
the `AbiHash`), as that will make looking up the relevant `AbiReport` (specifically the `AbiCapture` part) trivial.


### `AbiReport` index

Designed to contain forward and reverse look-up functionality for recipes, their `AbiReport` structs and individual 
`AbiHash` and `AbiCapture` structs.

The idea is that:

- It should be possible to look up a symbol in a constrained set of executables and/or shared objects
- It should be possible to look up the `AbiReport`, the `AbiHash` and the `AbiCapture` for any given executable or 
shared object.


## BootStrap seeding approach

For the initial bootstrap effort, it may be necessary initially mark some impported `AbiHash` structs as "unknown".

This is because it might not be practical to introspect all .stones in the entire repo for ABI correctness "on the fly", 
because there is no real way currently to check which version of their build deps they were built against.

Q: Would it make sense to begin saving the pkgID (naïvely `name-version-sourcerelease-buildrelease` + potentially the origin repo?)     of each .stone used as a builddep in the output .stone relatively soon...?


## Proposed rebuild resolver

Once the above is in place, it should be possible to do a rebuild check pass using only `AbiReport` structs and associated 
`AbiHash` checks.

If the user has asked for it, it should now also be possible to compare exported symbols in the `AbiCapture` structs of 
each builddep with the imported symbols for each soname/executable, and then compute the diff on mismatch as evidence for why 
a rebuild is necessary.
