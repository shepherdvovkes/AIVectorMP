#!/bin/bash

# AIVectorMP Automated Deployment Script
# Usage: ./deploy-aivectormp.sh [local|rococo|kusama|polkadot]

set -e

# Configuration
NETWORK=${1:-local}
PROJECT_NAME="AIVectorMP"
PARACHAIN_ID=2000
BASE_PATH="/tmp/aivectormp"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."
    
    # Check Rust
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo not found. Install from https://rustup.rs/"
        exit 1
    fi
    
    # Check Node.js
    if ! command -v npm &> /dev/null; then
        log_error "Node.js/npm not found. Install from https://nodejs.org/"
        exit 1
    fi
    
    # Check cargo-contract
    if ! command -v cargo-contract &> /dev/null; then
        log_warning "cargo-contract not found. Installing..."
        cargo install cargo-contract --force
    fi
    
    # Check wasm32 target
    if ! rustup target list --installed | grep -q wasm32-unknown-unknown; then
        log_warning "wasm32-unknown-unknown target not found. Installing..."
        rustup target add wasm32-unknown-unknown
    fi
    
    # Check polkadot-js-api
    if ! command -v polkadot-js-api &> /dev/null; then
        log_warning "polkadot-js-api not found. Installing..."
        npm install -g @polkadot/api-cli
    fi
    
    log_success "Prerequisites check completed"
}

# Setup network configuration
setup_network_config() {
    log_info "Setting up network configuration for $NETWORK..."
    
    case $NETWORK in
        "local")
            RELAY_CHAIN_SPEC="rococo-local"
            WS_URL="ws://127.0.0.1:9944"
            RELAY_WS_URL="ws://127.0.0.1:9945"
            ;;
        "rococo")
            RELAY_CHAIN_SPEC="rococo"
            WS_URL="wss://aivectormp-rpc.rococo.subsocial.network"
            RELAY_WS_URL="wss://rococo-rpc.polkadot.io"
            ;;
        "kusama")
            RELAY_CHAIN_SPEC="kusama"
            WS_URL="wss://aivectormp-rpc.kusama.network"
            RELAY_WS_URL="wss://kusama-rpc.polkadot.io"
            ;;
        "polkadot")
            RELAY_CHAIN_SPEC="polkadot"
            WS_URL="wss://aivectormp-rpc.polkadot.network"
            RELAY_WS_URL="wss://rpc.polkadot.io"
            ;;
        *)
            log_error "Unknown network: $NETWORK"
            exit 1
            ;;
    esac
    
    log_success "Network configuration set for $NETWORK"
}

# Build parachain
build_parachain() {
    log_info "Building AIVectorMP parachain..."
    
    # Clean previous builds
    cargo clean
    
    # Build runtime and node
    # Some builds may not provide the optional `runtime-benchmarks` feature.
    # Attempt to build with it first and fall back to a normal release build
    # if the feature is unavailable.
    cargo build --release --features runtime-benchmarks || cargo build --release
    
    if [ ! -f "./target/release/aivectormp-node" ]; then
        log_error "Parachain build failed"
        exit 1
    fi
    
    log_success "Parachain built successfully"
}

# Build smart contracts
build_contracts() {
    log_info "Building smart contracts..."
    
    cd contracts
    
    # Build all contracts
    for contract in dataset-registry payment-manager zk-verifier oracle-connector governance-voting; do
        if [ -d "$contract" ]; then
            log_info "Building $contract contract..."
            cd $contract
            cargo contract build --release
            cd ..
        fi
    done
    
    cd ..
    log_success "All contracts built successfully"
}

# Generate chain specifications
generate_chain_specs() {
    log_info "Generating chain specifications..."
    
    # Create specs directory
    mkdir -p specs
    
    # Generate development spec
    ./target/release/aivectormp-node build-spec \
        --disable-default-bootnode \
        --chain dev > specs/dev-spec.json
    
    # Generate raw spec
    ./target/release/aivectormp-node build-spec \
        --chain specs/dev-spec.json \
        --raw \
        --disable-default-bootnode > specs/dev-spec-raw.json
    
    if [ "$NETWORK" != "local" ]; then
        # Generate network-specific specs
        ./target/release/aivectormp-node build-spec \
            --disable-default-bootnode \
            --chain $NETWORK > specs/$NETWORK-spec.json
            
        ./target/release/aivectormp-node build-spec \
            --chain specs/$NETWORK-spec.json \
            --raw \
            --disable-default-bootnode > specs/$NETWORK-spec-raw.json
            
        # Export genesis for parachain registration
        ./target/release/aivectormp-node export-genesis-state \
            --chain specs/$NETWORK-spec-raw.json > specs/$NETWORK-genesis-state
            
        ./target/release/aivectormp-node export-genesis-wasm \
            --chain specs/$NETWORK-spec-raw.json > specs/$NETWORK-genesis-wasm
    fi
    
    log_success "Chain specifications generated"
}

