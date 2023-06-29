# kubi.zone

`kubi.zone` is a Kubernetes ecosystem of DNS resources and controllers.

The core of `kubizone` consists of the Kubernetes Custom Resources [Record](crds/kubi.zone/v1alpha1/records.kubi.zone.yaml) and [Zone](crds/kubi.zone/v1alpha1/zones.kubi.zone.yaml), as well as the [kubizone](kubizone/) controller, which manages their relations after creation.

