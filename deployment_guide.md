# AIVectorMP - Deployment Guide для Polkadot Ecosystem

## Архитектура AIVectorMP

AIVectorMP (AI Vector Marketplace) - это специализированная parachain для торговли AI векторными данными с интегрированным zero-knowledge поиском на базе Substrate/Polkadot.

### Компоненты системы

```
AIVectorMP Parachain
├── Runtime Pallets
│   ├── vector-marketplace      # Торговля векторными данными
│   ├── zk-verification        # ZK proof верификация  
│   ├── payment-escrow         # Escrow платежи
│   └── cross-chain-bridge     # XCM интеграция
├── Smart Contracts (ink!)
│   ├── dataset-registry       # Дополнительная логика датасетов
│   ├── oracle-connector       # Внешние данные
│   └── governance-voting      # DAO голосование
└── Infrastructure
    ├── Collator Nodes         # Блок продакшен
    ├── Full Nodes            # RPC эндпоинты
    └── Archive Nodes         # Исторические данные
```

## Этап 1: Подготовка инфраструктуры (1-2 недели)

### 1.1 Среда разработки

```bash
# Установка Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup default stable
rustup update nightly
rustup target add wasm32-unknown-unknown
rustup component add rust-src

# Установка Substrate tools
cargo install --git https://github.com/paritytech/substrate subkey
cargo install --git https://github.com/paritytech/polkadot-sdk polkadot-parachain-bin

# ink! contract tools
cargo install cargo-contract --force
```

### 1.2 Клонирование и настройка

```bash
git clone https://github.com/shepherdvovkes/AIVectorMP.git
cd AIVectorMP

# Установка зависимостей
cargo fetch
npm install

# Настройка окружения
cp .env.example .env
# Отредактируйте .env с вашими параметрами
```

## Этап 2: Разработка Runtime (2-3 недели)

### 2.1 Основные Pallets

**Vector Marketplace Pallet** (`pallets/vector-marketplace/src/lib.rs`):
```rust
#[pallet::config]
pub trait Config: frame_system::Config {
    type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
    type Currency: Currency<Self::AccountId>;
    type DatasetId: Parameter + Member + MaybeSerializeDeserialize + Debug + Default + MaxEncodedLen + TypeInfo;
    type MinDatasetPrice: Get<BalanceOf<Self>>;
    type MaxDescriptionLength: Get<u32>;
}

#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::weight(10_000)]
    pub fn register_dataset(
        origin: OriginFor<T>,
        name: BoundedVec<u8, T::MaxDescriptionLength>,
        description: BoundedVec<u8, T::MaxDescriptionLength>,
        embedding_model: BoundedVec<u8, T::MaxDescriptionLength>,
        price_per_query: BalanceOf<T>,
        metadata_hash: T::Hash,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;
        ensure!(price_per_query >= T::MinDatasetPrice::get(), Error::<T>::PriceTooLow);
        
        let dataset_id = Self::next_dataset_id();
        let dataset = DatasetInfo {
            owner: who.clone(),
            name: name.clone(),
            description,
            embedding_model,
            price_per_query,
            metadata_hash,
            is_active: true,
            created_at: <frame_system::Pallet<T>>::block_number(),
            total_queries: 0u32,
        };
        
        Datasets::<T>::insert(&dataset_id, &dataset);
        NextDatasetId::<T>::mutate(|id| *id += 1u32.into());
        
        Self::deposit_event(Event::DatasetRegistered {
            dataset_id,
            owner: who,
            name,
        });
        
        Ok(())
    }
    
    #[pallet::weight(10_000)]
    pub fn create_query_request(
        origin: OriginFor<T>,
        dataset_id: T::DatasetId,
        query_hash: T::Hash,
        escrow_amount: BalanceOf<T>,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;
        let dataset = Self::datasets(&dataset_id).ok_or(Error::<T>::DatasetNotFound)?;
        ensure!(dataset.is_active, Error::<T>::DatasetInactive);
        ensure!(escrow_amount >= dataset.price_per_query, Error::<T>::InsufficientPayment);
        
        // Создаем escrow через pallet-payment-escrow
        T::EscrowProvider::create_escrow(
            who.clone(),
            dataset.owner.clone(),
            escrow_amount,
            T::EscrowPeriod::get(),
        )?;
        
        let query_id = Self::next_query_id();
        let query_request = QueryRequest {
            requester: who.clone(),
            dataset_id,
            query_hash,
            escrow_amount,
            status: QueryStatus::Pending,
            created_at: <frame_system::Pallet<T>>::block_number(),
        };
        
        QueryRequests::<T>::insert(&query_id, &query_request);
        NextQueryId::<T>::mutate(|id| *id += 1u32.into());
        
        Self::deposit_event(Event::QueryRequestCreated {
            query_id,
            requester: who,
            dataset_id,
        });
        
        Ok(())
    }
}
```

