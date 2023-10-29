+++
title = "Manage DNS in Kubernetes"

# The homepage contents
[extra]
lead = '<b>Kubizone</b> is an ecosystem of custom resources and controllers for defining and serving domain zones in a Kubernetes-native way.'
url = "/docs/getting-started/introduction/"
url_button = "Get started"
repo_version = "GitHub v0.1.0"
repo_license = "Open-source MIT License."
repo_url = "https://github.com/MathiasPius/kubizone"

# Menu items
[[extra.menu.main]]
name = "Docs"
section = "docs"
url = "/docs/getting-started/introduction/"
weight = 10

#[[extra.menu.main]]
#name = "Blog"
#section = "blog"
#url = "/blog/"
#weight = 20

[[extra.list]]
title = "Kubernetes Native"
content = 'Managed <a href="/docs/custom-resources/dnsrecord/">DNSRecords</a> and <a href="/docs/custom-resources/zone/">Zones</a> like any other Kubernetes resource.'

[[extra.list]]
title = 'Build your own Automation'
content = 'For propagating the in-cluster <a href="Zonefile">state</a> to your external DNS provider'

[[extra.list]]
title = '.. Or use the provided charts'
content = 'To serve your zone authoritatively directly from your cluster with <a href="#">CoreDNS</a>'

+++
