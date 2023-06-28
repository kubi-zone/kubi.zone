# dnsetes

```mermaid
flowchart TD
    Service-- owns -->DNSRecord
    Ingress-- owns -->DNSRecord
    RecordName-- subdomain of -->ZoneName1
    DNSRecord-- has -->RecordName["fqdn in name"]
    DNSRecord-- has -->ZoneRef1
    subgraph OR1[OR]
        ZoneRef1[zoneRef]
        RecordName
    end
    ZoneRef1-- references --> DNSZone
    ZoneRef2-- references --> DNSZone
    DNSZone-- has -->ZoneName1["fqdn in name"]
    ZoneName2["fqdn in name"]
    subgraph OR2[OR]
        ZoneRef2[zoneRef]
    end
    DNSZone-- serialized into -->ZoneFile
    DNSZone-- defines -->Route53
    subgraph zonefile module
        ZoneFile
    end
    subgraph aws module
        Route53[Route53 Hosted Zone]
    end
```
