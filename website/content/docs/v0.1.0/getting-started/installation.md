+++
title = "Installation"
description = "Installing the Kubizone Helm Chart"
date = 2023-08-16T13:53:00+02:00
updated = 2023-11-06T20:08:16+01:00
draft = false
weight = 1
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Installing the Kubizone Helm Chart"
toc = true
top = false
+++

The easiest way to install Kubizone and jump right into managing zones, is to install the Helm Chart.

Because of the lack of real support for managing Custom Resource Definitions using Helm Charts,
you will have to install these manually first:

```bash
$ kubectl apply -f https://raw.githubusercontent.com/kubi-zone/kubi.zone/main/crds/kubi.zone/v1alpha1/Record.yaml
$ kubectl apply -f https://raw.githubusercontent.com/kubi-zone/kubi.zone/main/crds/kubi.zone/v1alpha1/Zone.yaml
```

If you want to make use of the [Zonefile Operator](../operators/zonefile) to generate zonefile ConfigMaps,
you also have to install the `ZoneFile` CRD:
```bash
$ kubectl apply - f https://raw.githubusercontent.com/kubi-zone/kubi.zone/main/crds/kubi.zone/v1alpha1/ZoneFile.yaml
```


Next, install the chart located [here](https://github.com/kubi-zone/kubi.zone/blob/main/charts/kubizone).

**Note** Working on automatically packaging the charts so they can be installed directly, without git cloning.

Please see the [values.yaml](https://github.com/kubi-zone/kubi.zone/blob/main/charts/kubizone/values.yaml) file
for the chart, for a list of available configuration options.






Kubizone delivers Kubernetes-native Domain Name System (DNS) primitives such as 
[Records](../../custom-resources/record/) and [Zones](../../custom-resources/zone/),
and employs an operator to ensure that associations between Records and Zones
are kept up to date, and that Zones' [Delegation](../../custom-resources/zone/#spec-delegations) rules
are respected.

## Purpose
Kubizone aims to be a more flexible and versatile replacement for projects such as
[external-dns](https://github.com/kubernetes-sigs/external-dns).

Instead of building providers into Kubizone itself like `external-dns` does, Kubizone instead
exposes a very simple interface in the form of [Records](../../custom-resources/record/) and
[Zones](../../custom-resources/zone/), which custom providers in turn can use to update external
DNS provider state.

These custom resources makes Kubizone a lot more powerful than `external-dns`, which by its
design is limited to exposing `Service` and `Ingress` resources via `A` and `AAAA` records, allowing
Kubizone to represent all kinds of DNS Records, including but not limited to: `SRV`, `NS`, `CNAME`, and so on.

## Overview
By default, Kubizone deploys the following [Custom Resources](../../custom-resources/):

1. [Record](../../custom-resources/record/), represents a single DNS Record entry, `A`, `AAAA`, `CNAME`, what have you.

2. [Zone](../../custom-resources/zone/) can be a top-level domain or even a subdomain of another.

3. [ZoneFile](../../custom-resources/zonefile/) is used by the [ZoneFile Operator](../../operators/zonefile/) to produce
   [RFC1035](https://www.rfc-editor.org/rfc/rfc1035)-compatible zonefile `ConfigMaps` which can be used with `bind` servers such as `CoreDNS`.

And the following Operators:

1. [Kubizone Operator](../../operators/kubizone/) monitors all [Records](../../custom-resources/record/)
   and [Zones](../../custom-resources/zone/) cluster-wide, keeping track of associations between them,
   and enriching them with all the information necessary for downstream providers to maintain
   external state, such as serials, zone hashes, compiled list of child records, etc.

2. [Zonefile Operator](../../operators/zonefile/) monitors [Zonefiles](../../custom-resources/zonefile/)
   and [Zones](../../custom-resources/zone/), compiling them into [RFC1035](https://www.rfc-editor.org/rfc/rfc1035)-compatible
   `ConfigMaps` which can then either be mounted directly into a `CoreDNS` (or similar) container and served,
   or pushed to an external DNS provider.

   This operator also acts as an example of a downstream provider, whose only interaction with the rest of the Kubizone project
   is as a passive observer of zone changes.

## Developing
If you're interested in building a custom provider integration for Kubizone, take a look at the [Custom Resources](../../custom-resources/)
and get to work!

Because of the architecture of Kubizone, your controller doesn't need to coordinate or embed itself in
Kubizone to be useful.

Instead, independent providers can simply run -- inside or outside -- of the cluster, watching for zone or
record changes, and acting accordingly.
In fact, this design allows **multiple** providers to co-exist, meaning you can easily mirror
your entire configuration, allowing you to easily fail over to a secondary authority in case of an outage.
