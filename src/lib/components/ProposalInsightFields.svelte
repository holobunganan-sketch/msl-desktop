<script lang="ts">
  import {locale} from '$lib/i18n';
  import {categories} from '$lib/services/knowledge';
  import type {ProposalExpert} from '$lib/services/proposalPresentation';
  let {payload,experts,updating=false,disabled=false,onchange}:{payload:Record<string,unknown>;experts:ProposalExpert[];updating?:boolean;disabled?:boolean;onchange:(key:string,value:unknown)=>void}=$props();
  const en=$derived($locale==='en-US');
  const selected=$derived(Array.isArray(payload.categories)?payload.categories.filter((value):value is string=>typeof value==='string'):[]);
  const custom=$derived(selected.filter(value=>!categories.some(category=>category[0]===value)));
  const text=(key:string)=>typeof payload[key]==='string'?payload[key] as string:'';
  function chooseExpert(value:string){const expert=experts.find(expert=>String(expert.id)===value);onchange('expert_id',expert?.id??null);onchange('expert_revision',expert?.revision??null);}
</script>
<div class="insight-fields" data-testid="proposal-insight-fields">
  <label>{en?'Expert':'归属专家'}<select data-testid="review-insight-expert" value={String(payload.expert_id??'')} disabled={disabled||updating} onchange={event=>chooseExpert(event.currentTarget.value)}><option value="">{en?'Choose the expert':'请核对并选择专家'}</option>{#each experts.filter(expert=>!expert.archived||expert.id===payload.expert_id) as expert(expert.id)}<option value={String(expert.id)}>{[expert.name,expert.institution,expert.department].filter(Boolean).join(' · ')}</option>{/each}</select><small>{updating?(en?'This updates the existing insight. Its expert stays unchanged.':'这次修改已有洞察，专家归属保持不变。'):(en?'Confirm the institution and department when names match.':'如专家同名，请同时核对机构和科室。')}</small></label>
  <fieldset disabled={disabled}><legend>{en?'Insight categories':'洞察分类'}</legend>{#each categories as category}<label class="category"><input type="checkbox" checked={selected.includes(category[0])} onchange={event=>onchange('categories',event.currentTarget.checked?[...selected,category[0]]:selected.filter(value=>value!==category[0]))}/>{category[en?2:1]}</label>{/each}{#each custom as value}<label class="category"><input type="checkbox" checked onchange={()=>onchange('categories',selected.filter(category=>category!==value))}/>{value}</label>{/each}</fieldset>
  {#each [['observation','观察 / 专家表达','Observation / expert statement'],['implication','可能的意义','Possible significance'],['uncertainty','待核实','Uncertainty'],['next_question','下次追问','Next question']] as field}<label>{field[en?2:1]}<textarea data-testid={`review-insight-${field[0]}`} value={text(field[0])} rows="3" {disabled} oninput={event=>onchange(field[0],event.currentTarget.value)} placeholder={updating?(en?'Leave unchanged unless edited':'未调整时保留原有内容'):''}></textarea></label>{/each}
</div>
<style>
  .insight-fields{grid-column:1/-1;display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:16px;min-width:0}
  label{display:grid;gap:8px;min-width:0;font-size:1rem;line-height:1.6}small{font-size:.9rem;color:var(--color-muted)}
  select,textarea{width:100%;min-width:0;font:inherit;padding:10px 12px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-text)}select{min-height:44px}textarea{resize:vertical;min-height:108px;line-height:1.7}
  fieldset{margin:0;padding:10px 12px;border:1px solid var(--color-border);border-radius:9px;min-width:0}legend{font-size:1rem}.category{display:flex;align-items:center;gap:8px;margin:5px 0}.category input{width:18px;height:18px;flex-shrink:0}
  @container(max-width:760px){.insight-fields{grid-template-columns:minmax(0,1fr)}}
</style>
