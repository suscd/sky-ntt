# This script documents the steps used to deploy the NTT bridge for SKY/USDS between Ethereum and Solana.
# For context, the working directory should be the sky NTT repo: https://github.com/suscd/sky-ntt

# Prerequisites
#
# - Solana CLI (https://solana.com/docs/intro/installation)
# - Solana Verify (https://github.com/Ellipsis-Labs/solana-verifiable-build)
# - NTT CLI (from repo)

#####################################################
# Configuration

# The rate limit to configure on each chain
export RATE_LIMIT="500000000"

# Ethereum address of the existing token being bridged
export ETH_TOKEN_ADDR='<TOKEN_ADDR>'

# The address of the pause proxy on Ethereum, which will have final authority
export ETH_PAUSE_PROXY_ADDR='<PAUSE_PROXY_ADDR>'

# List of features to pass through when building the Solana NTT program,
# mainly used to change the hardcoded addresses that get compiled in.
export SOLANA_PROGRAM_FEATURES='token-usds,mainnet'

# Name of the token to create on Solana
export TOKEN_NAME='<TOKEN_NAME>'

# An address to a metaplex token metadata compatible file, which will be
# included in the created token on Solana.
export TOKEN_METADATA_URL='<METADATA_URL>'

# Private key for ETH wallet to use for deploying contracts
#
# This is used to pay all the network costs associated with deploying everything
export ETH_PRIVATE_KEY='<PRIVATE_KEY>'

# Path to a wallet keypair to use when deploying on Solana
#
# This is used to pay all the network costs associated with deploying everything
export SOLANA_WALLET_KEYPAIR='/path/to/keypair'

# Specify the keypairs to use for addreses below, since they can be pregenerated

# Path to the keypair for the new token on Solana
export SOLANA_TOKEN_KEYPAIR='/path/to/keypair'

# Path to the keypair for the NTT program on Solana
export SOLANA_NTT_PROGRAM_KEYPAIR='/path/to/keypair'

# Path to the keypair for the governance program on Solana
# export SOLANA_GOV_PROGRAM_KEYPAIR='/path/to/keypair'

#####################################################
# Steps

# Initialize the NTT config
ntt init Mainnet -p deployment.json

#####################
# Prepare Ethereum side

ntt add-chain Ethereum --latest --mode locking --token $ETH_TOKEN_ADDR -p deployment.json --skip-verify

#####################
# Prepare Solana side

# Read the actual addresses from the keypair files
TARGET_TOKEN_ADDRESS=$(solana-keygen pubkey $SOLANA_TOKEN_KEYPAIR)
TARGET_NTT_ADDRESS=$(solana-keygen pubkey $SOLANA_NTT_PROGRAM_KEYPAIR)
TARGET_GOV_ADDRESS=$(solana-keygen pubkey $SOLANA_GOV_PROGRAM_KEYPAIR)

# Get the authority addresses we'll need
TARGET_TOKEN_AUTHORITY=$(ntt solana token-authority $TARGET_TOKEN_ADDRESS)
TARGET_GOV_AUTHORITY=$(solana find-program-derived-address $TARGET_GOV_ADDRESS string:governance)

# Build the solana programs so they are verifiable

cd solana
solana-verify build --library-name ntt -- --features $SOLANA_PROGRAM_FEATURES --no-default-features
solana-verify build --library-name wormhole_governance -- --features $SOLANA_PROGRAM_FEATURES --no-default-features

# Deploy the Solana programs

## Governance (only need single deployment for all tokens with same authority)
# solana-keygen new -o gov-buffer.json
# solana program write-buffer target/deploy/wormhole_governance.so --use-quic --buffer gov-buffer.json
# solana program deploy --buffer gov-buffer.json --program-id $SOLANA_GOV_PROGRAM_KEYPAIR
# solana program set-upgrade-authority $TARGET_GOV_ADDRESS --new-upgrade-authority $TARGET_GOV_AUTHORITY

## NTT
solana-keygen new -o ntt-buffer.json
solana program write-buffer target/deploy/ntt.so --use-quic --buffer ntt-buffer.json
solana program deploy --buffer ntt-buffer.json --program-id $SOLANA_NTT_PROGRAM_KEYPAIR
solana program set-upgrade-authority $TARGET_NTT_ADDRESS --new-upgrade-authority $TARGET_GOV_AUTHORITY

cd ..

# Create the token on Solana

spl-token create-token \
    --program-id TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb \
    --decimals 6 \
    --enable-metadata \
    --enable-freeze \
    $SOLANA_TOKEN_KEYPAIR


## Create the token metadata
spl-token initialize-metadata $TARGET_TOKEN_ADDRESS $TOKEN_NAME $TOKEN_NAME $TOKEN_METADATA_URL

## Configure the authorities on the token so that it is controlled by NTT
spl-token authorize $TARGET_TOKEN_ADDRESS mint $TARGET_TOKEN_AUTHORITY
spl-token authorize $TARGET_TOKEN_ADDRESS freeze $TARGET_GOV_AUTHORITY

# Configure NTT on Solana

ntt add-chain Solana --latest \
    --program-key $SOLANA_NTT_PROGRAM_KEYPAIR \
    --token $TARGET_TOKEN_ADDRESS \
    --payer $SOLANA_WALLET_KEYPAIR \
    -p deployment.json \
    --skip-verify

#####################
# Update the rate limits

cat deployment.json                                                                  |
    jq ".chains.Ethereum.limits.outbound = \"$RATE_LIMIT.000000000000000000\""       |
    jq ".chains.Ethereum.limits.inbound.Solana = \"$RATE_LIMIT.000000000000000000\"" |
    jq ".chains.Solana.limits.outbound = \"$RATE_LIMIT.000000\""                     |
    jq ".chains.Solana.limits.inbound.Ethereum = \"$RATE_LIMIT.000000\""             \
    > deployment.limits.json

ntt push -y -p deployment.limits.json

#####################
# Finalize Config Authority

cat deployment.limits.json                                  |
    jq ".chains.Ethereum.owner = \"$ETH_PAUSE_PROXY_ADDR\"" |
    jq ".chains.Solana.owner = \"$TARGET_GOV_AUTHORITY\""   |
    > deployment.final.json

ntt push -y -p deployment.final.json
