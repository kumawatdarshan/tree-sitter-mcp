test *args:
    cargo nextest run {{ args }}

check *args:
    cargo check {{ args }}

fmt:
    nix fmt
