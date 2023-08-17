+++
title = "Quick Start"
description = "Trying it out - the quick and dirty way."
date = 2023-08-16T13:53:00+02:00
updated= 2023-08-16T13:53:00+02:00
draft = false
weight = 2
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = "Trying it out - the quick and dirty way."
toc = true
top = false
+++

To demonstrate an example setup, we will:

1. Create a test namespace for the demonstration.

2. Install the [Kubizone Controller](@/docs/controllers/kubizone.md).

3. Create a [Zone](@/docs/custom-resources/zone.md) and some [DNSRecords](@/docs/custom-resources/dnsrecord.md)
   to test that the associations between them are worked out correctly by the controller.

4. Install the [Zonefile Controller](@/docs/controllers/zonefile.md).

5. Create a [ZoneFile](@/docs/custom-resources/zonefile.md), to render the zone from step 3 into a
   [zone file](https://en.wikipedia.org/wiki/Zone_file).
   
6. Delete the demonstration namespace  

## 1. Create the demonstration namespace.
I've chosen to name the namespace `kubizone` but anything will do, as long as you're consistent!
```bash
kubectl create namespace kubizone
```
   
## 2. Installing the Kubizone Controller.
Install the [Kubizone Controller](@/docs/controllers/kubizone.md) using the helm chart:
```bash
$ helm install -n kubizone kubizone-controller oci://registry.kronform.pius.dev/kubizone/kubizone --version 0.1.0
```

## 3. Create a Zone and DNS Records
We'll use `example.org` as our test domain.
```yaml
---
apiVersion: kubi.zone/v1alpha1
kind: Zone
metadata:
  name: example-org
  namespace: kubizone
spec:
  domainName: example.org.
```
Applying the above manifest, and then listing zones, we should see:
```bash
$ kubectl get zones
NAME          DOMAIN NAME    FQDN           HASH                  PARENT
example-org   example.org.   example.org.   8556445246977061536
```

Next create a single `A`-record for it:
```yaml
---
apiVersion: kubi.zone/v1alpha1
kind: Record
metadata:
  name: www-example-org
  namespace: kubizone
spec:
  domainName: www.example.org.
  type: A
  rdata: "192.168.0.2"
```
Once applied, it should appear when listing records, and if the Kubizone Controller has already
had a chance to pick it up, the parent and FQDN fields will already be populated:

```bash
$ kubectl get records
NAME              DOMAIN NAME        CLASS   TYPE   DATA          FQDN               PARENT
www-example-org   www.example.org.   IN      A      192.168.0.2   www.example.org.   example-org.kubizon
```
If `FQDN` or `PARENT` are *not* populated, give it a few seconds and check again, and make sure you've
completed the prior steps correctly!

Finally, check that the [Hash](@/docs/custom-resources/zone.md#hash) of the `example-org` zone has updated,
indicating that a new DNS Record has been adopted by the zone:

```bash
$ kubectl get zones
NAME          DOMAIN NAME    FQDN           HASH                  PARENT
example-org   example.org.   example.org.   11807449997348042864
```

## 4. Install the ZoneFile Controller.
Install the [Zonefile Controller](@/docs/controllers/zonefile.md) using the helm chart:
```bash
$ helm install -n kubizone zonefile-controller oci://registry.kronform.pius.dev/kubizone/zonefile --version 0.1.0
```
## 5. Create a ZoneFile to render the Zone contents.
[ZoneFile](@/docs/custom-resources/zonefile.md) simply reference a [Zone](@/docs/custom-resources/zone.md) to render into a [ConfigMap](https://kubernetes.io/docs/concepts/configuration/configmap/).

In our case, `example-org`:

```yaml
---
apiVersion: zonefile.kubi.zone/v1alpha1
kind: ZoneFile
metadata:
  name: example
spec:
  zoneRef:
    name: example-org
```
Assuming the [Zonefile Controller](@/docs/controllers/zonefile.md) has already picked up our ZoneFile
and produced the configmap, listing zonefiles should produce something like this:

```bash
$ kubectl get zonefiles
NAME      ZONE          SERIAL       HASH                   CONFIGMAP
example   example-org   2023081702   11807449997348042864   example-2023081702
```
Notice that the *hash* matches that of our zone.

We can check out the contents of our rendered zonefile by getting the specified CONFIGMAP:

```bash
$ kubectl get configmap example-2023081702 -o yaml
apiVersion: v1
data:
  zonefile: |
    $ORIGIN example.org.

    example.org. IN SOA ns.example.org. noc.example.org. (
        2023081702
        86400
        7200
        3600000
        360
    )

    www 360      IN    A      192.168.0.2
kind: ConfigMap
metadata:
  name: example-2023081702
  namespace: kubizone
```
<small>Some fields ommitted for brevity</small>

The relevant bit is of course the `zonefile` entry in the data section, containing a rendered zone describing
both the `example-org` [SOA](https://en.wikipedia.org/wiki/SOA_record) (Start of Authority), and our one `www` DNS Record.

## 6. Clean up our namespace.
Once satisfied with the setup, and ready to set it up for real, delete the namespace, controllers, zones and records along with it:
```bash
$ kubectl delete namespace kubizone
```

