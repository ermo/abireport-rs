Managed to get a couple of decent ideas re. how to solve the "how and what do we hash" problem for individual
`AbiCapture` structs, where each `AbiCapture` struct contains the dynamic linking related information we care
about for a single ELF file.

I figure that we could create a top-level `AbiReport` struct per recipe, which will contain (references to)
`AbiCapture` structs, and which will take care of building an `AbiHash` struct per `AbiCapture` struct for
quicker rebuild graph checking.

The `AbiReport` struct can also contain reverse lookup tables for resolving symbols to sonames and resolving
forward/reverse `AbiCapture` structs <-> `AbiHash` relationships, which can only be done once all relevant
`AbiCapture` structs have been created.

With this kind of architecture, we should be able to quickly check dynamic linking related hashes per node in
the DAG, where we then referencing incoming and outgoing `AbiReport` structs and do the requisite lookups + checks.

The benefit of all this is that we effectively abstract the individual `AbiCapture` structs into their associated
`AbiHash` structs, whilst and ensuring that each can evolve independently of each other.

This in turn enables quick reverse lookups of symbols -> sonames, which can then be displayed as the reason for
rebuilds once we are far enough along in the implementation phase.