# Start local network
start_local_network() {
    log_info "Starting local development network..."
    
    # Kill any existing processes
    pkill -f aivectormp-node || true
    pkill -f polkadot || true
    
    # Start relay chain (Alice)
    log_info "Starting relay chain validator Alice..."
    polkadot --alice \
        --validator \
        --base-path $BASE_PATH/relay/alice \
        --chain rococo-local \
        --port 30333 \
        --ws-port 9945 \
        --rpc-port 9934 &
    
    sleep 5
    
    # Start relay chain (Bob)
    log_info "Starting relay chain validator Bob..."
    polkadot --bob \
        --validator \
        --base-path $BASE_PATH/relay/bob \
        --chain rococo-local \
        --port 30334 \
        --ws-port 9946 \
        --rpc-port 9935 \
        --bootnodes /ip4/127.0.0.1/tcp/30333/p2p/12D3KooWEyoppNCUx8Yx66oV9fJnriXwCcXwDDUA2kj6vnc6iDEp &
    
    sleep 10
    
    # Start parachain collator
    log_info "Starting parachain collator..."
    ./target/release/aivectormp-node \
        --alice \
        --collator \
        --force-authoring \
        --chain specs/dev-spec-raw.json \
        --base-path $BASE_PATH/parachain/alice \
        --port 40333 \
        --ws-port 8844 \
        --rpc-port 8833 \
        --unsafe-ws-external \
        --unsafe-rpc-external \
        --rpc-cors all \
        -- \
        --execution wasm \
        --chain rococo-local \
        --port 30343 \
        --ws-port 9977 &
    
    sleep 15
    log_success "Local development network started"
}

# Deploy smart contracts
deploy_contracts() {
    log_info "Deploying smart contracts to $NETWORK..."
    
    # Wait for node to be ready
    local max_retries=30
    local retry_count=0
    
    while [ $retry_count -lt $max_retries ]; do
        if polkadot-js-api --ws $WS_URL query.system.chain &>/dev/null; then
            break
        fi
        log_info "Waiting for node to be ready... ($((retry_count + 1))/$max_retries)"
        sleep 10
        retry_count=$((retry_count + 1))
    done
    
    if [ $retry_count -eq $max_retries ]; then
        log_error "Node not ready after $max_retries attempts"
        exit 1
    fi
    
    # Deploy contracts
    local contract_addresses=""
    
    # Dataset Registry Contract
    log_info "Deploying Dataset Registry contract..."
    cd contracts/dataset-registry
    local dataset_registry_addr=$(cargo contract instantiate \
        --constructor new \
        --args 1000000000000 \
        --suri //Alice \
        --url $WS_URL \
        --skip-confirm \
        --output-json | jq -r '.contract')
    
    if [ "$dataset_registry_addr" != "null" ] && [ -n "$dataset_registry_addr" ]; then
        log_success "Dataset Registry deployed at: $dataset_registry_addr"
        contract_addresses="DATASET_REGISTRY=$dataset_registry_addr\n"
    else
        log_error "Failed to deploy Dataset Registry contract"
    fi
    cd ../..
    
    # Payment Manager Contract
    log_info "Deploying Payment Manager contract..."
    cd contracts/payment-manager
    local payment_manager_addr=$(cargo contract instantiate \
        --constructor new \
        --args "$dataset_registry_addr" "$dataset_registry_addr" 250 86400000 \
        --suri //Alice \
        --url $WS_URL \
        --skip-confirm \
        --output-json | jq -r '.contract')
    
    if [ "$payment_manager_addr" != "null" ] && [ -n "$payment_manager_addr" ]; then
        log_success "Payment Manager deployed at: $payment_manager_addr"
        contract_addresses="${contract_addresses}PAYMENT_MANAGER=$payment_manager_addr\n"
    else
        log_error "Failed to deploy Payment Manager contract"
    fi
    cd ../..
    
    # ZK Verifier Contract
    log_info "Deploying ZK Verifier contract..."
    cd contracts/zk-verifier
    local zk_verifier_addr=$(cargo contract instantiate \
        --constructor new \
        --args "$payment_manager_addr" "$dataset_registry_addr" 1000000000000 86400000 \
        --suri //Alice \
        --url $WS_URL \
        --skip-confirm \
        --output-json | jq -r '.contract')
    
    if [ "$zk_verifier_addr" != "null" ] && [ -n "$zk_verifier_addr" ]; then
        log_success "ZK Verifier deployed at: $zk_verifier_addr"
        contract_addresses="${contract_addresses}ZK_VERIFIER=$zk_verifier_addr\n"
    else
        log_error "Failed to deploy ZK Verifier contract"
    fi
    cd ../..
    
    # Save addresses
    echo -e "$contract_addresses" > deployment/contract-addresses-$NETWORK.env
    log_success "Contract addresses saved to deployment/contract-addresses-$NETWORK.env"
}

