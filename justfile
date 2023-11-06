#!/usr/bin/env just
kubizone_version := `cat kubizone/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`
zonefile_version := `cat zonefile/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`

namespace := "kubizone"

default:
    just --list

@build target:
    docker build --target {{target}} -t ghcr.io/mathiaspius/kubizone/{{target}}:dev .

@publish target: (build target)
    docker push ghcr.io/mathiaspius/kubizone/{{target}}:dev

@publish-all: (publish "kubizone") (publish "zonefile")

@install zonefile="false" recreate="false":
    helm -n {{namespace}} upgrade --install     \
        --set kubizone.image.tag=dev            \
        --set zonefile.image.tag=dev            \
        --set zonefile.enable={{zonefile}}      \
        --set dangerRecreateCrds={{recreate}}   \
        --set image.pullPolicy=Always           \
        kubizone ./charts/kubizone

@uninstall:
    helm -n {{namespace}} uninstall kubizone

@test:
    kubectl -n {{namespace}} delete -f zonefile/examples/simple-zonefile.yaml || true
    kubectl -n {{namespace}} apply -f zonefile/examples/simple-zonefile.yaml
    #kubectl -n {{namespace}} get pods -o name | grep kubizone | xargs -n1 kubectl -n {{namespace}} delete

@clean:
    helm -n {{namespace}} uninstall kubizone || true
    kubectl -n {{namespace}} delete zones --all
    kubectl -n {{namespace}} delete records --all

@dump-crds:
    cargo run --bin zonefile -- dump-crds crds
    cargo run --bin kubizone -- dump-crds crds

@danger-recreate-crds:
    cargo run --bin zonefile -- danger-recreate-crds
    cargo run --bin kubizone -- danger-recreate-crds

@install-coredns action="upgrade --install":
    echo '{                                         \
        "zoneFiles": [                              \
            {                                       \
                "zonefile": "example",              \
                "zones": [                          \
                    "example.org.",                 \
                    "subdomain.example.org."        \
                ]                                   \
            }                                       \
        ]                                           \
    }' | helm -f - -n {{namespace}} {{action}}      \
        zonefile-coredns ./charts/zonefile-coredns

@docs:
    sleep 1 && xdg-open http://localhost:1111 &
    docker run --rm -it -p 1111:1111 -p 1024:1024 -v $(pwd)/website:/app \
        --workdir=/app ghcr.io/getzola/zola:v0.17.2         \
        serve --interface=0.0.0.0 --output-dir=/public


@update-timestamps:
    shopt -s globstar; for file in website/content/**/*.md; do                          \
        last_accessed="$(git log -1 --pretty="format:%ci" website/content/docs/_index.md "$file" | sed 's/ /T/' | sed 's/ //')";     \
        last_accessed="$(date --iso-8601=seconds --date=$last_accessed)";               \
        recorded_timestamp=$(rg -o 'updated\s?=\s?(.*)' -r '$1' "$file");           \
        if [[ "$last_accessed" == "$recorded_timestamp" ]]; then                        \
            echo "up to date $file";                                                    \
        else                                                                            \
            echo "updating $file timestamp from $recorded_timestamp to $last_accessed"; \
            sed -i -E "s/updated\s?=\s?.*/updated = $last_accessed/" "$file";   \
            touch -d "$last_accessed" "$file";                                          \
        fi;                                                                             \
    done


    # stat --format '%y' website/content/docs/v0.1.0/getting-started/introduction.md | date --iso-8601=seconds