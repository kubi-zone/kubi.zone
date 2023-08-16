+++
title = "DNSRecord"
description = "A DNSRecord represents a single DNS entry within a zone."
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'A DNSRecord represents a single DNS entry within a zone'
toc = true
top = false
+++

## What is a DNS Record?
A DNS Record `.spec` is made up of:

* a `domainName`
* a `type`
* a string of `rdata`

and in cases where `domainName` does not contain a fully qualified domain name:

* also a `zoneRef`.

The `domainName` of the record can either be a fully qualified domain name such as `www.example.org.` (notice the trailing dot),
or a relative name such as `www` or `www.subdomain`, in which case a `zoneRef` must also be specified indicating the parent [Zone](@/docs/custom-resources/zone.md)
the record is relative to.

The record `type` can be any which is [supported](https://en.wikipedia.org/wiki/List_of_DNS_record_types) by the domain name system, such as `A`, `AAAA`, `CNAME`, `MX`, etc..

`rdata` contains the *value* of the record. For `A`-records, this would be your IPv4 address such as `"127.0.0.1"`, for an `MX` record,
it would be the preference followed by the exchange: `"10 mx01.example.org"`.

## Examples
A fully qualified DNS record:
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
Since no parent zone is defined, the [Kubizone controller](@/docs/controllers/kubizone.md) will attempt to deduce the parent zone based
on the `domainName`. If no zone is found which matches any of the potential parent domains (`subdomain.example.org.`, `example.org.`, `org.`),
the record will effectively be an orphan.

A relative DNS Record:
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Record
metadata:
  name: www-subdomain-example-org
spec:
  domainName: www.subdomain
  type: A
  rdata: "192.168.0.2"
  zoneRef:
    name: example-org
```
The above example references a Zone by the name `example-org` in the same namespace, which may be defined as:
```yaml
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: example-org
spec:
  domainName: example.org.
```
Applying the above and listing the records, the Kubizone controller will have deduced the FQDN as follows:

```bash
$ kubectl get records
NAME                        DOMAIN NAME     CLASS   TYPE    DATA               FQDN                         PARENT
www-subdomain-example-org   www.subdomain   IN      A       192.168.0.2        www.subdomain.example.org.   example-org.kubizone
```