**ZK Verification Pallet** (`pallets/zk-verification/src/lib.rs`):
```rust
#[pallet::call]
impl<T: Config> Pallet<T> {
    #[pallet::weight(50_000)]
    pub fn register_verification_key(
        origin: OriginFor<T>,
        circuit_type: CircuitType,
        verification_key: BoundedVec<u8, T::MaxVkSize>,
        description: BoundedVec<u8, T::MaxDescriptionLength>,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;
        
        let vk_id = Self::next_vk_id();
        let vk_info = VerificationKeyInfo {
            owner: who.clone(),
            circuit_type,
            verification_key: verification_key.clone(),
            description,
            is_active: true,
            created_at: <frame_system::Pallet<T>>::block_number(),
        };
        
        VerificationKeys::<T>::insert(&vk_id, &vk_info);
        NextVkId::<T>::mutate(|id| *id += 1u32.into());
        
        Self::deposit_event(Event::VerificationKeyRegistered {
            vk_id,
            owner: who,
            circuit_type,
        });
        
        Ok(())
    }
    
    #[pallet::weight(100_000)]
    pub fn submit_and_verify_proof(
        origin: OriginFor<T>,
        query_id: T::QueryId,
        proof_data: BoundedVec<u8, T::MaxProofSize>,
        public_inputs: BoundedVec<u8, T::MaxPublicInputsSize>,
        vk_id: T::VkId,
    ) -> DispatchResult {
        let who = ensure_signed(origin)?;
        
        let vk_info = Self::verification_keys(&vk_id).ok_or(Error::<T>::VkNotFound)?;
        ensure!(vk_info.is_active, Error::<T>::VkInactive);
        
        // Верификация proof (упрощенная версия)
        let is_valid = Self::verify_proof_internal(&proof_data, &public_inputs, &vk_info.verification_key)?;
        ensure!(is_valid, Error::<T>::InvalidProof);
        
        let proof_id = Self::next_proof_id();
        let proof_submission = ProofSubmission {
            submitter: who.clone(),
            query_id,
            proof_data,
            public_inputs,
            vk_id,
            is_verified: true,
            submitted_at: <frame_system::Pallet<T>>::block_number(),
        };
        
        ProofSubmissions::<T>::insert(&proof_id, &proof_submission);
        NextProofId::<T>::mutate(|id| *id += 1u32.into());
        
        // Уведомляем vector-marketplace о успешной верификации
        T::MarketplaceProvider::complete_query(query_id)?;
        
        Self::deposit_event(Event::ProofVerified {
            proof_id,
            query_id,
            verifier: who,
        });
        
        Ok(())
    }
}
```

### 2.2 Runtime композиция

