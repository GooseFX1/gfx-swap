set dotenv-load := true

default:
  @just --choose

build:
  anchor build

check:
  cargo check --tests --examples
  
prodcli prog +ARGS="":
  cargo run --example {{prog}} -- {{ARGS}}

cli prog +ARGS="":
  cargo run --example {{prog}} --features ci -- {{ARGS}}

keygen which:
  solana-keygen new --no-passphrase -o programs/{{which}}/key.json
  
deploy which +ARGS="":
  anchor build -p $(echo "{{which}}" | sed -e "s/\-/_/g") -- {{ARGS}}
  solana program deploy --program-id programs/{{which}}/key.json target/deploy/$(echo "{{which}}" | sed -e "s/\-/_/g").so

test +ARGS="":
  cargo test {{ARGS}} -- --nocapture

idl which:
  mkdir -p target/deploy/idl
  anchor idl parse --file programs/{{which}}/src/lib.rs > target/idl/`echo "{{which}}"| sed -e 's/^gfx-//g'`.json

idl-program-id which:
  tmp=$(mktemp)
  jq --arg address $(solana-keygen pubkey programs/{{which}}/key.json) '. + {metadata: { address: $address }}' target/idl/$(echo "{{which}}"| sed -e 's/^gfx-//g').json > "$tmp"
  mv "$tmp" target/idl/$(echo "{{which}}"| sed -e 's/^gfx-//g').json

# ======== Begin custom commands ========
