const { ApiPromise, WsProvider, Keyring } = require('@polkadot/api');
const { CodePromise, ContractPromise } = require('@polkadot/api-contract');
const fs = require('fs');

// Configuration
const CONFIG = {
    // Rococo Contracts Node
    nodeUrl: 'wss://rococo-contracts-rpc.polkadot.io',
    // Local testnet
    // nodeUrl: 'ws://127.0.0.1:9944',
    
    // Deployment parameters
    gasLimit: 200000000000,
    storageDepositLimit: 500000000000,
    value: 1000000000000, // 1 ROC
    
    // Contract parameters
    registrationFee: 1000000000000, // 1 ROC
    platformFeeBps: 250, // 2.5%
    escrowPeriod: 86400000, // 24 hours
    challengeStake: 1000000000000, // 1 ROC
    challengePeriod: 86400000, // 24 hours
};

class ContractDeployer {
    constructor() {
        this.api = null;
        this.keyring = new Keyring({ type: 'sr25519' });
        this.deployer = null;
        this.contracts = {};
    }

    async initialize(seedPhrase) {
        console.log('🚀 Initializing deployment...');
        
        // Connect to node
        const provider = new WsProvider(CONFIG.nodeUrl);
        this.api = await ApiPromise.create({ provider });
        
        // Setup deployer account
        this.deployer = this.keyring.addFromUri(seedPhrase);
        console.log(`📝 Deployer address: ${this.deployer.address}`);
        
        // Check balance
        const { data: balance } = await this.api.query.system.account(this.deployer.address);
        console.log(`💰 Deployer balance: ${balance.free.toHuman()}`);
        
        console.log('✅ Initialization complete');
    }

    async deployContract(contractName, constructorArgs = []) {
        console.log(`\n📦 Deploying ${contractName}...`);
        
        try {
            // Load contract metadata and wasm
            const metadata = JSON.parse(fs.readFileSync(`./contracts/${contractName}/target/ink/${contractName}.json`, 'utf8'));
            const wasm = fs.readFileSync(`./contracts/${contractName}/target/ink/${contractName}.wasm`);
            
            // Create code promise
            const code = new CodePromise(this.api, metadata, wasm);
            
            // Deploy contract
            console.log(`⏳ Uploading and instantiating ${contractName}...`);
            
            const tx = code.tx.new(
                {
                    gasLimit: CONFIG.gasLimit,
                    storageDepositLimit: CONFIG.storageDepositLimit,
                    value: CONFIG.value,
                },
                ...constructorArgs
            );
            
            return new Promise((resolve, reject) => {
                tx.signAndSend(this.deployer, (result) => {
                    if (result.status.isInBlock) {
                        console.log(`⛓️  ${contractName} included in block: ${result.status.asInBlock}`);
                    } else if (result.status.isFinalized) {
                        console.log(`✅ ${contractName} finalized in block: ${result.status.asFinalized}`);
                        
                        // Find contract instantiated event
                        const instantiated = result.events.find(({ event }) =>
                            event.section === 'contracts' && event.method === 'Instantiated'
                        );
                        
                        if (instantiated) {
                            const contractAddress = instantiated.event.data.contract.toString();
                            console.log(`🏠 ${contractName} deployed at: ${contractAddress}`);
                            
                            this.contracts[contractName] = {
                                address: contractAddress,
                                metadata: metadata,
                                instance: new ContractPromise(this.api, metadata, contractAddress)
                            };
                            
                            resolve(contractAddress);
                        } else {
                            reject(new Error(`Failed to find instantiated event for ${contractName}`));
                        }
                    } else if (result.isError) {
                        reject(new Error(`Error deploying ${contractName}: ${result.dispatchError}`));
                    }
                });
            });
            
        } catch (error) {
            console.error(`❌ Error deploying ${contractName}:`, error);
            throw error;
        }
    }

    async setupCrossContractCalls() {
        console.log('\n🔗 Setting up cross-contract relationships...');
        
        try {
            // Add payment manager as authorized caller in ZK verifier
            if (this.contracts.zk_verifier && this.contracts.payment_manager) {
                console.log('⚙️  Configuring ZK verifier permissions...');
                
                const tx = this.contracts.zk_verifier.instance.tx.addValidator(
                    {
                        gasLimit: CONFIG.gasLimit,
                        storageDepositLimit: null,
                    },
                    this.contracts.payment_manager.address
                );
                
                await this.signAndSend(tx, 'ZK verifier validator setup');
            }
            
            console.log('✅ Cross-contract setup complete');
            
        } catch (error) {
            console.error('❌ Error setting up cross-contract calls:', error);
            throw error;
        }
    }

