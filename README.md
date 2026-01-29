# mini-sui-explorer

A minimal Sui explorer that shows things the official one doesn't—like which objects exist under a package ID or what coins top whale wallets are holding.

## Local Docker Compose (step-by-step)

Prepare env files:

```sh
cp postgres.env.example postgres.env
cp backend/.example.env backend/.env
cp indexer/.example.env indexer/.env
```

### 1) Start PostgreSQL

```sh
docker compose up -d postgres
```

### 2) Start backend

```sh
docker compose up -d backend
```

### 3) Start indexer

```sh
docker compose up -d indexer
```

By default, the indexer starts from the latest checkpoint and requires a separate RPC URL:

```sh
docker compose up -d indexer -- --latest-rpc-url <grpc_fullnode_url>
```

To start from genesis (checkpoint 0):

```sh
docker compose up -d indexer -- --from-genesis
```

If you changed Dockerfiles or source code and want those changes in the containers, rebuild images first:

```sh
docker compose build backend indexer
```

Stop everything:

```sh
docker compose down
```

## Local Run (cargo)

Run the custom indexer directly:

```sh
cargo run -- \
  --latest-rpc-url https://fullnode.testnet.sui.io:443 \
  --remote-store-url https://checkpoints.testnet.sui.io
```