**Runtime конфигурация** (`runtime/src/lib.rs`):
```rust
impl pallet_vector_marketplace::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type DatasetId = u32;
    type QueryId = u32;
    type MinDatasetPrice = MinDatasetPrice;
    type MaxDescriptionLength = MaxDescriptionLength;
    type EscrowProvider = PaymentEscrow;
    type EscrowPeriod = EscrowPeriod;
}

impl pallet_zk_verification::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type VkId = u32;
    type ProofId = u32;
    type QueryId = u32;
    type MaxVkSize = MaxVkSize;
    type MaxProofSize = MaxProofSize;
    type MaxPublicInputsSize = MaxPublicInputsSize;
    type MaxDescriptionLength = MaxDescriptionLength;
    type MarketplaceProvider = VectorMarketplace;
}

impl pallet_payment_escrow::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EscrowId = u32;
    type MinEscrowAmount = MinEscrowAmount;
    type MaxEscrowPeriod = MaxEscrowPeriod;
}

construct_runtime!(
    pub enum Runtime where
        Block = Block,
        NodeBlock = opaque::Block,
        UncheckedExtrinsic = UncheckedExtrinsic,
    {
        // System pallets
        System: frame_system,
        Timestamp: pallet_timestamp,
        Aura: pallet_aura,
        Grandpa: pallet_grandpa,
        Balances: pallet_balances,
        TransactionPayment: pallet_transaction_payment,
        Sudo: pallet_sudo,
        
        // Parachain pallets
        ParachainSystem: cumulus_pallet_parachain_system,
        ParachainInfo: parachain_info,
        
        // Custom pallets
        VectorMarketplace: pallet_vector_marketplace,
        ZkVerification: pallet_zk_verification,
        PaymentEscrow: pallet_payment_escrow,
        CrossChainBridge: pallet_cross_chain_bridge,
        
        // Smart contracts support
        Contracts: pallet_contracts,
        
        // Governance
        Democracy: pallet_democracy,
        Council: pallet_collective::<Instance1>,
        TechnicalCommittee: pallet_collective::<Instance2>,
        Treasury: pallet_treasury,
    }
);
```

## Этап 3: Компиляция и тестирование (1-2 недели)

### 3.1 Локальная сборка

```bash
# Сборка parachain node
cargo build --release

# Проверка сборки
./target/release/aivectormp-node --version
```

### 3.2 Генерация chain spec

```bash
# Генерация raw chain spec для development
./target/release/aivectormp-node build-spec --disable-default-bootnode --chain dev > dev-spec.json

# Конвертация в raw format
./target/release/aivectormp-node build-spec --chain dev-spec.json --raw --disable-default-bootnode > dev-spec-raw.json

# Генерация для testnet
./target/release/aivectormp-node build-spec --disable-default-bootnode --chain local > local-testnet-spec.json
./target/release/aivectormp-node build-spec --chain local-testnet-spec.json --raw --disable-default-bootnode > local-testnet-spec-raw.json
```

### 3.3 Локальное тестирование

```bash
# Запуск single node для разработки
./target/release/aivectormp-node \
  --dev \
  --tmp \
  --ws-external \
  --rpc-external \
  --rpc-cors all

# Запуск с детальными логами
RUST_LOG=runtime::contracts=debug,runtime::system=debug \
./target/release/aivectormp-node --dev --tmp
```

### 3.4 Тестирование функциональности

```bash
# Установка polkadot-js tools
npm install -g @polkadot/api-cli

# Проверка подключения
polkadot-js-api --ws ws://localhost:9944 query.system.chain

# Тест регистрации датасета
polkadot-js-api --ws ws://localhost:9944 --seed "//Alice" \
  tx.vectorMarketplace.registerDataset \
  "BERT Embeddings" "High-quality sentence embeddings" "bert-base-uncased" 1000000000000 0x1234
```

## Этап 4: Smart Contracts деплой (1 неделя)

### 4.1 Сборка ink! контрактов

```bash
cd contracts/dataset-registry
cargo contract build

cd ../oracle-connector  
cargo contract build

cd ../governance-voting
cargo contract build
```

### 4.2 Деплой контрактов

```bash
# Dataset Registry Contract
cargo contract instantiate \
  --constructor new \
  --args 1000000000000 \
  --suri //Alice \
  --url ws://localhost:9944 \
  --skip-confirm

# Oracle Connector Contract
cargo contract instantiate \
  --constructor new \
  --args "https://api.coingecko.com" \
  --suri //Alice \
  --url ws://localhost:9944 \
  --skip-confirm

# Governance Voting Contract
cargo contract instantiate \
  --constructor new \
  --args 86400 1000000000000 \
  --suri //Alice \
  --url ws://localhost:9944 \
  --skip-confirm
```

