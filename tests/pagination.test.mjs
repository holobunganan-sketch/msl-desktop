import test from 'node:test';
import assert from 'node:assert/strict';
import {paginate} from '../src/lib/services/pagination.ts';
test('ten thousand records stay reachable through bounded pages without changing source order',()=>{
 const records=Array.from({length:10000},(_,id)=>({id,projectId:3}));
 const visited=[];
 for(let page=1;page<=200;page++){
  const view=paginate(records,page);assert.equal(view.items.length,50);visited.push(...view.items);
 }
 assert.deepEqual(visited,records);assert.equal(records[0].id,0);assert.equal(paginate(records,200).end,10000);
});
test('empty results, shrinking pages and invalid input produce valid ranges',()=>{
 assert.deepEqual(paginate([],10),{items:[],total:0,pages:1,page:1,start:0,end:0});
 assert.equal(paginate([1,2],20).page,1);
 assert.equal(paginate([1,2],NaN,NaN).items.length,2);
 assert.equal(paginate([1,2],-1,0).items.length,1);
});
