# kubi.zone
Kubernetes ecosystem of DNS resources and controllers.

The core of `kubi.zone` consists of the Kubernetes Custom Resources [Record](crds/kubi.zone/v1alpha1/records.kubi.zone.yaml) and [Zone](crds/kubi.zone/v1alpha1/zones.kubi.zone.yaml), as well as the [kubizone](kubizone/) controller, which manages their relations after creation.

This projects also contains the [kubizone-zonefile](kubizone-zonefile/) controller, which produces [RFC1035](https://datatracker.ietf.org/doc/html/rfc1035#section-5) zonefiles from a `Zone`, as an example controller which consumes `kubi.zone` resources.