    async testDeployment() {
        console.log('\n🧪 Testing deployment...');
        
        try {
            // Test 1: Register a dataset
            if (this.contracts.dataset_registry) {
                console.log('📋 Testing dataset registration...');
                
                const registerTx = this.contracts.dataset_registry.instance.tx.registerDataset(
                    {
                        gasLimit: CONFIG.gasLimit,
                        storageDepositLimit: null,
                        value: CONFIG.registrationFee,
                    },
                    'Test Dataset',
                    'A test dataset for deployment verification',
                    new Array(32).fill(0), // embedding_root
                    new Array(32).fill(1), // metadata_hash
                    1000000000000 // price_per_query (1 ROC)
                );
                
                const datasetId = await this.signAndSend(registerTx, 'Dataset registration');
                console.log(`✅ Test dataset registered with ID: ${datasetId}`);
            }
            
            // Test 2: Create a payment
            if (this.contracts.payment_manager) {
                console.log('💳 Testing payment creation...');
                
                const paymentTx = this.contracts.payment_manager.instance.tx.createPayment(
                    {
                        gasLimit: CONFIG.gasLimit,
                        storageDepositLimit: null,
                        value: 1000000000000, // 1 ROC
                    },
                    1 // dataset_id
                );
                
                const queryId = await this.signAndSend(paymentTx, 'Payment creation');
                console.log(`✅ Test payment created with query ID: ${queryId}`);
            }
            
            // Test 3: Register verification key and submit proof
            if (this.contracts.zk_verifier) {
                console.log('🔐 Testing ZK proof submission...');
                
                // Register verification key
                const vkTx = this.contracts.zk_verifier.instance.tx.registerVerificationKey(
                    {
                        gasLimit: CONFIG.gasLimit,
                        storageDepositLimit: null,
                    },
                    [1, 2, 3, 4], // mock key data
                    'halo2'
                );
                
                const keyHash = await this.signAndSend(vkTx, 'Verification key registration');
                console.log(`✅ Verification key registered with hash: ${keyHash}`);
                
                // Submit proof
                const proofTx = this.contracts.zk_verifier.instance.tx.submitProof(
                    {
                        gasLimit: CONFIG.gasLimit,
                        storageDepositLimit: null,
                    },
                    1, // query_id
                    1, // dataset_id
                    [5, 6, 7, 8], // proof_data
                    [9, 10], // public_inputs
                    keyHash, // verification_key_hash
                    new Array(32).fill(0) // challenge_hash
                );
                
                const proofId = await this.signAndSend(proofTx, 'Proof submission');
                console.log(`✅ Test proof submitted with ID: ${proofId}`);
            }
            
            console.log('✅ All tests passed successfully!');
            
        } catch (error) {
            console.error('❌ Error during testing:', error);
            throw error;
        }
    }

    async signAndSend(tx, operation) {
        return new Promise((resolve, reject) => {
            tx.signAndSend(this.deployer, (result) => {
                if (result.status.isInBlock) {
                    console.log(`⛓️  ${operation} included in block: ${result.status.asInBlock}`);
                } else if (result.status.isFinalized) {
                    console.log(`✅ ${operation} finalized`);
                    
                    // Extract relevant data from events
                    const relevantEvent = result.events.find(({ event }) => {
                        return event.section === 'contracts' && event.method === 'ContractEmitted';
                    });
                    
                    if (relevantEvent) {
                        // Decode the event data if needed
                        resolve(relevantEvent.event.data.toString());
                    } else {
                        resolve(result.status.asFinalized.toString());
                    }
                } else if (result.isError) {
                    reject(new Error(`Error in ${operation}: ${result.dispatchError}`));
                }
            });
        });
    }

    async saveDeploymentInfo() {
        const deploymentInfo = {
            timestamp: new Date().toISOString(),
            network: CONFIG.nodeUrl,
            deployer: this.deployer.address,
            contracts: Object.keys(this.contracts).reduce((acc, name) => {
                acc[name] = {
                    address: this.contracts[name].address,
                    deployed: true
                };
                return acc;
            }, {})
        };
        
        fs.writeFileSync('./deployment/addresses.json', JSON.stringify(deploymentInfo, null, 2));
        console.log('💾 Deployment info saved to ./deployment/addresses.json');
    }

    async deployAll(seedPhrase) {
        try {
            await this.initialize(seedPhrase);
            
            // Deploy contracts in dependency order
            console.log('\n🎯 Starting contract deployment sequence...');
            
            // 1. Deploy Dataset Registry first (no dependencies)
            await this.deployContract('dataset_registry', [CONFIG.registrationFee]);
            
            // 2. Deploy ZK Verifier (needs dataset registry address)
            await this.deployContract('zk_verifier', [
                this.contracts.dataset_registry.address, // payment_manager will be updated later
                this.contracts.dataset_registry.address,
                CONFIG.challengeStake,
                CONFIG.challengePeriod
            ]);
            
            // 3. Deploy Payment Manager (needs both other contracts)
            await this.deployContract('payment_manager', [
                this.contracts.dataset_registry.address,
                this.contracts.zk_verifier.address,
                CONFIG.platformFeeBps,
                CONFIG.escrowPeriod
            ]);
            
            // Setup cross-contract relationships
            await this.setupCrossContractCalls();
            
            // Test the deployment
            await this.testDeployment();
            
            // Save deployment information
            await this.saveDeploymentInfo();
            
            console.log('\n🎉 Deployment completed successfully!');
            console.log('\n📋 Deployed Contracts:');
            Object.entries(this.contracts).forEach(([name, contract]) => {
                console.log(`   ${name}: ${contract.address}`);
            });
            
        } catch (error) {
            console.error('\n💥 Deployment failed:', error);
            process.exit(1);
        } finally {
            await this.api?.disconnect();
        }
    }
}

// CLI interface
async function main() {
    const args = process.argv.slice(2);
    
    if (args.length < 1) {
        console.log('Usage: node deploy.js <seed_phrase> [--testnet|--mainnet]');
        console.log('Example: node deploy.js "//Alice" --testnet');
        process.exit(1);
    }
    
    const seedPhrase = args[0];
    const network = args[1];
    
    // Update config based on network
    if (network === '--mainnet') {
        CONFIG.nodeUrl = 'wss://rpc.polkadot.io'; // Update to mainnet URL when available
        console.log('🚨 WARNING: Deploying to MAINNET!');
    } else if (network === '--local') {
        CONFIG.nodeUrl = 'ws://127.0.0.1:9944';
        console.log('🏠 Deploying to local testnet');
    } else {
        console.log('🧪 Deploying to Rococo testnet');
    }
    
    const deployer = new ContractDeployer();
    await deployer.deployAll(seedPhrase);
}

// Handle script execution
if (require.main === module) {
    main().catch(console.error);
}

module.exports = { ContractDeployer, CONFIG };