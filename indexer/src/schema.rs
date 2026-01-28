// @generated automatically by Diesel CLI.

diesel::table! {
    transaction_digests (tx_digest) {
        tx_digest -> Text,
        checkpoint_sequence_number -> Int8,
    }
}

diesel::table! {
    sui_objects (object_id, object_version) {
        object_id -> Text,
        object_version -> Int8,
        object_digest -> Text,
        checkpoint_sequence_number -> Int8,
        owner_type -> Nullable<Text>,
        owner_id -> Nullable<Text>,
        object_type -> Nullable<Text>,
        object_bcs -> Nullable<Bytea>,
    }
}
