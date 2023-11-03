+++
title = "Record"
description = "A Record represents a single DNS entry within a zone."
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A Record represents a single DNS entry within a zone'
toc = true
top = false
+++

The latest version of the `Record`'s Custom Resource Definition can be found [here](https://github.com/MathiasPius/kubizone/blob/main/crds/kubi.zone/v1alpha1/Record.yaml)

## What is a DNS Record?
A [DNS Record](https://en.wikipedia.org/wiki/Domain_Name_System#Resource_records) is a single named entry within a zone.

## Examples
**A fully qualified DNS record:**
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Record
metadata:
  name: www-subdomain-example-org
spec:
  domainName: www.subdomain.example.org.
  type: A
  rdata: "192.168.0.2"
```
On its own this `Record` won't be very useful, but assuming a [Zone](../zones/) matching the record's
fully qualified `domainName`, which allows delegation of this domain name exists, the record will be
incorporated into it, and from there be consumed by downstream providers.

---

**A partial record, referencing a parent zone by name:**
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Record
metadata:
  name: www-subdomain-root
spec:
  domainName: www.subdomain
  zoneRef:
    name: root-zone
  type: A
  rdata: "192.168.0.1s"
```
Using partial domain names and a `zoneRef` instead of fully qualified ones, allows you to change
the top-level domain without editing the `domainName` across all records, or to define records
without knowledge of the super-zone. This is useful for deployments of application where only
the relative subdomain is relevant, such as `static.<mydomain>` or `blog.<top-domain>`.

---
Both of the above examples assumes the presence of a zone _like_ the one defined below. Note that the zone explicitly allows delegation of the `www.subdomain` domain
to _any_ record by not imposing limits on the namespace, class or type of the record.
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: root-zone
spec:
  domainName: example.org.
  delegations:
  - records:
    - pattern: ["*.subdomain.example.org."]
```

## Specification
The `Record`'s `.spec` is made up of the following fields:
### `.spec.domainName`
Either a fully-qualified domain name such as `www.example.org.` (notice the trailing dot), _or_
a partial domain name (the name is only a partial name, such as the `www` in `www.example.org.`) in
which case the `.spec.zoneRef` field must be populated.

If using a fully qualified domain name, the [Kubizone operator](../../operators/kubizone/) will
automatically attempt to deduce which parent [Zone](../zones/) the record belongs to, favoring
the longest matching parent domain name.

Regardless of the domain name type, the operator will respect the parent zone delegations.

### `.spec.type`
Type of record. See a list [here](https://en.wikipedia.org/wiki/List_of_DNS_record_types) for examples.

No validation is performed on this field, limitations are only imposed by downstream providers.

### `.spec.rdata`
Contents of the record. In the case of `A` records, this will be the IP address.

In the case of an `NS` record, this will be the hostname of the nameserver.

For `MX` it will be the preference and exchange expressed as a string, e.g.: `10 mail.protonmail.ch.`

### `.spec.ttl`
Time-to-live for the record. If none is set the parent zone's default will be used, which in turn defaults to `360` seconds.

### `.spec.class`
Can also be set, but defaults to `IN`.


## Status
The record status only contains the fully qualified domain name of the record.

### `.status.fqdn`
If the zone has been defined using a fully qualified `domainName`, then `.status.fqdn` will simply reflect the `.spec.domainName`.

If not, then the [Kubizone Operator](../../operators/kubizone/) will automatically deduce the fully qualified domain name for the record, by following and concatenating domain names of the parent zones as defined by the `zoneRef`s until a fully qualified domain name is constructed.