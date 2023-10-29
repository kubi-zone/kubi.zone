+++
title = "Introduction"
description = "Quick overview of the Kubizone project"
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Quick overview of the Kubizone project"
toc = true
top = false
+++

The project is designed to be modular, to allow extension and compatibility with or by similar projects.

## Kubizone
Kubizone is the core part of the project, and consists only of a single Kubernetes controller and two
[Custom Resources](https://kubernetes.io/docs/concepts/extend-kubernetes/api-extension/custom-resources/),
namely [DNSRecords](@/docs/custom-resources/dnsrecord.md) and [Zones](@/docs/custom-resources/zone.md)

Kubizone keeps track of DNSRecords and Zones within a cluster, tracking ownership and associations
between them. Its primary responsibility is to:

1. Recompute the [hash](@/docs/custom-resources/zone.md#hash) of zones whenever their constituent records
   or zones change
   
2. Ensure that the `.status.fqdn` field of both [DNSRecords](@/docs/custom-resources/dnsrecord.md) and
[Zones](@/docs/custom-resources/zone.md) is populated and up to date.

As such, using Kubizone on its own does not add much value, unless you are prepared to write your own
integrations for reflecting these changes in your authoritative DNS service of choice.

An example implementation of such functionality is found in **Kubizone Zonefile** (see below)

## Kubizone Zonefile
Builds upon the Kubizone [DNSRecord](@/docs/custom-resources/dnsrecord.md) and
[Zone](@/docs/custom-resources/zone.md) resources, and introduces a new [Zonefile](@/docs/custom-resources/zonefile.md)
resource, which describes a way for the [Kubizone Zonefile Controller](@/docs/controllers/zonefile.md)
to map these records into an [RFC1035](https://www.rfc-editor.org/rfc/rfc1035) *zonefile*, which can be
read and understood by an authoritative DNS server such as CoreDNS.

Note that this part of the Kubizone project serves mainly as an example downstream controller implementation
and is not strictly required, unless you plan on pushing the zonefiles elsewhere, or serving them directly
from within your cluster.
