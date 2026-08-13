pub fn create_table() -> String {
    r#"
    CREATE TABLE IF NOT EXISTS chunks (
        chunk_id        INTEGER PRIMARY KEY,
        object_id       INTEGER NOT NULL REFERENCES objects(object_id),
        chunk_index     INTEGER NOT NULL,           -- thứ tự trong object: 0,1,2...
        disk_id         INTEGER NOT NULL REFERENCES disks(disk_id),
        segment_id      INTEGER REFERENCES segments(segment_id), -- NULL ở Giai đoạn 1
        "offset"        INTEGER,                    -- NULL ở Giai đoạn 1, bắt buộc ở Giai đoạn 2
        length          INTEGER NOT NULL,
        checksum        BLOB NOT NULL,               -- CRC32/SHA-256 của riêng chunk này
        status          TEXT NOT NULL DEFAULT 'temp'
                            CHECK (status IN ('temp','committed','trash','deleted')),
        created_at      INTEGER NOT NULL,
        UNIQUE (object_id, chunk_index)
    );
    CREATE INDEX IF NOT EXISTS idx_chunks_object   ON chunks(object_id, chunk_index);
    CREATE INDEX IF NOT EXISTS idx_chunks_segment  ON chunks(segment_id);
    CREATE INDEX IF NOT EXISTS idx_chunks_disk     ON chunks(disk_id);
    CREATE INDEX IF NOT EXISTS idx_chunks_status   ON chunks(status);
    "#.to_string()
}
