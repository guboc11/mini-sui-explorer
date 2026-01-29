# mini-sui-explorer
A minimal Sui explorer that shows things the official one doesn't—like which objects exist under a package ID or what coins top whale wallets are holding.

## Indexer Docker
Build the indexer image:
```sh
docker build -t simple-sui-indexer:local ./indexer
```

Run the container (set your database URL):
```sh
docker run --rm \
  -e DATABASE_URL=postgres://USER:PASSWORD@HOST:5432/sui_indexer \
  simple-sui-indexer:local
```
