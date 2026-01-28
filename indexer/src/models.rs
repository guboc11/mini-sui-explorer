use diesel::prelude::*;
use sui_indexer_alt_framework::FieldCount;
use crate::schema::transaction_digests;
use crate::schema::sui_objects;

#[derive(Insertable, Debug, Clone, FieldCount)]
#[diesel(table_name = transaction_digests)]
pub struct StoredTransactionDigest {
    pub tx_digest: String,
    pub checkpoint_sequence_number: i64,
}

#[derive(Insertable, Debug, Clone, FieldCount)]
#[diesel(table_name = sui_objects)]
pub struct StoredObjectData {
    pub object_id: String,
    pub object_version: i64,
    pub object_digest: String,
    pub checkpoint_sequence_number: i64,
    pub owner_type: Option<String>,
    pub owner_id: Option<String>,
    pub object_type: Option<String>,
    pub object_bcs: Option<Vec<u8>>,
}
