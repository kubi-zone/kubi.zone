+++
title = "Zonefile"
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

The latest version of the `ZoneFile`'s' Custom Resource Definition can be found [here](https://github.com/MathiasPius/kubizone/blob/main/crds/zonefile.kubi.zone/v1alpha1/ZoneFile.yaml)

## What is a ZoneFile?

Within the Domain Name System a [Zone file](https://en.wikipedia.org/wiki/Zone_file) is a text file that describes a DNS zone.

Within the context of Kubizone, a `ZoneFile` resource describes a way for the [Kubizone Zonefile Controller](@/docs/controllers/zonefile.md)
to produce [ConfigMaps](https://kubernetes.io/docs/concepts/configuration/configmap/) containing the zonefile representations
of the [DNSRecords](@/docs/custom-resources/dnsrecord.md) and [Zones](@/docs/custom-resources/zone.md) defined within
the cluster.

## Spec

The `ZoneFile` resource has only one required field, a `zoneRef` which indicates the zone to generate the `ConfigMap` from.

Apart from that, it also has a number of optional fields, relevant for the SOA or "Source of Authority" record of the zone:

* `history` <small>default: 10</small>

  Number of zonefile revisions to keep around in the form of ConfigMaps.
  
  The controller will create a new configmap whenever the hash of the zone changes, which can very quickly add up to a lot,
  unless pruned.
  
* `ttl` <small>default: 360</small>

  Time-to-Live. Represents how long (in seconds) recursive resolvers should keep this record in their cache.

* `refresh` <small>default: 86400</small>

  Number of seconds after which secondary name servers should query the master for the `SOA` record to detect zone changes.

* `retry` <small>default: 7200</small>
  
  Number of seconds after which secondary name servers should retry to request the serial number from the master if the
  master does not respond. It must be less than `refresh`.

* `expire` <small>default: 3600000</small>
  
  Number of seconds after which secondary name servers should stop answering request for this zone if the master does not respond.
    
  This value must be bigger than the sum of `refresh` and `retry`.

* `negativeResponseCache` <small>default: 360</small>

  Used in calculating the time to live for purposes of negative caching.
  
  Authoritative name servers take the smaller of the SOA TTL and this value to send as the SOA TTL in negative responses.

  Resolvers use the resulting SOA TTL to understand for how long they are allowed to cache a negative response.

Default values are derived from [RIPE Guidelines](https://www.ripe.net/publications/docs/ripe-203), except for `ttl` and
`negativeResponseCache` where the recommended (larger) values might cause long-lived caching of invalid or as-of-yet undefined
answers to queries, because of the *eventually consistent* way in which Kubernetes controllers operate.

## Example