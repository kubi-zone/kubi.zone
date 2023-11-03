+++
title = "Zonefile Operator"
description = 'The Zonefile Operator monitors Zones for hash updates, and translates their entries fields into a ConfigMap'
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'The Zonefile Operator monitors Zones for hash updates, and translates their entries field into a ConfigMap'
toc = true
top = false
+++

## Procedure

Whenever a [Zone](../../custom-resources/zone/) referenced by a [ZoneFile](../custom-resources/zonefile/) changes,
the operator rebuilds the [RFC1035](https://datatracker.ietf.org/doc/html/rfc1035#section-5)-compatible text representation
of the zone, and creates/updates the `ConfigMap` with the new data.

