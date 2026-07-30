test *args:
    cargo nextest run {{ args }}

check *args:
    cargo check {{ args }}

fmt:
    nix fmt

insta *args:
    cargo insta test --review {{args}}
