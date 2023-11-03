+++
title = "Zone"
description = "A Zone represents a logical grouping of DNSRecords and sub-zones."
date = 2023-08-16T13:53:00+02:00
updated = 2023-11-03T12:59:44+01:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "A Zone represents a logical grouping of DNSRecords and sub-zones."
toc = true
top = false
+++

The latest version of the `Zone`'s Custom Resource Definition can be found [here](https://github.com/MathiasPius/kubizone/blob/main/crds/kubi.zone/v1alpha1/Zone.yaml)

## What is a Zone?

In DNS terms, a [Zone](https://en.wikipedia.org/wiki/DNS_zone) defines a subset of the DNS namespace.

Zones can either be defined as "standalone", or sub-zones of other Zone resources within the cluster.

## Examples
A zone can either represent a standalone domain with a [Fully Qualified Domain Name (FQDN)](https://en.wikipedia.org/wiki/Fully_qualified_domain_name) as in this example:
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: example-org
spec:
  # Fully qualified domain name (notice the trailing dot.)
  domainName: example.org.
  delegations:
  - zones: ["subdomain.example.org."]
    # If you want to delegate sub-zones or records
    # to specific namespaces, you can specify a namespace
    # field like below. By default, delegations implicitly
    # allow delegation to any namespace.
    # namespace: subdomain-namespace
```

--- 

Or be a sub-zone of another parent Zone using a `zoneRef`:
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: subdomain-example-org
spec:
  # This is a sub-zone of the example-org zone as defined below
  # by the zoneRef, and therefore ust not be a fully qualified
  # domain name (no trailing dot).
  domainName: subdomain
  zoneRef:
    # Name refers to the .metadata.name of the zone we created above.
    name: example-org
    # namespace: other-namespace
  delegations:
  - records:
    # In this case we don't necessarily know what the example-org zone's
    # fully qualified domain name is, it could change to `test.org.` tomorrow
    # using the @-symbol here means match whatever *our* Zone's FQDN is,
    # taking into account the FQDN of our parent, and their parents, and so on.
    - pattern: "*.@"
      # Optionally specify which types of records are allowed to be delegated
      # types: ["A", "CNAME"]
  # Delegate dev.subdomain.example.org. to the dev namespace
  - namespace: dev
    zones:
    - "dev.@"
  # Delegate staging.subdomain.example.org to the staging namespace
  - namespace: staging
    zones:
    - "staging.@"
```

---

_Or_ you can use a fully qualified domain name for your sub-zone which the [Kubizone Operator](../../operators/kubizone/) will automatically associate with the parent zone, if the delegation rules allow it:

```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: subdomain-example-org
spec:
  # Fully qualified domain name (notice the trailing dot.)
  domainName: subdomain.example.org.
  delegations:
  - records:
    - pattern: "*.subdomain.example.org."
```

## Specification
The `Zone`'s `.spec` is made up of the following fields:

### `.spec.domainName` string
Either a fully-qualified domain name such as `subdomain.example.org.` (notice the trailing dot), _or_
a partial domain name (the name is only a partial name, such as the `dev` in `dev.example.org.`) in
which case the [`.spec.zoneRef`](#spec-zoneref) field must be populated.

If using a fully qualified domain name, the [Kubizone Operator](../../operators/kubizone/) will
automatically attempt to deduce which parent [Zone](../zones/) the record belongs to, favoring
the longest matching parent domain name.

Regardless of the domain name type, the operator will respect the parent zone delegations.

Note that _either_ `.spec.zoneRef` _or_ a fully qualified `.spec.domainName` must be used.

### `.spec.zoneRef`
a `ZoneRef` object has just two fields: `.spec.zoneRef.name` and optionally `.spec.zoneRef.namespace`,
and is used to refer to a zone by its kubernetes `.metadata.name`.

If the zone exists within a different namespace than the one from which it is being referenced,
the `.spec.zoneRef.namespace` field must also be specified.

Note that _either_ `.spec.zoneRef` _or_ a fully qualified `.spec.domainName` must be used.

### `.spec.delegations`
List of rules by which records and sub-zones can be adopted by this zone.

Each rule is made up of the following _optional_ fields:

* `records` [RecordDelegation]: List of record delegations to allow.
    
    Each record delegation in turn has the fields:
    * `types`, a list of record types to delegate (`A`, `CNAME`, etc.)
    * `pattern`, a simple pattern for matching domain names.

        Allows wildcards `*` for matching entire domain name segments (the parts between dots)
        both at the beginning, middle and end of domain names.

        Can use `@` as a short-hand for this Zone's fully qualified domain name, if it is
        subject to change or unknown at the time of creation.

* `zones`: List of sub-zone FQDNs to allow delegation to.

    Allows wildcards `*` for matching entire domain name segments (the parts between dots)
    both at the beginning, middle and end of domain names.

    Can use `@` as a short-hand for this Zone's fully qualified domain name, if it is
    subject to change or unknown at the time of creation.

* `namespace`: Limit the above rules to a singular namespace.

### `.spec.ttl` u32
Set a default Time-To-Live (TTL) value across the zone, which will be used if records don't specify one
themselves.

Defaults to 360 seconds, to increase cache responsiveness.

### `.spec.refresh` u32
Number of seconds after which secondary name servers should query the master for the SOA record,
to detect zone changes. 

Defaults to the recommendation for small and stable zones: [86400 seconds (24 hours)](https://www.ripe.net/publications/docs/ripe-203).

### `.spec.retry` u32
Number of seconds after which secondary name servers should retry to request the serial number from the master if the master does not respond.
  
It must be less than `.spec.refresh`.

Defaults to the recommendation for small and stable zones: [7200 seconds (2 hours)](https://www.ripe.net/publications/docs/ripe-203).

### `.spec.expire` u32
Number of seconds after which secondary name servers should stop answering request for this zone if the master does not respond.

This value must be bigger than the _sum_ of `.spec.refresh` and `.spec.retry`.

Defaults to recommendation for small and stable zones: [3600000 seconds (1000 hours)](https://www.ripe.net/publications/docs/ripe-203).


### `.spec.negativeResponseCache` u32
Used in calculating the Time-To-Live (TTL) for purposes of _negative_ caching.
  
Authoritative name servers take the smaller of the SOA TTL and this value to send as the SOA TTL in negative responses.

Resolvers use the resulting SOA TTL to understand for how long they are allowed to cache a negative response.

Recommendation for small and stable zones: [172800 seconds (2 days)](https://www.ripe.net/publications/docs/ripe-203).

Defaults to a much lower value (360 seconds) to increase cache responsiveness and reduce failed lookups to records still being provisioned.


## Status
The Zone status contains the fully qualified domain name of the Zone, a composite list of all discovered child records and zones,
as well as a hash value of these records, which can be used by downstream providers to easily detect changes.

### `.status.entries`
Contains a composite list of all child records successfully adopted by this zone, any `NS` and related glue records (`A` and `AAAA` records) of sub-zones, as well as an `SOA` record for the zone, utilizing the variables defined in the `.spec`.

Each entry contains:
* `fqdn` string
* `type` string
* `class` string
* `ttl` u32
* `rdata` string

Changes in immediate child records, as well as changes to `NS` and related glue records (`A` and `AAAA` records) of sub-zones
causes the entries list to be re-populated.

### `.status.fqdn` string
If the zone has been defined using a fully qualified `domainName`, then `.status.fqdn` will simply reflect the `.spec.domainName`.

If not, then the [Kubizone Operator](../../operators/kubizone/) will automatically deduce the fully qualified domain name for the zone, by following and concatenating domain names of the parent zones as defined by the `zoneRef`s until a fully qualified domain name is constructed.

### `.status.hash` string
contains a hash of the zone and its constituent parts, computed based on the `.status.entries` field.

Changes to the `.status.entries` list causes the hash to be recomputed.
