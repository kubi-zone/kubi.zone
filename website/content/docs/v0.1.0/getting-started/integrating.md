+++
title = "Integrating"
description = "Integrating Kubizone resources with your DNS provider."
date = 2023-11-09T14:00:00+01:00
updated = 2023-11-09T14:00:00+01:00
draft = false
weight = 4
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Integrating Kubizone resources with your DNS provider."
toc = true
top = false
+++

This section is only for developers who want to create an integration for Kubizone
to some currently (or poorly) unsupported provider.

The easiest way to integrate with Kubizone, is to write a program which
reads the [Zone](../../custom-resources/zone/) resources directly from the 
Kubernetes API, and pushes/applies these changes directly to the DNS provider
through an API.

If your provider supports uploading [RFC1035](https://datatracker.ietf.org/doc/html/rfc1035#section-5)-compatible
text representations of a zone, it might be easier to use the [ZoneFile](../../custom-resources/zonefile/)'s
configmaps instead.

You can retrieve a list of current records associated with a zone by reading
the `.status.entries` field of the zone, and use this list to populate the
zone as defined by your provider.

Please see [usage](../usage/) and [Zone](../../custom-resources/zone/) to get
an idea for how zones and records interact, as well as the structure of it.

## Example Provider
A very simple provider has been implemented for AWS' Route53 in bash, using
just `kubectl` and the `aws` cli.

The provider is available [here](https://github.com/kubi-zone/route53-bash-integration)
