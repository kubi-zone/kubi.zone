+++
title = "Usage"
description = "Using the Zone and Record resources."
date = 2023-11-06T20:26:16+01:00
updated = 2023-11-06T20:08:16+01:00
draft = false
weight = 3
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Using the Zone and Record resources."
toc = true
top = false
+++

Once you have completed the [installation](../installation), you can start building out your Zones and Records,
the same way you do all your other Kubernetes Resources.

```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: example-org
  namespace: my-namespace
spec:
  domainName: example.org.
  delegations:
  - namespace: my-namespace
    records: ["*.example.org."]
```
This zone will automatically adopt any Records within its own namespace (`my-namespace`), so let's create one.

We'll start with our website's main home page:

```yaml
apiVersion: kubi.zone/v1alpha1
kind: Record
metadata:
  name: www-example-org
  namespace: my-namespace
spec:
  domainName: www.example.org.
  type: A
  rdata: "192.168.0.2"
```

We're relying on the [Kubizone Operator](../operators/kubizone/)'s ability to discern that these two are related
based solely on their fully qualified `domainName`s, and since our zone explicitly allows delegation of any sub-domain
record to this namespace, it will be automatically included.

Using `kubectl get zone -n my-namespace example-org -o yaml` should produce something similar to this:

```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: example-org
  namespace: my-namespace
spec:
  delegations:
  - namespaces: ["my-namespace"]
    records:
    - pattern: '*.example.org.'
      types: []
    zones: []
  domainName: example.org.
  expire: 3600000
  negativeResponseCache: 360
  refresh: 86400
  retry: 7200
  ttl: 360
status:
  entries:
  - class: IN
    fqdn: www.example.org.
    rdata: 192.168.0.2
    ttl: 360
    type: A
  fqdn: example.org.
  hash: "3820432254183799767"
  serial: 2023110206
```

The interesting bit is the `.status.entries` field, which now contains our record! This is the field
that the [Zonefile Operator](../operators/zonefile/) or other integrations will use to populate a text
representation of the zonefile, push the configuration to a DNS provider, or whatever else they might
want.