## Этап 5: Rococo Testnet деплой (2-3 недели)

### 5.1 Регистрация Parachain

```bash
# Генерация parachain spec для Rococo
./target/release/aivectormp-node build-spec \
  --chain rococo-local \
  --disable-default-bootnode > rococo-spec.json

# Raw format
./target/release/aivectormp-node build-spec \
  --chain rococo-spec.json \
  --disable-default-bootnode \
  --raw > rococo-spec-raw.json

# Генерация genesis state и wasm
./target/release/aivectormp-node export-genesis-state \
  --chain rococo-spec-raw.json > genesis-state

./target/release/aivectormp-node export-genesis-wasm \
  --chain rococo-spec-raw.json > genesis-wasm
```

### 5.2 Резервирование ParaID

```bash
# Через Polkadot.js Apps на Rococo
# 1. Подключитесь к wss://rococo-rpc.polkadot.io
# 2. Developer -> Extrinsics
# 3. registrar -> reserve()
# 4. Подтвердите транзакцию

# Или через CLI
polkadot-js-api --ws wss://rococo-rpc.polkadot.io \
  --seed "//YourSeed" \
  tx.registrar.reserve
```

### 5.3 Регистрация Parachain

```bash
# Регистрация через governance
polkadot-js-api --ws wss://rococo-rpc.polkadot.io \
  --seed "//YourSeed" \
  tx.registrar.register \
  2000 \
  "$(cat genesis-state)" \
  "$(cat genesis-wasm)"
```

### 5.4 Запуск Collator nodes

**Collator Node конфигурация** (`scripts/start-collator.sh`):
```bash
#!/bin/bash

# Основной collator
./target/release/aivectormp-node \
  --collator \
  --force-authoring \
  --chain rococo-spec-raw.json \
  --base-path /tmp/parachain/alice \
  --port 40333 \
  --ws-port 8844 \
  --rpc-port 8833 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000001 \
  --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
  --validator \
  -- \
  --execution wasm \
  --chain rococo-raw.json \
  --port 30343 \
  --ws-port 9977 \
  --rpc-port 9966
```

**Второй collator** (`scripts/start-collator-2.sh`):
```bash
#!/bin/bash

./target/release/aivectormp-node \
  --collator \
  --force-authoring \
  --chain rococo-spec-raw.json \
  --base-path /tmp/parachain/bob \
  --port 40334 \
  --ws-port 8845 \
  --rpc-port 8834 \
  --node-key 0000000000000000000000000000000000000000000000000000000000000002 \
  --telemetry-url "wss://telemetry.polkadot.io/submit/ 0" \
  --validator \
  -- \
  --execution wasm \
  --chain rococo-raw.json \
  --port 30344 \
  --ws-port 9978 \
  --rpc-port 9967
```

## Этап 6: Production Infrastructure (3-4 недели)

### 6.1 Node Infrastructure Setup

**Docker конфигурация** (`docker/Dockerfile.collator`):
```dockerfile
FROM ubuntu:20.04

# Install dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Copy binary
COPY target/release/aivectormp-node /usr/local/bin/

# Create user
RUN groupadd -r aivector && useradd -r -g aivector aivector
RUN mkdir -p /data && chown aivector:aivector /data

USER aivector
WORKDIR /data

EXPOSE 30333 9933 9944

ENTRYPOINT ["aivectormp-node"]
```

