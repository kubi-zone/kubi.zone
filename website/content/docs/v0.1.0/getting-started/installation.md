+++
title = "Installation"
description = "Installing the Kubizone Helm Chart"
date = 2023-08-16T13:53:00+02:00
updated = 2023-11-08T13:27:07+01:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Installing the Kubizone Helm Chart"
toc = true
top = false
+++

The easiest way to install Kubizone and jump right into managing zones, is to install the Helm Chart.

## Custom Resource Definitions

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

## Deploying the chart
Add the Helm repository
```bash
$ helm repo add kubizone https://charts.kubi.zone/
```

Install the chart
```bash
$ helm install -n kube-system kubizone kubizone/kubizone
```

Please see the [values.yaml](https://github.com/kubi-zone/charts/blob/main/charts/kubizone/values.yaml) file for a list of available configuration options.
