#!/bin/bash

# Set default network if not provided
NETWORK=${NETWORK:-"mainnet"}
NETWORK_UPPER="$(echo "$NETWORK" | tr '[:lower:]' '[:upper:]')"

# ------------------------------
# Function to Load Environment Variables
# ------------------------------
load_env() {
    if [ -f .env ]; then
        # Read the .env file line by line, ignoring comments and empty lines
        while IFS= read -r line || [ -n "$line" ]; do
            # Skip comments and empty lines
            [[ $line =~ ^[[:space:]]*# ]] && continue
            [[ -z "$line" ]] && continue
            # Export each variable
            export "${line?}"
        done < .env
    else
        echo "Error: .env file not found. Please create a .env file with the necessary variables."
        exit 1
    fi
}

# ------------------------------
# Clean up the .env file
# ------------------------------
cleanup_env() {
    if [ -f .env ]; then
        # First check if the WARNING line exists
        warning_line=$(grep -n "# WARNING: Everything below this line is auto-generated" .env | cut -d: -f1)

        if [ ! -z "$warning_line" ]; then
            # Find the empty line before the warning
            empty_line=$((warning_line - 1))
            # Delete from the empty line to the end of file
            sed -i '' -e "${empty_line},\$d" .env
        fi
    fi
}

# Clean up previous auto-generated content
cleanup_env

# Load initial environment
load_env

# Set and export network-specific variables
export NETWORK=$NETWORK
export RESERVED_NODES=$(eval echo "\$${NETWORK_UPPER}_RESERVED_NODES")
export RELAYER_V2_LISTENING_CONTRACTS=$(eval echo "\$${NETWORK_UPPER}_RELAYER_V2_LISTENING_CONTRACTS")
export RELAYER_DA_DEPLOY_HEIGHT=$(eval echo "\$${NETWORK_UPPER}_RELAYER_DA_DEPLOY_HEIGHT")
export RELAYER=$(eval echo "\$${NETWORK_UPPER}_RELAYER")
export SYNC_HEADER_BATCH_SIZE=$(eval echo "\$${NETWORK_UPPER}_SYNC_HEADER_BATCH_SIZE")
export RELAYER_LOG_PAGE_SIZE=$(eval echo "\$${NETWORK_UPPER}_RELAYER_LOG_PAGE_SIZE")
export CHAIN_CONFIG=$NETWORK

# Append network-specific variables to .env file
{
    echo -e "\n# WARNING: Everything below this line is auto-generated by set_env.sh"
    echo "# Network-specific variables for $NETWORK"
    echo "# Last generated: $(date)"
    echo "NETWORK=$NETWORK"
    echo "RESERVED_NODES=$RESERVED_NODES"
    echo "RELAYER_V2_LISTENING_CONTRACTS=$RELAYER_V2_LISTENING_CONTRACTS"
    echo "RELAYER_DA_DEPLOY_HEIGHT=$RELAYER_DA_DEPLOY_HEIGHT"
    echo "RELAYER=$RELAYER"
    echo "SYNC_HEADER_BATCH_SIZE=$SYNC_HEADER_BATCH_SIZE"
    echo "RELAYER_LOG_PAGE_SIZE=$RELAYER_LOG_PAGE_SIZE"
    echo "CHAIN_CONFIG=$CHAIN_CONFIG"
} >> .env
