// Creates synthetic prior-schema data; never reads a user's database.
import {DatabaseSync} from 'node:sqlite';
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
const root=path.resolve(import.meta.dirname,'..');
assert.ok(process.env.MSL_TEST_ROOT,'MSL_TEST_ROOT is required');
const target=path.resolve(process.env.MSL_TEST_ROOT,'formal-migration-copy','synthetic-v20.db');
assert.ok(!target.startsWith(root+path.sep),'Test database must stay outside source');
assert.ok(!fs.existsSync(target),'Refusing to overwrite an existing database');
fs.mkdirSync(path.dirname(target),{recursive:true});
const db=new DatabaseSync(target);
db.exec('PRAGMA foreign_keys=ON; CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY,name TEXT NOT NULL,applied_at INTEGER NOT NULL)');
for(const file of fs.readdirSync(path.join(root,'src-tauri/migrations')).filter(f=>/^\d{4}_.*\.sql$/.test(f)).sort()){
 const version=Number(file.slice(0,4));if(version>20)continue;
 db.exec('BEGIN IMMEDIATE');
 db.exec(fs.readFileSync(path.join(root,'src-tauri/migrations',file),'utf8'));
 db.prepare('INSERT INTO schema_migrations VALUES(?,?,0)').run(version,file);
 db.exec('COMMIT');
}
db.exec("INSERT INTO works(title,created_at,updated_at) VALUES('Synthetic migration project',1,1); INSERT INTO tasks(work_id,title,created_at,updated_at) VALUES(1,'Synthetic migration task',1,1)");
assert.equal(db.prepare('PRAGMA integrity_check').get().integrity_check,'ok');db.close();
console.log('Created isolated schema-20 fixture with project and task relationships.');
