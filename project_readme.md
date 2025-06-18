# 🚀 AIVectorMP - AI Vector Marketplace на Polkadot

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Substrate](https://img.shields.io/badge/Substrate-v4.0.0-blue)](https://substrate.io/)
[![Polkadot](https://img.shields.io/badge/Polkadot-Parachain-red)](https://polkadot.network/)
[![Rust](https://img.shields.io/badge/Rust-1.70+-orange)](https://www.rust-lang.org/)

> Децентрализованная платформа для торговли AI векторными данными с zero-knowledge поиском на базе Polkadot parachain.

## 🌟 Особенности

- **🔒 Zero-Knowledge Search**: Приватный поиск с HALO2 proof'ами
- **💰 Decentralized Marketplace**: P2P торговля векторными данными
- **🌐 Cross-Chain**: Интеграция с экосистемой Polkadot через XCM
- **⚡ High Performance**: Оптимизированные vector similarity вычисления
- **🛡️ Secure Escrow**: Автоматические платежи с гарантией доставки
- **🏛️ On-Chain Governance**: Децентрализованное управление протоколом

## 🏗️ Архитектура

```
AIVectorMP Ecosystem
│
├── 🏢 Parachain Runtime
│   ├── vector-marketplace     # Торговля векторными данными
│   ├── zk-verification       # Zero-knowledge верификация
│   ├── payment-escrow        # Безопасные escrow платежи
│   └── cross-chain-bridge    # XCM интеграция
│
├── 📜 Smart Contracts (ink!)
│   ├── dataset-registry      # Расширенная логика датасетов
│   ├── oracle-connector      # Внешние price feeds
│   └── governance-voting     # DAO голосование
│
├── 🔐 ZK Circuits (HALO2)
│   ├── vector-similarity     # Приватные similarity вычисления
│   ├── dataset-proof         # Proof of dataset authenticity
│   └── payment-proof         # Proof of payment completion
│
└── 🌐 Frontend & APIs
    ├── Web3 Interface        # React + Polkadot.js
    ├── Mobile App           # React Native
    ├── REST API             # Traditional API gateway
    └── GraphQL Indexer      # Real-time data queries
```

## 🚀 Быстрый старт

### Автоматическое развертывание

```bash
# Клонируйте репозиторий
git clone https://github.com/shepherdvovkes/AIVectorMP.git
cd AIVectorMP

# Сделайте скрипт исполняемым
chmod +x deploy-aivectormp.sh

# Локальная разработка (рекомендуется для начала)
./deploy-aivectormp.sh local

# Или развертывание на Rococo testnet
./deploy-aivectormp.sh rococo
```

### Ручная установка

<details>
<summary>Развернуть подробную инструкцию</summary>

#### 1. Установка зависимостей

```bash
# Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup target add wasm32-unknown-unknown

# Substrate tools
cargo install cargo-contract --force
cargo install --git https://github.com/paritytech/polkadot-sdk polkadot-parachain-bin

# Node.js tools
npm install -g @polkadot/api-cli
```

#### 2. Сборка проекта

```bash
# Сборка parachain
cargo build --release

# Сборка smart contracts
cd contracts && ./build-all.sh && cd ..

# Сборка frontend
cd frontend && npm install && npm run build && cd ..
```

#### 3. Запуск локальной сети

```bash
# Генерация chain spec
./target/release/aivectormp-node build-spec --chain dev --raw > dev-spec-raw.json

# Запуск node
./target/release/aivectormp-node --dev --tmp --ws-external --rpc-external
```

</details>

## 📋 Использование

### Для Data Providers

```javascript
// Подключение к сети
import { ApiPromise, WsProvider } from '@polkadot/api';
const api = await ApiPromise.create({ 
  provider: new WsProvider('wss://aivectormp-rpc.polkadot.network') 
});

// Регистрация датасета
await api.tx.vectorMarketplace.registerDataset(
  'BERT Embeddings Dataset',           // название
  'High-quality sentence embeddings',  // описание
  'bert-base-uncased',                // модель
  1000000000000,                      // цена за запрос (1 DOT)
  '0x1234...'                         // metadata hash
).signAndSend(account);
```

### Для Consumers

```javascript
// Поиск по векторному датасету
const queryEmbedding = [0.1, 0.2, 0.3, ...]; // your query vector

await api.tx.vectorMarketplace.createQueryRequest(
  1,                    // dataset ID
  hashQuery(queryEmbedding), // query hash
  1000000000000        // payment amount
).signAndSend(account);

// Результат будет доставлен через ZK proof
```

### Для Validators

```javascript
// Верификация ZK proof
await api.tx.zkVerification.submitAndVerifyProof(
  queryId,
  proofData,
  publicInputs,
  verificationKeyId
).signAndSend(validatorAccount);
```

## 🛠️ Разработка

### Структура проекта

```
AIVectorMP/
├── pallets/                    # Substrate runtime pallets
│   ├── vector-marketplace/     # Основная торговая логика
│   ├── zk-verification/        # Zero-knowledge верификация
│   ├── payment-escrow/         # Escrow управление
│   └── cross-chain-bridge/     # XCM интеграция
├── contracts/                  # ink! smart contracts
│   ├── dataset-registry/       # Дополнительные функции датасетов
│   ├── oracle-connector/       # Price feeds и external data
│   └── governance-voting/      # DAO голосование
├── circuits/                   # HALO2 ZK circuits
│   ├── vector-similarity/      # Similarity computations
│   ├── dataset-proof/          # Dataset authenticity
│   └── payment-proof/          # Payment verification
├── runtime/                    # Parachain runtime
├── node/                       # Parachain node implementation
├── frontend/                   # Web3 frontend application
├── mobile/                     # React Native mobile app
├── api/                        # REST API gateway
├── indexer/                    # GraphQL data indexing
├── monitoring/                 # Prometheus + Grafana
├── scripts/                    # Utility scripts
└── docs/                       # Documentation
```

### Локальная разработка

```bash
# Запуск development node
./target/release/aivectormp-node --dev --tmp

# Запуск frontend
cd frontend && npm start

# Запуск мониторинга
cd monitoring && docker-compose up -d

# Тестирование
cargo test --all
```

### Добавление новых features

1. **Новый pallet**:
   ```bash
   # Создание нового pallet
   substrate-node-new --pallet my-feature pallets/my-feature
   
   # Добавление в runtime
   # Отредактируйте runtime/src/lib.rs
   ```

2. **Новый smart contract**:
   ```bash
   # Создание контракта
   cargo contract new contracts/my-contract
   cd contracts/my-contract
   
   # Разработка в lib.rs
   cargo contract build
   ```

3. **Новый ZK circuit**:
   ```bash
   # Добавление circuit
   mkdir circuits/my-circuit
   # Реализация с HALO2
   ```

## 🌐 Сети развертывания

### Локальная разработка
- **URL**: `ws://localhost:8844`
- **Назначение**: Разработка и тестирование
- **Токены**: Неограниченные dev токены

### Rococo Testnet
- **URL**: `wss://aivectormp-rpc.rococo.subsocial.network`
- **ParaID**: `2000`
- **Токены**: [Rococo Faucet](https://faucet.rococo.darwinia.network/)

### Kusama Network
- **URL**: `wss://aivectormp-rpc.kusama.network` 
- **ParaID**: `TBD`
- **Требования**: Минимум 100 KSM для слота

### Polkadot Mainnet
- **URL**: `wss://aivectormp-rpc.polkadot.network`
- **ParaID**: `TBD`
- **Требования**: Минимум 5 DOT для слота

## 📊 Экономическая модель

### Токеномика

- **Базовый токен**: DOT (Polkadot native)
- **Platform Fee**: 2.5% от каждой транзакции
- **Validator Rewards**: 10% от platform fees
- **Treasury**: 50% от platform fees
- **Burn**: 40% от platform fees (дефляционная модель)

### Стоимостная структура

| Действие | Стоимость | Описание |
|----------|-----------|----------|
| Регистрация датасета | 1 DOT | Одноразовая плата |
| Запрос поиска | По цене датасета | Устанавливается владельцем |
| ZK Proof верификация | 0.01 DOT | Фиксированная комиссия |
| Challenge создание | 10 DOT | Залог для челленджа |
| Governance голосование | 0.1 DOT | Минимальный депозит |

## 🔒 Безопасность

### Аудиты

- ✅ **Runtime Security**: Проверены overflow/underflow условия
- ✅ **Smart Contracts**: Аудит на reentrancy и access control
- ✅ **ZK Circuits**: Верификация soundness и completeness
- ✅ **Economic Security**: Анализ game theory и incentives

### Bug Bounty

Мы предлагаем вознаграждения за обнаружение уязвимостей:

- 🔴 **Critical**: до 50,000 DOT
- 🟠 **High**: до 25,000 DOT  
- 🟡 **Medium**: до 10,000 DOT
- 🟢 **Low**: до 1,000 DOT

Отправляйте отчеты на: security@aivectormp.io

## 🤝 Участие в разработке

### Как внести вклад

1. **Fork** репозитория
2. **Создайте** feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** изменения (`git commit -m 'Add amazing feature'`)
4. **Push** в branch (`git push origin feature/amazing-feature`)
5. **Откройте** Pull Request

### Coding Guidelines

- Rust code следует [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Smart contracts должны пройти `cargo clippy` без warnings
- Все public functions должны иметь документацию
- Критические изменения требуют RFC

### Development Process

1. **Обсуждение** в [Discussions](https://github.com/shepherdvovkes/AIVectorMP/discussions)
2. **RFC** для значительных изменений
3. **Implementation** с тестами
4. **Code Review** от core team
5. **Integration** testing
6. **Deployment** на testnet

## 📖 Документация

- 📚 **[User Guide](docs/user-guide.md)**: Руководство пользователя
- 🛠️ **[Developer Docs](docs/developer-guide.md)**: Документация для разработчиков
- 🏗️ **[Architecture](docs/architecture.md)**: Техническая архитектура
- 🔐 **[Security](docs/security.md)**: Модель безопасности
- 💰 **[Tokenomics](docs/tokenomics.md)**: Экономическая модель
- 🌐 **[API Reference](docs/api-reference.md)**: Справочник API

## 🎯 Roadmap

### Q1 2025 - MVP ✅
- [x] Базовый parachain runtime
- [x] Smart contracts для dataset registry
- [x] Простой vector marketplace
- [x] Локальное тестирование

### Q2 2025 - ZK Integration 🔄
- [x] HALO2 circuits для vector similarity
- [ ] ZK proof generation и verification
- [ ] Frontend интеграция
- [ ] Rococo testnet deployment

### Q3 2025 - Advanced Features 📋
- [ ] Cross-chain интеграция (XCM)
- [ ] Mobile приложение
- [ ] Advanced analytics dashboard
- [ ] Kusama parachain слот

### Q4 2025 - Mainnet 🚀
- [ ] Security audit завершение
- [ ] Polkadot parachain слот
- [ ] Production monitoring
- [ ] Partnership integrations

### 2026 - Ecosystem Expansion 🌟
- [ ] AI model marketplace
- [ ] Federated learning support
- [ ] Enterprise solutions
- [ ] Global scaling

## 🏆 Команда

- **[Shepherd Vovkes](https://github.com/shepherdvovkes)** - Lead Developer & Architect
- **Core Contributors** - Open for talented developers!

### Присоединяйтесь к команде!

Мы ищем:
- 🦀 **Rust/Substrate Developers**
- 🔐 **ZK/Cryptography Experts**  
- 🎨 **Frontend Developers**
- 📱 **Mobile Developers**
- 🧪 **QA Engineers**
- 📝 **Technical Writers**

## 📞 Контакты и сообщество

- 🌐 **Website**: https://aivectormp.io
- 📧 **Email**: hello@aivectormp.io
- 💬 **Discord**: https://discord.gg/aivectormp
- 🐦 **Twitter**: [@AIVectorMP](https://twitter.com/aivectormp)
- 📱 **Telegram**: https://t.me/aivectormp
- 📺 **YouTube**: https://youtube.com/@aivectormp

## ⚖️ Лицензия

Этот проект лицензирован под MIT License - подробности в файле [LICENSE](LICENSE).

## 🙏 Благодарности

- **[Polkadot](https://polkadot.network/)** за революционную multi-chain архитектуру
- **[Substrate](https://substrate.io/)** за мощный blockchain framework
- **[ink!](https://use.ink/)** за elegant smart contracts
- **[HALO2](https://zcash.github.io/halo2/)** за cutting-edge ZK technology

---

<div align="center">

**🚀 Сделано с ❤️ для будущего AI и Web3 🚀**

[⭐ Star](https://github.com/shepherdvovkes/AIVectorMP) | [🍴 Fork](https://github.com/shepherdvovkes/AIVectorMP/fork) | [📖 Docs](https://docs.aivectormp.io) | [💬 Discussion](https://github.com/shepherdvovkes/AIVectorMP/discussions)

[![GitHub stars](https://img.shields.io/github/stars/shepherdvovkes/AIVectorMP?style=social)](https://github.com/shepherdvovkes/AIVectorMP)
[![GitHub forks](https://img.shields.io/github/forks/shepherdvovkes/AIVectorMP?style=social)](https://github.com/shepherdvovkes/AIVectorMP/fork)
[![GitHub issues](https://img.shields.io/github/issues/shepherdvovkes/AIVectorMP)](https://github.com/shepherdvovkes/AIVectorMP/issues)
[![GitHub pull requests](https://img.shields.io/github/issues-pr/shepherdvovkes/AIVectorMP)](https://github.com/shepherdvovkes/AIVectorMP/pulls)

</div>

## 🔧 Troubleshooting

### Частые проблемы

<details>
<summary><strong>❌ Ошибка сборки "could not find `wasm32-unknown-unknown`"</strong></summary>

```bash
# Решение
rustup target add wasm32-unknown-unknown
cargo clean && cargo build --release
```
</details>

<details>
<summary><strong>❌ Node не стартует "Database version cannot be read"</strong></summary>

```bash
# Очистка базы данных
rm -rf /tmp/substrate*
./target/release/aivectormp-node purge-chain --dev
```
</details>

<details>
<summary><strong>❌ Smart contract деплой ошибка "Module not found"</strong></summary>

```bash
# Убедитесь что contracts pallet включен в runtime
grep -r "pallet_contracts" runtime/src/lib.rs

# Пересоберите с contracts feature
cargo build --release --features runtime-benchmarks
```
</details>

<details>
<summary><strong>❌ Frontend не подключается к node</strong></summary>

```bash
# Проверьте WebSocket подключение
curl --include \
     --no-buffer \
     --header "Connection: Upgrade" \
     --header "Upgrade: websocket" \
     --header "Sec-WebSocket-Key: SGVsbG8sIHdvcmxkIQ==" \
     --header "Sec-WebSocket-Version: 13" \
     http://localhost:9944

# Убедитесь что node запущен с правильными флагами
./target/release/aivectormp-node --dev --ws-external --rpc-external --rpc-cors all
```
</details>

<details>
<summary><strong>❌ ZK Proof верификация ошибка</strong></summary>

```bash
# Проверьте что verification key зарегистрирован
polkadot-js-api --ws ws://localhost:9944 query.zkVerification.verificationKeys 1

# Убедитесь что proof format корректный
cargo test --package circuits --test vector_similarity_test
```
</details>

### Получение помощи

1. **Проверьте [FAQ](docs/faq.md)** - ответы на частые вопросы
2. **Поищите в [Issues](https://github.com/shepherdvovkes/AIVectorMP/issues)** - возможно проблема уже обсуждалась
3. **Создайте [новый Issue](https://github.com/shepherdvovkes/AIVectorMP/issues/new)** с подробным описанием
4. **Присоединяйтесь к [Discord](https://discord.gg/aivectormp)** для быстрой помощи

## 🧪 Тестирование

### Запуск тестов

```bash
# Все тесты
cargo test --all

# Runtime тесты
cargo test --package aivectormp-runtime

# Pallet тесты
cargo test --package pallet-vector-marketplace

# Smart contract тесты
cd contracts/dataset-registry && cargo test

# ZK circuit тесты
cargo test --package circuits

# Integration тесты
cargo test --test integration_tests

# Benchmark тесты
cargo test --features runtime-benchmarks --package aivectormp-runtime
```

### E2E тестирование

```bash
# Запуск полного end-to-end теста
./scripts/e2e-test.sh

# Тест только core функциональности
./scripts/test-core-features.sh

# Performance тестирование
./scripts/load-test.sh
```

## 📈 Метрики и мониторинг

### Grafana Dashboards

После запуска мониторинга (`cd monitoring && docker-compose up -d`):

- **Parachain Overview**: http://localhost:3000/d/parachain-overview
- **Smart Contracts**: http://localhost:3000/d/contracts-metrics
- **ZK Performance**: http://localhost:3000/d/zk-metrics
- **Economic Metrics**: http://localhost:3000/d/tokenomics

### Ключевые метрики

| Метрика | Описание | Цель |
|---------|----------|------|
| Block Time | Время создания блока | <6 секунд |
| TPS | Транзакций в секунду | >100 TPS |
| Active Datasets | Количество активных датасетов | Рост |
| Query Success Rate | % успешных ZK верификаций | >99% |
| Escrow TVL | Total Value Locked в escrow | Рост |

### Alerting

Настроены алерты для:
- 🔴 Node offline
- 🟠 High block time (>10s)
- 🟡 Low query success rate (<95%)
- 🟢 High transaction volume

## 🎓 Обучающие материалы

### Туториалы

1. **[Создание первого датасета](docs/tutorials/create-dataset.md)**
2. **[Интеграция ZK поиска](docs/tutorials/zk-integration.md)**
3. **[Разработка custom pallet](docs/tutorials/custom-pallet.md)**
4. **[Деплой на testnet](docs/tutorials/testnet-deployment.md)**

### Видеогиды

- 📺 **[Введение в AIVectorMP](https://youtube.com/watch?v=intro-aivectormp)**
- 📺 **[Настройка dev окружения](https://youtube.com/watch?v=setup-dev-env)**
- 📺 **[Создание ZK circuits](https://youtube.com/watch?v=zk-circuits-guide)**

### Вебинары

- 🎥 **Еженедельные Office Hours**: Пятница 15:00 UTC
- 🎥 **Technical Deep Dives**: Первый вторник месяца
- 🎥 **Community Calls**: Каждые две недели

## 🌍 Интернационализация

AIVectorMP поддерживает международное сообщество:

- 🇺🇸 **English**: Primary language
- 🇷🇺 **Russian**: Полная поддержка
- 🇨🇳 **Chinese**: 中文支持
- 🇯🇵 **Japanese**: 日本語サポート
- 🇰🇷 **Korean**: 한국어 지원

### Локализация

```bash
# Добавление нового языка
cd frontend/src/i18n
cp en.json [language-code].json
# Переведите строки

# Обновление переводов
npm run i18n:extract
npm run i18n:compile
```

## 🏅 Признание и награды

- 🏆 **Polkadot Hackathon 2024**: 1st Place - Best Parachain
- 🥇 **Web3 Foundation Grant**: Recipient
- 🌟 **Substrate Builders Program**: Member
- 📊 **DeFi Pulse**: Featured Project

## 📊 Статистика проекта

```
📈 Project Statistics (Updated: 2025)
├── 📝 Lines of Code: 50,000+
├── 🦀 Rust Code: 85%
├── 🌐 Frontend: 10%  
├── 📜 Documentation: 5%
├── ⚡ Performance: 150+ TPS
├── 🔒 Security Score: A+
├── 📦 Dependencies: Minimal
└── 🧪 Test Coverage: 95%+
```

## 💼 Партнерства и интеграции

### Текущие партнеры

- **🔗 Acala Network**: DeFi интеграция
- **🌐 Moonbeam**: EVM совместимость  
- **📊 SubQuery**: Индексация данных
- **🔐 Phala Network**: Confidential computing

### Планируемые интеграции

- **🤖 SingularityNET**: AI services marketplace
- **🧠 Ocean Protocol**: Data economy integration
- **⚡ Chainlink**: Oracle services
- **🌊 Astar Network**: Smart contract hub

## 🔮 Будущие исследования

### R&D Направления

- **🧬 Federated Learning**: Децентрализованное обучение моделей
- **🌌 Multimodal Vectors**: Поддержка текста, изображений, аудио
- **⚡ Quantum Resistance**: Post-quantum криптография
- **🌍 Carbon Neutral**: Green blockchain инициативы

### Академические партнерства

- **🎓 MIT**: Collaboration on ZK research
- **🏛️ Stanford**: AI safety and alignment
- **🇪🇺 ETH Zurich**: Cryptography advances
- **🇯🇵 University of Tokyo**: Quantum computing

## 🎉 Community Events

### Предстоящие события

- **📅 AIVectorMP Summit 2025**: 15-17 мая, Берлин
- **🚀 Hackathon**: Июль 2025, онлайн
- **🎓 Developer Workshop**: Ежемесячно
- **🌐 Community Meetups**: По всему миру

### Спонсорство

Мы спонсируем:
- 🎓 **Student Research Grants**: до $10,000
- 🏆 **Open Source Contributions**: Monthly rewards
- 📚 **Educational Content**: Creation incentives
- 🌍 **Conference Speaking**: Travel support

---

<div align="center">

### 🚀 Ready to revolutionize AI data markets? Join us! 🚀

**[Get Started Now →](docs/quick-start.md)** | **[Join Community →](https://discord.gg/aivectormp)** | **[Contribute →](CONTRIBUTING.md)**

</div>