+++
title = "Zonefile Controller"
description = 'The Zonefile controller monitors Zones for hash updates, and translates their constituent DNSRecords in a ConfigMap'
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = 'The Zonefile controller monitors Zones for hash updates, and translates their constituent DNSRecords in a ConfigMap'
toc = true
top = false
+++

## Procedure

Whenever a [Zone](@/docs/custom-resources/zone.md) referenced by a [ZoneFile](@/docs/custom-resources/zonefile.md) changes,
the controller compares the `.status.hash` of the ZoneFile to that of the referenced `Zone`.

If the two hashes do not match, the ZoneFile is considered invalidated, and a new serial is generated according to
[RFC 1912](https://datatracker.ietf.org/doc/html/rfc1912#section-2.2).
 
Next, all DNSRecords in the cluster, whose `kubi.zone/parent-zone` label matches that referenced by the ZoneFile, are 
serialized into a ConfigMap named by concatenating the ZoneFile's `.metadata.name` with the computed serial.

Finally, the computed serial, the last observed hash from the updated zone, as well as the name of the generated configmap
is patched into the ZoneFile's status object as `.status.serial`, `.status.hash`, and `.status.configMap` respectively.
