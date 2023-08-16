+++
title = "Zone"
description = "A Zone represents a logical grouping of independent DNSRecords"
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "A Zone represents a logical grouping of independent DNSRecords"
toc = true
top = false
+++

## What is a Zone?

In DNS terms, a [Zone](https://en.wikipedia.org/wiki/DNS_zone) defines a subset of the DNS namespace.

The same applies in Kubizone. Here, a Zone either represents a fully qualified domain name, or refers
to a parent zone of which it is a sub-zone. In the latter case, this parent can either itself
represent a fully qualified domain name, or point to yet another zone.

This `zoneRef` chain must eventually conclude in a Zone which represents a fully qualified domain.

## Examples
A zone can either represent a [Fully Qualified Domain Name](https://en.wikipedia.org/wiki/Fully_qualified_domain_name)
(FQDN) as in this example:
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: example-org
spec:
  # Fully qualified domain name (notice the trailing dot.)
  domainName: example.org.
```

Or be a sub-zone of another parent Zone:
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: subdomain-example-org
spec:
  # This is a sub-zone of the example.org. zone as defined below,
  # and therefore is not a fully qualified domain name.
  domainName: subdomain
  zoneRef:
    # Name refers to the .metadata.name of the zone we created above.
    name: example-org
```

<small>A zone which represents neither a fully qualified domain name, nor points to a parent zone is invalid.</small>

Applying the above manifests, we can query the Kubernetes API for information:

```bash
$ kubectl get zones
NAME                    DOMAIN NAME    FQDN                     HASH                  PARENT
example-org             example.org.   example.org.             7997031354861544638
subdomain-example-org   subdomain      subdomain.example.org.   7012023166823367      example-org.kubizone
```

## Status
The Zone status contains the fully qualified domain name of the Zone, as well as a hash of the zones constituent parts.

### Hash
`.status.hash` contains a hash of the zone and its constituent parts, and can be used to determine if changes have been made
to the zone that should be propagated.

The hash is computed based on DNSRecords and Zones that reference the zone.

Changes in records or subzones of subzones do not affect the hash of parent zones.

### Fully Qualified Domain Name
If the zone has been defined using a fully qualified `domainName`, then `.status.fqdn` will simply reflect the `.spec.domainName`.

If not, then the [Kubizone controller](@/docs/controllers/kubizone.md) 
will automatically deduce the fully qualified domain name for the zone, by following and concatenating domain names of the parent
zones as defined by the zoneRefs until a fully qualified domain name is constructed.

In the [Example](#examples) above, the `.status.fqdn` of the `subdomain-example-org` Zone is automatically deduced by
the controller as `subdomain.example.org.`

## Labels
In order to track associations between zones, the [Kubizone controller](@/docs/controllers/kubizone.md) applies a
`kubi.zone/parent-zone` label to sub-zones, in order to monitor changes in these that might affect the parent zones.
