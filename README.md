# mini-sui-explorer
A minimal Sui explorer that shows things the official one doesn't—like which objects exist under a package ID or what coins top whale wallets are holding.

## Indexer Docker (separate services)
This setup assumes the custom indexer and PostgreSQL run on different instances.

### Build the indexer image
```sh
docker build -t simple-sui-indexer:local ./indexer
```

### Run the indexer container
Set `DATABASE_URL` to point at your **external** PostgreSQL instance:
```sh
docker run --rm \
  -e DATABASE_URL=postgres://USER:PASSWORD@HOST:5432/sui_indexer \
  simple-sui-indexer:local
```

### Run PostgreSQL separately
Use a managed database (recommended) or run Postgres on another server/VM.
Example Docker run (on the DB host):
```sh
docker run --name sui-postgres -d \
  -e POSTGRES_USER=USER \
  -e POSTGRES_PASSWORD=PASSWORD \
  -e POSTGRES_DB=sui_indexer \
  -p 5432:5432 \
  postgres:16
```

## Local Docker Compose
Run Postgres + indexer together for local development:
```sh
docker compose up --build
```