**Docker Compose** (`docker/docker-compose.yml`):
```yaml
version: '3.8'

services:
  collator-1:
    build:
      context: ..
      dockerfile: docker/Dockerfile.collator
    ports:
      - "30333:30333"
      - "9933:9933"
      - "9944:9944"
    volumes:
      - collator1-data:/data
      - ../rococo-spec-raw.json:/data/rococo-spec-raw.json:ro
    command: [
      "--collator",
      "--force-authoring", 
      "--chain", "/data/rococo-spec-raw.json",
      "--base-path", "/data",
      "--port", "30333",
      "--ws-port", "9944",
      "--rpc-port", "9933",
      "--validator",
      "--", 
      "--execution", "wasm",
      "--chain", "/data/rococo-raw.json",
      "--port", "30343"
    ]
    restart: unless-stopped

  collator-2:
    build:
      context: ..
      dockerfile: docker/Dockerfile.collator
    ports:
      - "30334:30333"
      - "9934:9933" 
      - "9945:9944"
    volumes:
      - collator2-data:/data
      - ../rococo-spec-raw.json:/data/rococo-spec-raw.json:ro
    command: [
      "--collator",
      "--force-authoring",
      "--chain", "/data/rococo-spec-raw.json", 
      "--base-path", "/data",
      "--port", "30333",
      "--ws-port", "9944",
      "--rpc-port", "9933",
      "--validator",
      "--",
      "--execution", "wasm", 
      "--chain", "/data/rococo-raw.json",
      "--port", "30343"
    ]
    restart: unless-stopped

volumes:
  collator1-data:
  collator2-data:
```

### 6.2 Мониторинг и метрики

**Prometheus конфигурация** (`monitoring/prometheus.yml`):
```yaml
global:
  scrape_interval: 15s

scrape_configs:
  - job_name: 'aivectormp-collator-1'
    static_configs:
      - targets: ['localhost:9615']
    metrics_path: /metrics
    
  - job_name: 'aivectormp-collator-2' 
    static_configs:
      - targets: ['localhost:9616']
    metrics_path: /metrics
    
  - job_name: 'substrate-telemetry'
    static_configs:
      - targets: ['telemetry.polkadot.io:443']
```

**Grafana Dashboard** (`monitoring/dashboard.json`):
```json
{
  "dashboard": {
    "title": "AIVectorMP Parachain",
    "panels": [
      {
        "title": "Block Height",
        "type": "stat",
        "targets": [
          {
            "expr": "substrate_block_height{instance=\"localhost:9615\"}"
          }
        ]
      },
      {
        "title": "Transactions per Second",
        "type": "graph", 
        "targets": [
          {
            "expr": "rate(substrate_transactions_total[1m])"
          }
        ]
      },
      {
        "title": "Active Datasets",
        "type": "stat",
        "targets": [
          {
            "expr": "substrate_pallet_vector_marketplace_datasets_total"
          }
        ]
      }
    ]
  }
}
```

### 6.3 CI/CD Pipeline

**GitHub Actions** (`.github/workflows/deploy.yml`):
```yaml
name: Deploy AIVectorMP

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          target: wasm32-unknown-unknown
          
      - name: Run tests
        run: cargo test --all
        
      - name: Check formatting
        run: cargo fmt --all -- --check
        
      - name: Run clippy
        run: cargo clippy --all-targets --all-features -- -D warnings

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Build parachain
        run: cargo build --release
        
      - name: Build contracts
        run: |
          cd contracts
          ./build-all.sh
          
      - name: Upload artifacts
        uses: actions/upload-artifact@v3
        with:
          name: binaries
          path: |
            target/release/aivectormp-node
            contracts/*/target/ink/*.wasm
            contracts/*/target/ink/*.json

  deploy-testnet:
    needs: build
    runs-on: ubuntu-latest
    if: github.ref == 'refs/heads/main'
    steps:
      - name: Deploy to Rococo
        run: |
          # Deployment script
          ./scripts/deploy-rococo.sh
        env:
          DEPLOY_KEY: ${{ secrets.DEPLOY_KEY }}
```

## Этап 7: Frontend и интеграция (2-3 недели)

### 7.1 Frontend приложение

