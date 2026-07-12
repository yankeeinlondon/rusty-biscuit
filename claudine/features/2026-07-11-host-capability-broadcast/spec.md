# Host Capability Broadcast

Claudine is going to support running _jobs_ both **later** in time (think "queuing" and "scheduling") as well as across a mesh of host platforms (rather than always being executed on the current host). In order to setup for that future we will need for all "compute nodes" in the mesh to broadcast their "capabilities" and "characteristics" so that when a compute job is triggered it can choose an appropriate host to run on.

## Capabilities and Characteristics

In this section we will detail out what is expressed a capability or characteristic of a host (referred to as just "capabilities" going forward):

### Hardware Detection

All of the below are detectable using the sniff library:

- `os`: enum(macOs,Linux,Windows) 
- `os_version`
- `memory`: number
    - the amount of RAM the machine has
- `cpu_cores`: number
- `gpu`: enum(none,metal,nvidia,other)
- `gpu_features`: features[]
- `machine`: enum(bare-metal,virtual-machine,lxc-container)
- `arch`: enum(amd64,arm64,etc.)
- `avx`: boolean
- `avx2`: boolean
- `avx512bw`: boolean
- `avx512f`: boolean
- `avx512vl`: boolean
- `neon`: boolean
- `sse`: boolean
- `sse2`: boolean
- `sse3`: boolean
- `sse4_1`: boolean
- `sse4_2`: boolean
- `ssse3`: boolean
- `available_storage`: number

### Other

- `id`: string
    - a immutable and unique identifier on the network
- `name`: string
    - a unique (for the mesh) name that is typically the hostname
    - unlike `id` a `name` is allowed to be changed so long as it maintains a name that is unique

- `repos`: "{ '<string>': string }"
    - a dictionary of repo's which the machine has already checked out and the last commit of that repo (keys are repo, values are last commit hash)
    - this is important because the latency of working a repo that is already on local storage 
    - repo should be represented in a canonical manner which uniquely the remote host

- `online`: boolean


## State Storage

The host capabilities are recorded as a CRDT document (one per host). Each capability document is a living record of a host's capabilities. Therefore the document's name should include the immutable `id` property for the host: `capability-${id}` 

- all **Rendezvous** daemon's will have visibility into _all_ hosts in the mesh
- only the  **Rendezvous** daemon on a given host will _write_ to the CRDT document for that host's capabilities
    - this will be enforced by the daemon but is also a design note for the CRDT document

All of the capabilities and characteristics will have a relatively low update cadence so while it's true that any given daemon is only "eventually consistent" with it's peers capabilities, it's likely that 99% of the time their view of other hosts capabilities is perfectly in sync.

> QUESTION: should we include `online` as a property of the capabilities document or some other document? Its update cadence is likely to be more frequent. Leaning toward YES.

> QUESTION: is there value in moving the capabilities data from **redb** database to **duckdb**? If so, is this current state info being updated or some aggregated metric

> QUESTION: if we wanted to measure a host's uptime over a 24 hour window and have that single metric be recorded in **duckdb**, is this an easy thing to do from this document structure (note: assuming the addition of the )
