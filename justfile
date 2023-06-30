kubizone_version := `cat kubizone/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`
zonefile_version := `cat kubizone-zonefile/Cargo.toml | grep version | head -n1 | awk '{ print $3 }' | tr -d '"'`

docker-build-kubizone:    
    docker build --target kubizone -t ghcr.io/mathiaspius/kubizone/kubizone:v{{kubizone_version}}-dev .

docker-build-zonefile:
    docker build --target zonefile -t ghcr.io/mathiaspius/kubizone/zonefile:v{{zonefile_version}}-dev .

docker-build: docker-build-kubizone docker-build-zonefile

docker-publish: docker-build
    docker push ghcr.io/mathiaspius/kubizone/kubizone:v{{kubizone_version}}-dev
    docker push ghcr.io/mathiaspius/kubizone/zonefile:v{{zonefile_version}}-dev
