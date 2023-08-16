+++
title = "Zone"
description = "A Zone represents a logical grouping of independent DNSRecords"
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "A Zone represents a logical grouping of independent DNSRecords"
toc = true
top = false
+++

In DNS terms, a [Zone](https://en.wikipedia.org/wiki/DNS_zone) defines a subset of the DNS namespace.

In Kubizone terms, a Zone is a resource to which a DNSRecord can associate itself.

When a new DNS Record is created, the Kubizone controller