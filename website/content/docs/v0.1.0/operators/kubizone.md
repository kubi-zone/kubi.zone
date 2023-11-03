+++
title = "Kubizone Operator"
description = 'The Kubizone Operator owns Zones and Records, determines zone and record associations automatically, and recomputes zone hashes and serials when necessary.'
date = 2023-08-16T13:53:00+02:00
updated = 2023-11-03T15:15:08+01:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'The Kubizone Operator owns Zones and Records, determines zone and record associations automatically, and recomputes zone hashes and serials when necessary.'
toc = true
top = false
+++

Monitoring of Zones and Records are independent processes, as described below.

Both [Zones](../../custom-resources/zone/) and [Records](../../custom-resources/record/)
across the entire cluster are monitored for changes.

## Zones

Whenever a Zone resource change is detected, the following process occurs:

1. Populate the zone's `.status.fqdn`.
      
   If the Zone has a fully qualified `domainName`, this field is copied directly.
   
   If not, the zone's parent is retrieved by following its `zoneRef` after which the `.status.fqdn` is
   populated by concatenating the `domainName` of the child zone with the `.status.fqdn` of the parent. 
   If the parent zone's `.status.fqdn` is not populated, the operator will stop and requeue the update,
   on the assumption that it will be populated eventually.
   
   In the case of a sub-zone, a `kubi.zone/parent-zone` label is added to the zone resource as well, referencing the
   specific parent zone.

2. Fetch all child `Records` and compile the zone entries into a list.

3. Find all sub-zones of this one (if any), and then:

      * Copy all `NS` records pointing to the sub-zone itself into this list.
      * Copy all `A` and `AAAA` records of the sub-zone, which relate to the above `NS` records.

4. Compute a hash based on the above entries.

5. If the above `hash` has changed from the previously known value, increment the `serial`

6. Insert an `SOA` record based on the `Zone`'s `.spec` and insert it at the top of the `entries` list.

7. Patch the `Zone` with the updated `entries`, `hash` and `serial` field.

## Records

Whenever a change to a Record resource is detected, the following process occurs:

1. Populate the record's `.status.fqdn`.
   
   If the Record has a fully qualified `domainName`, this field is copied directly.

   Otherwise, the record's parent is retrieved by following its `zoneRef` after which the `.status.fqdn` is
   populated by concatenating the `domainName` of the record with the `.status.fqdn` of the parent zone.

2. Parent zone is deduced.

   * If the record *does* have a `zoneRef`, then the parent zone is found by looking up the `zoneRef` and
      validating that the zone's delegations allow adoption of this record.

   * If the record *does not* have a `zoneRef`, the parent is deduced by iterating over all zones and
      checking if any of them match the fully qualified `domainName` of the record _and_ allow delegation to the record.

      The parent zone with the longest `.status.fqdn` is chosen.
   
      This means that a record with a fully qualified domain name like `www.subdomain.example.org.` will be adopted
      by a zone with a `.status.fqdn` of `subdomain.example.org.` *before* a zone whose `.status.fqdn` is `example.org.`,
      assuming both zones have delegation rules allowing the adoption.

   If successfully deduced, a `kubi.zone/parent-zone` label is added to the record resource, referencing the parent zone.
   
## Propagation
In both cases, setting the `kubi.zone/parent-zone` label on a Record or Zone signifies association with the
parent zone and will automatically trigger reconciliation of said parent, which in turn will cause the `hash`, `serial`
and `entries` fields of the zone to be recomputed.