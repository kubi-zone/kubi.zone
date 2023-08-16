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

### Kubizone
Kubizone is the core part of the project, and consists only of a single Kubernetes controller and two
[Custom Resources](https://kubernetes.io/docs/concepts/extend-kubernetes/api-extension/custom-resources/),
namely [DNSRecord](#)s and [Zone](#)s

Kubizone keeps track of DNSRecords and Zones within a cluster, keeping track of ownership and associations
between them, and invalidating the [hash](#) of a zone, if its constituent DNSRecords change.

### Kubizone Zonefile
Separately from the Kubizone core project is the Kubizone Zonefile controller and its associated Custom
Resource, the [Zonefile](#).
