# Publish a new version
# Usage: just publish <tag>
publish tag:
    @echo "Publishing version {{tag}}..."
    git checkout {{tag}}
    git submodule update --init --recursive
    cd libarchive2-sys && cargo publish
    @echo "Waiting 30 seconds for libarchive2-sys to be available on crates.io..."
    sleep 30
    cargo publish
    @echo "Successfully published {{tag}}!"
