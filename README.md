# kubi.zone

`kubi.zone` is a Kubernetes ecosystem of DNS resources and controllers.

The core of `kubizone` consists of the Kubernetes Custom Resource `Record` and `Zone`, as well as the kubizone controller, which manages their relations after creation.


* [Core CRDs](/kubizone-crds/). Defines the `Zone` and `Record` Custom Resources.
* [Core Controller](/kubizone/). Populates and manages linkages between `Record`s and `Zone`s, propagating changes up through sub-zones to invalidate `Zone` hashes, which in turn can be picked up by provider-specific controllers.

Provider Implementations
* [Zonefile CRDs](/kubizone-zonefile-crds/). Defines the `ZoneFile` Custom Resource which tracks a `Zone` and its `Record`s.
* [Zonefile Controller](/kubizone-zonefile/). Monitors `ZoneFile`s (re)populating a `ConfigMap` with the  [RFC1035 Zonefile](https://datatracker.ietf.org/doc/html/rfc1035#section-5) representation of the graph.
