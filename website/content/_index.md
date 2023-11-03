+++
title = "Manage DNS in Kubernetes"
updated = 2023-11-03T16:18:39+01:00

# The homepage contents
[extra]
lead = '<b>Kubizone</b> is an ecosystem of custom resources and controllers for defining and serving domain zones in a Kubernetes-native way.'
url = "/docs/v0.1.0/getting-started/introduction/"
url_button = "Get started"
repo_version = "GitHub v0.1.0"
repo_license = "Open-source MIT License."
repo_url = "https://github.com/MathiasPius/kubizone"

# Menu items
[[extra.menu.main]]
name = "Docs"
section = "docs"
url = "/docs/v0.1.0/getting-started/introduction/"
weight = 10

#[[extra.menu.main]]
#name = "Blog"
#section = "blog"
#url = "/blog/"
#weight = 20

[[extra.list]]
title = "Kubernetes Native"
content = 'manage <a href="/docs/v0.1.0/custom-resources/record/">Records</a> and <a href="/docs/v0.1.0/custom-resources/zone/">Zones</a> like any other Kubernetes resource.'

[[extra.list]]
title = 'Build your own Automation'
content = 'for propagating the in-cluster <a href="/docs/v0.1.0/custom-resources/zone/#status-entries">state</a> to your external DNS provider'

[[extra.list]]
title = '.. Or use the provided charts'
content = 'to serve your zone authoritatively directly from your cluster with a <a href="/docs/v0.1.0/custom-resources/zonefile/">ZoneFile</a> and <a href="https://github.com/kubi-zone/kubi.zone/tree/main/charts/zonefile-coredns">CoreDNS</a>'

+++
