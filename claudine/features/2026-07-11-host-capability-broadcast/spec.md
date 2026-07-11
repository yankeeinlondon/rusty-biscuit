# Host Capability Broadcast

Claudine is going to support running _jobs_ both **later** in time (think "queuing" and "scheduling") as well as across a mesh of host platforms (rather than always being executed on the current host). In order to setup for that future we will need for all "compute nodes" in the mesh to broadcast their "capabilities" so that when a compute job is triggered it can choose an appropriate host to run on.