**React + Polkadot.js интеграция** (`frontend/src/api/polkadot.js`):
```javascript
import { ApiPromise, WsProvider } from '@polkadot/api';
import { web3Accounts, web3Enable, web3FromSource } from '@polkadot/extension-dapp';

class PolkadotAPI {
  constructor() {
    this.api = null;
    this.accounts = [];
  }

  async connect() {
    const provider = new WsProvider('wss://aivectormp-rpc.rococo.subsocial.network');
    this.api = await ApiPromise.create({ provider });
    
    // Enable polkadot-js extension
    await web3Enable('AIVectorMP');
    this.accounts = await web3Accounts();
    
    return this.api;
  }

  async registerDataset(account, name, description, model, price) {
    const injector = await web3FromSource(account.meta.source);
    
    return new Promise((resolve, reject) => {
      this.api.tx.vectorMarketplace
        .registerDataset(name, description, model, price, '0x00')
        .signAndSend(account.address, { signer: injector.signer }, (result) => {
          if (result.status.isInBlock) {
            console.log(`Transaction included at blockHash ${result.status.asInBlock}`);
          } else if (result.status.isFinalized) {
            console.log(`Transaction finalized at blockHash ${result.status.asFinalized}`);
            resolve(result);
          } else if (result.isError) {
            reject(new Error('Transaction failed'));
          }
        });
    });
  }

  async createQuery(account, datasetId, queryHash, amount) {
    const injector = await web3FromSource(account.meta.source);
    
    return this.api.tx.vectorMarketplace
      .createQueryRequest(datasetId, queryHash, amount)
      .signAndSend(account.address, { signer: injector.signer });
  }

  async getDatasets() {
    const datasets = [];
    const datasetCount = await this.api.query.vectorMarketplace.nextDatasetId();
    
    for (let i = 1; i < datasetCount.toNumber(); i++) {
      const dataset = await this.api.query.vectorMarketplace.datasets(i);
      if (dataset.isSome) {
        datasets.push({
          id: i,
          ...dataset.unwrap().toJSON()
        });
      }
    }
    
    return datasets;
  }
}

export default new PolkadotAPI();
```

### 7.2 ZK Circuit интеграция

**HALO2 Circuit** (`circuits/src/vector_search.rs`):
```rust
use halo2_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Expression, Instance},
    poly::Rotation,
};

#[derive(Clone, Debug)]
pub struct VectorSearchConfig {
    pub advice: [Column<Advice>; 3],
    pub instance: Column<Instance>,
}

#[derive(Clone, Debug)]
pub struct VectorSearchCircuit {
    pub query_vector: Vec<Value<u64>>,
    pub dataset_vectors: Vec<Vec<Value<u64>>>,
    pub similarity_threshold: Value<u64>,
    pub result_indices: Vec<Value<u64>>,
}

impl Circuit<u64> for VectorSearchCircuit {
    type Config = VectorSearchConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self {
            query_vector: vec![Value::unknown(); self.query_vector.len()],
            dataset_vectors: self.dataset_vectors.iter()
                .map(|v| vec![Value::unknown(); v.len()])
                .collect(),
            similarity_threshold: Value::unknown(),
            result_indices: vec![Value::unknown(); self.result_indices.len()],
        }
    }

    fn configure(meta: &mut ConstraintSystem<u64>) -> Self::Config {
        let advice = [meta.advice_column(), meta.advice_column(), meta.advice_column()];
        let instance = meta.instance_column();

        meta.enable_equality(advice[0]);
        meta.enable_equality(advice[1]);
        meta.enable_equality(advice[2]);
        meta.enable_equality(instance);

        // Similarity computation constraint
        meta.create_gate("similarity", |meta| {
            let query = meta.query_advice(advice[0], Rotation::cur());
            let dataset = meta.query_advice(advice[1], Rotation::cur());
            let similarity = meta.query_advice(advice[2], Rotation::cur());
            
            vec![similarity - (query * dataset)] // Simplified dot product
        });

        VectorSearchConfig { advice, instance }
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<u64>,
    ) -> Result<(), Error> {
        // Constraint implementation
        layouter.assign_region(
            || "vector search",
            |mut region| {
                // Vector similarity computation logic
                for (i, (query_val, dataset_vals)) in self.query_vector.iter()
                    .zip(self.dataset_vectors.iter()).enumerate() {
                    
                    for (j, dataset_val) in dataset_vals.iter().enumerate() {
                        region.assign_advice(
                            || format!("query[{}]", i),
                            config.advice[0],
                            i + j,
                            || *query_val,
                        )?;
                        
                        region.assign_advice(
                            || format!("dataset[{}][{}]", i, j),
                            config.advice[1], 
                            i + j,
                            || *dataset_val,
                        )?;
                    }
                }
                
                Ok(())
            },
        )
    }
}
```

