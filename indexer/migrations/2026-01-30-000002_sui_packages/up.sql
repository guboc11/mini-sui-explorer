CREATE TABLE IF NOT EXISTS sui_packages (
    object_id TEXT NOT NULL,
    object_version BIGINT NOT NULL,
    object_digest TEXT NOT NULL,
    checkpoint_sequence_number BIGINT NOT NULL,
    owner_type TEXT,
    owner_id TEXT,
    object_type TEXT,
    object_bcs BYTEA,
    PRIMARY KEY (object_id, object_version)
);
