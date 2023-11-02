#!/usr/bin/env just
kubizone_version := `cat kubizone/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`
zonefile_version := `cat kubizone-zonefile/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`

namespace := "kubizone"

default:
    just --list

@build target:
    docker build --target {{target}} -t ghcr.io/mathiaspius/kubizone/{{target}}:dev .

@publish target: (build target)
    docker push ghcr.io/mathiaspius/kubizone/{{target}}:dev

@publish-all: (publish "kubizone") (publish "zonefile")

@install zonefile="false" recreate="false":
    helm -n {{namespace}} upgrade --install          \
        --set kubizone.image.tag=dev            \
        --set zonefile.image.tag=dev            \
        --set zonefile.enable={{zonefile}}      \
        --set dangerRecreateCrds={{recreate}}   \
        --set image.pullPolicy=Always           \
        kubizone ./charts/kubizone              \
        && kubectl delete pods -n {{namespace}} --all

@uninstall:
    helm -n {{namespace}} uninstall kubizone

@test:
    kubectl -n {{namespace}} delete -f kubizone-zonefile/examples/simple-zonefile.yaml || true
    kubectl -n {{namespace}} apply -f kubizone-zonefile/examples/simple-zonefile.yaml
    #kubectl -n {{namespace}} get pods -o name | grep kubizone | xargs -n1 kubectl -n {{namespace}} delete

@clean:
    helm -n {{namespace}} uninstall kubizone || true
    kubectl -n {{namespace}} delete zones --all
    kubectl -n {{namespace}} delete records --all

@dump-crds:
    cargo run --bin kubizone-zonefile -- dump-crds crds
    cargo run --bin kubizone -- dump-crds crds

@danger-recreate-crds:
    cargo run --bin kubizone-zonefile -- danger-recreate-crds
    cargo run --bin kubizone -- danger-recreate-crds

#danger-test-coredns: danger-test helm-install-zonefile-coredns
