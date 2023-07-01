#!/usr/bin/env just

kubizone_version := `cat kubizone/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`
zonefile_version := `cat kubizone-zonefile/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`

default:
    just --list

docker-build-kubizone:    
    docker build --target kubizone -t ghcr.io/mathiaspius/kubizone/kubizone:dev .

docker-build-zonefile:
    docker build --target zonefile -t ghcr.io/mathiaspius/kubizone/zonefile:dev .

docker-build: docker-build-kubizone docker-build-zonefile

docker-publish: docker-build
    docker push ghcr.io/mathiaspius/kubizone/kubizone:dev
    docker push ghcr.io/mathiaspius/kubizone/zonefile:dev

helm-install:
    helm upgrade --install              \
        --set image.tag=dev             \
        --set image.pullPolicy=Always   \
        kubizone ./charts/kubizone
    
    watch kubectl get pods

update-crds:
    cargo run --bin kubizone-zonefile -- dump-crds crds
    cargo run --bin kubizone -- dump-crds crds