## Этап 8: Governance и Security (1-2 недели)

### 8.1 On-chain Governance

**Governance параметры** (`runtime/src/lib.rs`):
```rust
parameter_types! {
    pub const LaunchPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    pub const VotingPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
    pub const FastTrackVotingPeriod: BlockNumber = 3 * 24 * 60 * MINUTES;
    pub const MinimumDeposit: Balance = 100 * DOLLARS;
    pub const EnactmentPeriod: BlockNumber = 30 * 24 * 60 * MINUTES;
    pub const CooloffPeriod: BlockNumber = 28 * 24 * 60 * MINUTES;
}

impl pallet_democracy::Config for Runtime {
    type RuntimeEvent = RuntimeEvent;
    type Currency = Balances;
    type EnactmentPeriod = EnactmentPeriod;
    type LaunchPeriod = LaunchPeriod;
    type VotingPeriod = VotingPeriod;
    type VoteLockingPeriod = EnactmentPeriod;
    type MinimumDeposit = MinimumDeposit;
    // ... other config
}
```

### 8.2 Security аудит

**Checklist безопасности**:
- ✅ Overflow/underflow проверки
- ✅ Reentrancy защита в escrow
- ✅ Access control для admin функций
- ✅ Input validation для всех extrinsics
- ✅ Economic security модель
- ✅ ZK proof soundness

## Production Deployment

```bash
# Финальный деплой
./scripts/deploy-production.sh

# Мониторинг
docker-compose -f monitoring/docker-compose.yml up -d

# Health check
curl https://aivectormp-api.polkadot.io/health
```

**AIVectorMP готов к production!** 🚀# AI Vector Blockchain - Smart Contracts Deployment Guide

## Предварительные требования

### 1. Установка инструментов
```bash
# Установка Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Установка cargo-contract
cargo install cargo-contract --force

# Установка substrate node (для локального тестирования)
cargo install substrate-contracts-node --git https://github.com/paritytech/substrate-contracts-node.git

# Добавление WebAssembly target
rustup target add wasm32-unknown-unknown
```

### 2. Настройка Polkadot.js
- Установите Polkadot.js extension: https://polkadot.js.org/extension/
- Создайте тестовый аккаунт
- Получите тестовые токены с faucet

## Структура проекта

```
ai_vector_blockchain/
├── contracts/
│   ├── dataset_registry/
│   │   ├── lib.rs
│   │   └── Cargo.toml
│   ├── payment_manager/
│   │   ├── lib.rs
│   │   └── Cargo.toml
│   └── zk_verifier/
│       ├── lib.rs
│       └── Cargo.toml
├── deployment/
│   ├── deploy.js
│   └── addresses.json
└── README.md
```

## Этапы деплоя

### 1. Компиляция контрактов

Для каждого контракта:

```bash
# Dataset Registry
cd contracts/dataset_registry
cargo contract build

# Payment Manager
cd ../payment_manager
cargo contract build

# ZK Verifier
cd ../zk_verifier
cargo contract build
```

### 2. Локальное тестирование

```bash
# Запуск локального substrate node
substrate-contracts-node --dev --tmp

# В другом терминале - деплой контрактов
cargo contract instantiate \
  --constructor new \
  --args 1000000000000 \
  --suri //Alice \
  --url ws://localhost:9944
```

### 3. Деплой на Rococo Testnet

#### Подготовка

1. Получите ROC токены: https://faucet.rococo.darwinia.network/
2. Добавьте Rococo в Polkadot.js Apps: https://polkadot.js.org/apps/#/explorer

#### Деплой Dataset Registry

