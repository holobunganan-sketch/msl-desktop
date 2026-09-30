import test from 'node:test';
import assert from 'node:assert/strict';
import {DatabaseSync} from 'node:sqlite';
import {readdirSync,readFileSync} from 'node:fs';

function database(){
  const db=new DatabaseSync(':memory:');
  db.exec('PRAGMA foreign_keys=ON');
  for(const file of readdirSync('src-tauri/migrations').filter(x=>/^\d+.*\.sql$/.test(x)).sort()) db.exec(readFileSync(`src-tauri/migrations/${file}`,'utf8'));
  return db;
}

test('legacy projects remain unclassified until an explicit classification is saved',()=>{
  const db=database();
  try {
    db.exec("INSERT INTO works(title,status,created_at,updated_at) VALUES('Existing project','active',1,1)");
    assert.ok(db.prepare('PRAGMA table_info(works)').all().some(x=>x.name==='category'),'project category must be persisted');
    assert.equal(db.prepare('SELECT category FROM works').get().category,null);
    db.exec("UPDATE works SET category='clinical' WHERE id=1");
    assert.equal(db.prepare('SELECT category FROM works').get().category,'clinical');
    assert.throws(()=>db.exec("UPDATE works SET category='guessed' WHERE id=1"));
  } finally {db.close();}
});

test('clinical association preserves the owning project and cleans up with deleted records',()=>{
  const db=database();
  try {
    assert.ok(db.prepare("SELECT name FROM sqlite_master WHERE name='project_relations'").get(),'clinical relations must be persisted');
    db.exec("INSERT INTO works(id,title,status,category,created_at,updated_at) VALUES(1,'Education','active','non_clinical',1,1),(2,'Study','active','clinical',1,1); INSERT INTO tasks(id,work_id,title,created_at,updated_at) VALUES(1,1,'Expert visit',1,1); INSERT INTO project_relations(entity_kind,entity_id,clinical_work_id,created_at,updated_at) VALUES('task',1,2,1,1)");
    assert.equal(db.prepare('SELECT work_id FROM tasks').get().work_id,1);
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM tasks').get().n,1);
    db.exec('DELETE FROM tasks WHERE id=1');
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM project_relations').get().n,0);
    db.exec("INSERT INTO waiting_items(id,work_id,title,waiting_for,started_at,status,created_at,updated_at) VALUES(1,1,'Evidence reply','Colleague',1,'open',1,1); INSERT INTO project_relations(entity_kind,entity_id,clinical_work_id,created_at,updated_at) VALUES('waiting',1,2,1,1); DELETE FROM works WHERE id=2");
    assert.equal(db.prepare('SELECT clinical_work_id FROM project_relations').get().clinical_work_id,null);
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM waiting_items').get().n,1);
  } finally {db.close();}
});

test('moving an associated item directly into a clinical project removes its additional association',()=>{
  const db=database();
  try {
    db.exec("INSERT INTO works(id,title,status,category,created_at,updated_at) VALUES(1,'Education','active','non_clinical',1,1),(2,'Study','active','clinical',1,1),(3,'Other study','active','clinical',1,1); INSERT INTO tasks(id,work_id,title,created_at,updated_at) VALUES(1,1,'Visit',1,1); INSERT INTO project_relations(entity_kind,entity_id,clinical_work_id,created_at,updated_at) VALUES('task',1,2,1,1); UPDATE tasks SET work_id=3 WHERE id=1");
    assert.equal(db.prepare('SELECT clinical_work_id FROM project_relations').get().clinical_work_id,null);
    assert.equal(db.prepare('SELECT work_id FROM tasks').get().work_id,3);
    assert.equal(db.prepare('SELECT COUNT(*) AS n FROM tasks').get().n,1);
  } finally {db.close();}
});
