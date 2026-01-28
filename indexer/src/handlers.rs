use anyhow::Result;
use std::sync::Arc;
use sui_indexer_alt_framework::pipeline::Processor;
use sui_indexer_alt_framework::types::full_checkpoint_content::Checkpoint;

use crate::models::StoredObjectData;
use crate::models::StoredTransactionDigest;
use crate::schema::sui_objects::dsl as sui_objects_dsl;
use crate::schema::transaction_digests::dsl as tx_digests_dsl;

use diesel_async::RunQueryDsl;
use sui_indexer_alt_framework::{
    pipeline::sequential::Handler,
    postgres::{Connection, Db},
    FieldCount,
};
use sui_indexer_alt_framework::types::object::Owner;

pub struct TransactionDigestHandler;

#[async_trait::async_trait]
impl Processor for TransactionDigestHandler {
    const NAME: &'static str = "transaction_digest_handler";

    type Value = StoredTransactionDigest;

    async fn process(&self, checkpoint: &Arc<Checkpoint>) -> Result<Vec<Self::Value>> {
        let checkpoint_seq = checkpoint.summary.sequence_number as i64;

        let digests = checkpoint
            .transactions
            .iter()
            .map(|tx| StoredTransactionDigest {
                tx_digest: tx.transaction.digest().to_string(),
                checkpoint_sequence_number: checkpoint_seq,
            })
            .collect();

        Ok(digests)
    }
}

#[async_trait::async_trait]
impl Handler for TransactionDigestHandler {
    type Store = Db;
    type Batch = Vec<Self::Value>;

    fn batch(&self, batch: &mut Self::Batch, values: std::vec::IntoIter<Self::Value>) {
        batch.extend(values);
    }

    async fn commit<'a>(&self, batch: &Self::Batch, conn: &mut Connection<'a>) -> Result<usize> {
        const MAX_POSTGRES_PARAMS: usize = 65_535;
        let max_rows = MAX_POSTGRES_PARAMS / StoredTransactionDigest::FIELD_COUNT;
        let mut total_inserted = 0;

        // Avoid exceeding Postgres bind parameter limits on large batches.
        for chunk in batch.chunks(max_rows) {
            let inserted = diesel::insert_into(tx_digests_dsl::transaction_digests)
                .values(chunk)
                .on_conflict(tx_digests_dsl::tx_digest)
                .do_nothing()
                .execute(conn)
                .await?;
            total_inserted += inserted;
        }

        Ok(total_inserted)
    }
}

pub struct ObjectDataHandler;

#[async_trait::async_trait]
impl Processor for ObjectDataHandler {
    const NAME: &'static str = "object_data_handler";

    type Value = StoredObjectData;

    async fn process(&self, checkpoint: &Arc<Checkpoint>) -> Result<Vec<Self::Value>> {
        let checkpoint_seq = checkpoint.summary.sequence_number as i64;

        let objects = checkpoint
            .transactions
            .iter()
            .flat_map(|tx| tx.output_objects(&checkpoint.object_set))
            .map(|object| {
                let (owner_type, owner_id) = match object.owner() {
                    Owner::AddressOwner(address) => (Some("address".to_string()), Some(address.to_string())),
                    Owner::ObjectOwner(address) => (Some("object".to_string()), Some(address.to_string())),
                    Owner::Shared { .. } => (Some("shared".to_string()), None),
                    Owner::Immutable => (Some("immutable".to_string()), None),
                    Owner::ConsensusAddressOwner { owner, .. } => {
                        (Some("consensus_address".to_string()), Some(owner.to_string()))
                    }
                };

                Ok(StoredObjectData {
                    object_id: object.id().to_string(),
                    object_version: object.version().value() as i64,
                    object_digest: object.digest().to_string(),
                    checkpoint_sequence_number: checkpoint_seq,
                    owner_type,
                    owner_id,
                    object_type: object.type_().map(|type_| type_.to_string()),
                    object_bcs: Some(bcs::to_bytes(object)?),
                })
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(objects)
    }
}

#[async_trait::async_trait]
impl Handler for ObjectDataHandler {
    type Store = Db;
    type Batch = Vec<Self::Value>;

    fn batch(&self, batch: &mut Self::Batch, values: std::vec::IntoIter<Self::Value>) {
        batch.extend(values);
    }

    async fn commit<'a>(&self, batch: &Self::Batch, conn: &mut Connection<'a>) -> Result<usize> {
        const MAX_POSTGRES_PARAMS: usize = 65_535;
        let max_rows = MAX_POSTGRES_PARAMS / StoredObjectData::FIELD_COUNT;
        let mut total_inserted = 0;

        for chunk in batch.chunks(max_rows) {
            let inserted = diesel::insert_into(sui_objects_dsl::sui_objects)
                .values(chunk)
                .on_conflict((sui_objects_dsl::object_id, sui_objects_dsl::object_version))
                .do_nothing()
                .execute(conn)
                .await?;
            total_inserted += inserted;
        }

        Ok(total_inserted)
    }
}
