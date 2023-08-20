#!/usr/bin/env just

kubizone_version := `cat kubizone/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`

default:
    just --list

build:
    docker build --target kubizone -t ghcr.io/kubi-zone/kubi.zone/kubizone:dev .

publish: build
    docker push ghcr.io/kubi-zone/kubi.zone/kubizone:dev

package:
    helm package charts/kubizone --destination charts/packaged/

publish-chart: package
    helm push charts/packaged/kubizone-$(grep 'version:' charts/kubizone/Chart.yaml | awk '{printf $2}').tgz oci://registry.kronform.pius.dev/kubizone

install:
    helm -n kubizone upgrade --install  \
        --set image.tag=dev             \
        --set image.pullPolicy=Always   \
        kubizone ./charts/kubizone

dump-crds:
    cargo run --bin kubizone -- dump-crds crds

danger-recreate-crds:
    cargo run --bin kubizone -- danger-recreate-crds