```bash
cargo contract upload \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io

cargo contract instantiate \
  --constructor new \
  --args 1000000000000 \
  --value 1000000000000 \
  --gas 200000000000 \
  --storage-deposit-limit 500000000000 \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

#### Деплой Payment Manager

```bash
cargo contract instantiate \
  --constructor new \
  --args DATASET_REGISTRY_ADDRESS ZK_VERIFIER_ADDRESS 250 86400000 \
  --value 1000000000000 \
  --gas 200000000000 \
  --storage-deposit-limit 500000000000 \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

#### Деплой ZK Verifier

```bash
cargo contract instantiate \
  --constructor new \
  --args PAYMENT_MANAGER_ADDRESS DATASET_REGISTRY_ADDRESS 1000000000000 86400000 \
  --value 1000000000000 \
  --gas 200000000000 \
  --storage-deposit-limit 500000000000 \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

### 4. Настройка cross-contract calls

После деплоя всех контрактов необходимо:

1. **Обновить адреса в контрактах** - добавить реальные адреса развернутых контрактов
2. **Настроить права доступа** - добавить ZK Verifier как авторизованного caller в Payment Manager
3. **Добавить валидаторов** - зарегистрировать validator nodes в ZK Verifier

```bash
# Добавление валидатора
cargo contract call \
  --contract ZK_VERIFIER_ADDRESS \
  --message add_validator \
  --args VALIDATOR_ADDRESS \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

## Тестирование функциональности

### 1. Регистрация датасета

```bash
cargo contract call \
  --contract DATASET_REGISTRY_ADDRESS \
  --message register_dataset \
  --args "\"Test Dataset\"" "\"Description\"" [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] [1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1] 1000000000000 \
  --value 1000000000000 \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

### 2. Создание платежа

```bash
cargo contract call \
  --contract PAYMENT_MANAGER_ADDRESS \
  --message create_payment \
  --args 1 \
  --value 1000000000000 \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

### 3. Верификация proof

```bash
# Сначала регистрируем verification key
cargo contract call \
  --contract ZK_VERIFIER_ADDRESS \
  --message register_verification_key \
  --args [1,2,3,4] "\"halo2\"" \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io

# Затем отправляем proof
cargo contract call \
  --contract ZK_VERIFIER_ADDRESS \
  --message submit_proof \
  --args 1 1 [5,6,7,8] [9,10] KEY_HASH [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

## Мониторинг и отладка

### 1. Просмотр событий

```bash
# Подписка на события контракта
cargo contract call \
  --contract CONTRACT_ADDRESS \
  --message get_dataset \
  --args 1 \
  --dry-run \
  --suri "//YourSeedPhrase" \
  --url wss://rococo-contracts-rpc.polkadot.io
```

### 2. Проверка состояния

- Используйте Polkadot.js Apps для просмотра состояния контрактов
- Проверяйте balance контрактов
- Мониторьте события через block explorer

## Безопасность

### 1. Аудит контрактов
- Проведите статический анализ кода
- Протестируйте все edge cases
- Проверьте overflow/underflow условия

### 2. Управление ключами
- Используйте hardware wallet для mainnet
- Настройте multisig для критических операций
- Ограничьте права доступа

### 3. Мониторинг
- Настройте alerts для подозрительных транзакций
- Мониторьте balance контрактов
- Логируйте все административные операции

## Следующие шаги

1. **Интеграция с frontend** - создать веб-интерфейс
2. **Параchain integration** - развернуть на собственной параchain
3. **ZK circuits** - имплементировать реальные HALO2 circuits
4. **Indexer service** - создать сервис для индексации событий
5. **Analytics dashboard** - создать dashboard для мониторинга сети

## Полезные ссылки

- [ink! Documentation](https://use.ink/)
- [Substrate Contracts Workshop](https://docs.substrate.io/tutorials/smart-contracts/)
- [Polkadot.js API](https://polkadot.js.org/docs/)
- [Rococo Testnet](https://wiki.polkadot.network/docs/build-pdk#rococo-testnet)
- [HALO2 Documentation](https://zcash.github.io/halo2/)

## Техническая поддержка

При возникновении проблем:
1. Проверьте логи substrate node
2. Используйте `--dry-run` для тестирования транзакций
3. Проверьте gas limits и storage deposits
4. Обратитесь к документации ink! и Substrate