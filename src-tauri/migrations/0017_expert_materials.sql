CREATE TABLE material_blobs (
 hash TEXT PRIMARY KEY,relative_path TEXT NOT NULL UNIQUE,byte_size INTEGER NOT NULL,
 media_type TEXT NOT NULL,created_at INTEGER NOT NULL,cleanup_error TEXT NOT NULL DEFAULT ''
);
CREATE TABLE kol_materials (
 id INTEGER PRIMARY KEY AUTOINCREMENT,expert_id INTEGER NOT NULL REFERENCES kol_experts(id) ON DELETE CASCADE,
 blob_hash TEXT NOT NULL REFERENCES material_blobs(hash),filename TEXT NOT NULL,description TEXT NOT NULL DEFAULT '',
 status TEXT NOT NULL DEFAULT 'queued' CHECK(status IN ('queued','reading','ready','partial','unsupported','failed')),
 error TEXT NOT NULL DEFAULT '',reader_key TEXT NOT NULL DEFAULT '',revision INTEGER NOT NULL DEFAULT 1,created_at INTEGER NOT NULL,
 UNIQUE(expert_id,blob_hash)
);
CREATE TABLE material_segments (
 id INTEGER PRIMARY KEY AUTOINCREMENT,material_id INTEGER NOT NULL REFERENCES kol_materials(id) ON DELETE CASCADE,
 ordinal INTEGER NOT NULL,locator TEXT NOT NULL,text TEXT NOT NULL,kind TEXT NOT NULL CHECK(kind IN ('extracted_text','model_interpretation')),
 UNIQUE(material_id,ordinal)
);
CREATE TABLE material_readings (
 blob_hash TEXT NOT NULL REFERENCES material_blobs(hash) ON DELETE CASCADE,reader_key TEXT NOT NULL,
 segments_json TEXT NOT NULL,status TEXT NOT NULL,note TEXT NOT NULL DEFAULT '',PRIMARY KEY(blob_hash,reader_key)
);
CREATE VIRTUAL TABLE material_search USING fts5(text,content='material_segments',content_rowid='id',tokenize='unicode61');
CREATE TRIGGER material_search_insert AFTER INSERT ON material_segments BEGIN INSERT INTO material_search(rowid,text) VALUES(new.id,new.text); END;
CREATE TRIGGER material_search_delete AFTER DELETE ON material_segments BEGIN INSERT INTO material_search(material_search,rowid,text) VALUES('delete',old.id,old.text); END;
CREATE TRIGGER material_search_update AFTER UPDATE ON material_segments BEGIN INSERT INTO material_search(material_search,rowid,text) VALUES('delete',old.id,old.text); INSERT INTO material_search(rowid,text) VALUES(new.id,new.text); END;
CREATE INDEX kol_material_expert ON kol_materials(expert_id,status);

CREATE TABLE material_import_journal(stage TEXT PRIMARY KEY,destination TEXT,created_at INTEGER NOT NULL);