# Test deployment
test_deployment() {
    log_info "Testing deployment..."
    
    # Source contract addresses
    source deployment/contract-addresses-$NETWORK.env
    
    # Test 1: Register a dataset
    log_info "Testing dataset registration..."
    polkadot-js-api --ws $WS_URL --seed "//Alice" \
        tx.vectorMarketplace.registerDataset \
        "Test BERT Dataset" \
        "High-quality BERT embeddings for semantic search" \
        "bert-base-uncased" \
        1000000000000 \
        0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef
    
    sleep 10
    
    # Test 2: Query dataset info
    log_info "Querying dataset information..."
    local dataset_info=$(polkadot-js-api --ws $WS_URL query.vectorMarketplace.datasets 1)
    
    if echo "$dataset_info" | grep -q "Test BERT Dataset"; then
        log_success "Dataset registration test passed"
    else
        log_warning "Dataset registration test may have failed"
    fi
    
    # Test 3: Create a query request
    log_info "Testing query request creation..."
    polkadot-js-api --ws $WS_URL --seed "//Bob" \
        tx.vectorMarketplace.createQueryRequest \
        1 \
        0xabcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890 \
        1000000000000
    
    sleep 10
    
    # Test 4: Check smart contract deployment
    if [ -n "$DATASET_REGISTRY" ]; then
        log_info "Testing smart contract interaction..."
        cargo contract call \
            --contract $DATASET_REGISTRY \
            --message get_registration_fee \
            --suri //Alice \
            --url $WS_URL \
            --dry-run
        
        if [ $? -eq 0 ]; then
            log_success "Smart contract interaction test passed"
        else
            log_warning "Smart contract interaction test failed"
        fi
    fi
    
    log_success "Deployment testing completed"
}

# Setup frontend
setup_frontend() {
    log_info "Setting up frontend application..."
    
    if [ -d "frontend" ]; then
        cd frontend
        
        # Install dependencies
        npm install
        
        # Create environment config
        cat > .env.local << EOF
REACT_APP_WS_PROVIDER=$WS_URL
REACT_APP_NETWORK=$NETWORK
REACT_APP_PARACHAIN_ID=$PARACHAIN_ID
EOF
        
        # Build frontend
        npm run build
        
        log_success "Frontend setup completed"
        log_info "To start frontend: cd frontend && npm start"
        
        cd ..
    else
        log_warning "Frontend directory not found, skipping frontend setup"
    fi
}

# Setup monitoring
setup_monitoring() {
    log_info "Setting up monitoring stack..."
    
    if [ -d "monitoring" ]; then
        cd monitoring
        
        # Start monitoring services
        docker-compose up -d
        
        log_success "Monitoring stack started"
        log_info "Grafana: http://localhost:3000 (admin/admin)"
        log_info "Prometheus: http://localhost:9090"
        
        cd ..
    else
        log_warning "Monitoring directory not found, skipping monitoring setup"
    fi
}

# Register parachain (for non-local networks)
register_parachain() {
    if [ "$NETWORK" == "local" ]; then
        return 0
    fi
    
    log_info "Registering parachain on $NETWORK..."
    
    # Check if we have the required files
    if [ ! -f "specs/$NETWORK-genesis-state" ] || [ ! -f "specs/$NETWORK-genesis-wasm" ]; then
        log_error "Genesis files not found. Run generate_chain_specs first."
        exit 1
    fi
    
    log_warning "Parachain registration requires manual steps:"
    log_info "1. Reserve ParaID on $RELAY_WS_URL using registrar.reserve()"
    log_info "2. Register parachain using registrar.register() with:"
    log_info "   - ParaID: $PARACHAIN_ID"
    log_info "   - Genesis State: $(cat specs/$NETWORK-genesis-state)"
    log_info "   - Genesis Wasm: $(cat specs/$NETWORK-genesis-wasm)"
    log_info "3. Wait for governance approval (if required)"
    
    read -p "Press Enter when parachain registration is complete..."
}

