+++
title = "ZoneFile"
description = "A ZoneFile describes the mapping of a Zone and its DNSRecords into a ConfigMap containing the RFC1035 representation of that zone."
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "A ZoneFile describes the mapping of a Zone and its DNSRecords into a ConfigMap containing the RFC1035 representation of that zone."
toc = true
top = false
+++

The latest version of the `ZoneFile`'s' Custom Resource Definition can be found [here](https://github.com/MathiasPius/kubizone/blob/main/crds/kubi.zone/v1alpha1/ZoneFile.yaml)

## What is a ZoneFile?

Within the Domain Name System a [Zone file](https://en.wikipedia.org/wiki/Zone_file) is a text file representation of a DNS zone.

Within the context of Kubizone, a `ZoneFile` resource describes a way for the [Zonefile Operator](../operators/zonefile/)
to produce [ConfigMaps](https://kubernetes.io/docs/concepts/configuration/configmap/) containing the text representations
of the [Records](../custom-resources/record/) and [Zones](../custom-resources/zone/) defined within the cluster.


## Examples
The following manifest instructs the [Zonefile Operator](../operators/zonefile/) to produce a `ConfigMap` describing the
[Zone](../custom-resources/zone/) named `example-org`, and by extension all [Records](../custom-resources/record/)
and sub-zones associated with it.

```yaml
apiVersion: kubi.zone/v1alpha1
kind: ZoneFile
metadata:
  name: example
spec:
  zoneRefs:
    - name: example-org
```

---

`zoneRefs` is a list of zones to include in the resulting `ConfigMap`.

Each zone gets its own file/entry in the `ConfigMap`. For example, the following `ZoneFile` definition:

```yaml
apiVersion: kubi.zone/v1alpha1
kind: ZoneFile
metadata:
  name: example-zonefile
spec:
  zoneRefs:
    - name: example-org
    - name: dev-example-org
      namespace: dev
```

Would produce a configmap with the following structure:
```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: example-zonefile
data:
  example.org.: |-
    $ORIGIN example.org.

    (...)
  dev.example.org.: |-
    $ORIGIN dev.example.org.

    (...)
```
Assuming the `example-org` refers to a zone with the fully qualified domain name `example.org.`, and `dev-example-org`
likewise represents a Zone with an FQDN of `dev.example.org.`


## Specification

### `.spec.zoneRefs`
List of `zoneRef`s to include in the `ConfigMap`.

Each `zoneRef` must include the name parameter, and optionally a `namespace` parameter if the target Zone
is in a separate namespace from this one.

### `.spec.configMapName` string
Optionally override the name of the resulting `ConfigMap`. By default, the [Zonefile Operator](../operators/zonefile/) will produce configmaps with the same name as the `ZoneFile` resource itself.

## Status
Reflects the last observed hashes and serials for each of its constituent zones, primarily for troubleshooting purposes.

### `.status.hash` string
Map of FQDNs to latest observed hash for the zone.

### `.status.serial` u32
Map of FQDNs to latest observed serial for the zone.
