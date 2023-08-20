#!/usr/bin/env just

kubizone_version := `cat kubizone/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`
zonefile_version := `cat kubizone-zonefile/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`

default:
    just --list

docker-build:
    docker build --target kubizone -t ghcr.io/mathiaspius/kubizone/kubizone:dev .

docker-publish: docker-build
    docker push ghcr.io/kubi-zone/kubizone/kubizone:dev
    docker push ghcr.io/mathiaspius/kubizone/zonefile:dev

helm-package-kubizone:
    helm package charts/kubizone --destination charts/packaged/

helm-publish-kubizone: helm-package-kubizone
    helm push charts/packaged/kubizone-$(grep 'version:' charts/kubizone/Chart.yaml | awk '{printf $2}').tgz oci://registry.kronform.pius.dev/kubizone

helm-install-kubizone:
    helm -n kubizone upgrade --install  \
        --set image.tag=dev             \
        --set image.pullPolicy=Always   \
        kubizone ./charts/kubizone

dump-crds:
    cargo run --bin kubizone -- dump-crds crds

danger-recreate-crds:
    cargo run --bin kubizone -- danger-recreate-crds