# Start collator for non-local networks
start_collator() {
    if [ "$NETWORK" == "local" ]; then
        return 0
    fi
    
    log_info "Starting collator for $NETWORK..."
    
    # Create systemd service
    sudo tee /etc/systemd/system/aivectormp-collator.service > /dev/null << EOF
[Unit]
Description=AIVectorMP Collator
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/target/release/aivectormp-node \\
    --collator \\
    --force-authoring \\
    --chain specs/$NETWORK-spec-raw.json \\
    --base-path $BASE_PATH/collator \\
    --port 30333 \\
    --ws-port 9944 \\
    --rpc-port 9933 \\
    --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \\
    --validator \\
    -- \\
    --execution wasm \\
    --chain $RELAY_CHAIN_SPEC \\
    --port 30343 \\
    --ws-port 9977
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
    
    # Enable and start service
    sudo systemctl daemon-reload
    sudo systemctl enable aivectormp-collator
    sudo systemctl start aivectormp-collator
    
    log_success "Collator service started and enabled"
    log_info "Check status: sudo systemctl status aivectormp-collator"
    log_info "View logs: sudo journalctl -u aivectormp-collator -f"
}

# Cleanup function
cleanup() {
    log_info "Cleaning up..."
    
    if [ "$NETWORK" == "local" ]; then
        pkill -f aivectormp-node || true
        pkill -f polkadot || true
        rm -rf $BASE_PATH
    fi
    
    log_success "Cleanup completed"
}

# Main deployment function
main() {
    log_info "Starting AIVectorMP deployment for $NETWORK network"
    
    # Trap cleanup on exit
    trap cleanup EXIT
    
    check_prerequisites
    setup_network_config
    build_parachain
    build_contracts
    generate_chain_specs
    
    case $NETWORK in
        "local")
            start_local_network
            deploy_contracts
            test_deployment
            setup_frontend
            setup_monitoring
            ;;
        *)
            register_parachain
            start_collator
            sleep 30  # Wait for collator to sync
            deploy_contracts
            test_deployment
            setup_frontend
            setup_monitoring
            ;;
    esac
    
    log_success "AIVectorMP deployment completed successfully!"
    
    # Display connection info
    echo ""
    echo "=== Deployment Summary ==="
    echo "Network: $NETWORK"
    echo "Parachain ID: $PARACHAIN_ID"
    echo "WebSocket URL: $WS_URL"
    echo "Contract addresses: deployment/contract-addresses-$NETWORK.env"
    
    if [ "$NETWORK" == "local" ]; then
        echo ""
        echo "=== Local Development ==="
        echo "Relay chain (Alice): ws://127.0.0.1:9945"
        echo "Parachain: ws://127.0.0.1:8844"
        echo "Frontend: http://localhost:3000"
        echo "Grafana: http://localhost:3000"
        echo ""
        echo "To stop the network: pkill -f aivectormp-node && pkill -f polkadot"
    else
        echo ""
        echo "=== Production Deployment ==="
        echo "Collator service: sudo systemctl status aivectormp-collator"
        echo "Logs: sudo journalctl -u aivectormp-collator -f"
    fi
}

# Help function
show_help() {
    echo "AIVectorMP Deployment Script"
    echo ""
    echo "Usage: $0 [NETWORK]"
    echo ""
    echo "Networks:"
    echo "  local    - Local development network (default)"
    echo "  rococo   - Rococo testnet"
    echo "  kusama   - Kusama network"
    echo "  polkadot - Polkadot network"
    echo ""
    echo "Examples:"
    echo "  $0                 # Deploy to local network"
    echo "  $0 local          # Deploy to local network"
    echo "  $0 rococo         # Deploy to Rococo testnet"
    echo "  $0 kusama         # Deploy to Kusama"
    echo ""
    echo "Prerequisites:"
    echo "  - Rust toolchain with wasm32 target"
    echo "  - Node.js and npm"
    echo "  - cargo-contract"
    echo "  - polkadot-js-api CLI"
    echo "  - Docker (for monitoring)"
    echo ""
}

# Parse command line arguments
case ${1:-} in
    -h|--help)
        show_help
        exit 0
        ;;
    ""|local|rococo|kusama|polkadot)
        main
        ;;
    *)
        log_error "Unknown network: $1"
        show_help
        exit 1
        ;;
esac