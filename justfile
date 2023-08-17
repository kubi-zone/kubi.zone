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

helm-package-kubizone:
    helm package charts/kubizone --destination charts/packaged/
    
helm-package-zonefile:
    helm package charts/zonefile --destination charts/packaged/
    
helm-publish-kubizone: helm-package-kubizone
    helm push charts/packaged/kubizone-$(grep 'version:' charts/kubizone/Chart.yaml | awk '{printf $2}').tgz oci://registry.kronform.pius.dev/kubizone
    
helm-publish-zonefile: helm-package-zonefile
    helm push charts/packaged/zonefile-$(grep 'version:' charts/zonefile/Chart.yaml | awk '{printf $2}').tgz oci://registry.kronform.pius.dev/kubizone
    

helm-install-kubizone:
    helm -n kubizone upgrade --install  \
        --set image.tag=dev             \
        --set image.pullPolicy=Always   \
        kubizone ./charts/kubizone

helm-install-zonefile:
    helm -n kubizone upgrade --install  \
        --set image.tag=dev             \
        --set image.pullPolicy=Always   \
        zonefile ./charts/zonefile

helm-install-zonefile-coredns:
    helm -n kubizone upgrade --install              \
        --set zonefile.image.tag=dev                \
        --set zonefile.image.pullPolicy=Always      \
        --set "zonefiles={example}"                 \
        zonefile-coredns ./charts/zonefile-coredns

dump-crds:
    cargo run --bin kubizone-zonefile -- dump-crds crds
    cargo run --bin kubizone -- dump-crds crds

danger-recreate-crds:
    cargo run --bin kubizone-zonefile -- danger-recreate-crds
    cargo run --bin kubizone -- danger-recreate-crds

danger-test: danger-recreate-crds docker-publish helm-install-kubizone helm-install-zonefile
    kubectl -n kubizone apply -f kubizone-zonefile/examples/simple-zonefile.yaml
    kubectl -n kubizone get pods -o name | grep kubizone | xargs -n1 kubectl -n kubizone delete

danger-test-coredns: danger-test helm-install-zonefile-coredns

serve:
    cd website && zola serve --open