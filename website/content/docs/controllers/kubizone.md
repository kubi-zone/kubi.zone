+++
title = "Kubizone Controller"
description = 'The Kubizone controller owns Zones and DNSRecords, determines zone and record associations automatically, and recomputes zone hashes when necessary.'
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'The Kubizone controller owns Zones and DNSRecords, determines zone and record associations automatically, and recomputes zone hashes when necessary.'
toc = true
top = false
+++

Monitoring of Zones and DNSRecords are independent processes, as described below.

Both [Zones](@/docs/custom-resources/zone.md) and [DNSRecords](@/docs/custom-resources/dnsrecord.md)
across the entire cluster are monitored for changes.

## Zones

Whenever a Zone resource change is detected, the following process occurs:

1. Populate the zone's `.status.fqdn`.
      
   If the Zone has a fully qualified `domainName`, this field is copied directly.
   
   Otherwise, the zone's parent is retrieved by following the `zoneRef`, and the child's `.status.fqdn` is
   populated by concatenating the `domainName` of the child zone with the `.status.fqdn` of the parent. 
   If the parent zone's `.status.fqdn` is not populated, the controller will retry soon afterwards, on the
   assumption that it will be populated eventually.
   
   In the case of a sub-zone, a `kubi.zone/parent-zone` label is added to the zone resource as well, referencing the
   specific parent zone.
   
2. Compute and update zone's `.status.hash`.
   
   A new hash is calculated based on the [DNSRecords](@/docs/custom-resources/dnsrecord.md) and
   [Zones](@/docs/custom-resources/zone.md) which reference this zone, and the zone's status is patched
   with this new hash.

## DNSRecords

Whenever a change to a DNSRecord resurce is detected, the following process occurs:

1. Populate the record's `.status.fqdn`.
     
   If the DNSRecord has a fully qualified `domainName`, this field is copied directly.

   Otherwise, the record's parent is retrieved by following the `zoneRef`, and the record's `.status.fqdn` is
   populated by concatenating the `domainName` of the record with the `.status.fqdn` of the parent zone.

2. Parent zone is deduced.

   If the DNSRecord *does not* have a fully qualified `domainName`, the `.spec.zoneRef` field
   is used to determine the parent zone, and the `.status.fqdn` is populated by concatenating the
   `domainName` of the record, with the `.status.fqdn` of the parent zone.
   
   If the parent zone's `.status.fqdn` is not yet populated, the controller will retry soon afterwards,
   on the assumption that it will be populated eventually.
   
   If the DNSRecord *does* have a fully qualified domain name, then the parent zone is deduced by finding
   the longest `.status.fqdn` of all zones in the cluster that matches the `.spec.domainName` of the record.
   
   This means that a record with a fully qualified domain name like `www.subdomain.example.org.` will be adopted
   by a zone with a `.status.fqdn` of `subdomain.example.org.` *before* a zone whose `.status.fqdn` is `example.org.`.
   
   If successfully deduced, a `kubi.zone/parent-zone` label is added to the record resource as well, referencing the parent zone.
   
## Propagation
In both cases, setting the `kubi.zone/parent-zone` label on a DNSRecord or Zone signifies association with the
parent zone and will automatically trigger reconciliation of said parent, which in turn will cause the `.status.hash`
of the zone to be recomputed.
