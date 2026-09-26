<script lang="ts">
 import {locale} from '$lib/i18n';
 import {latestCompletion,restoreManual} from '$lib/stores/manualCompletions';
 import {navigateTo} from '$lib/services/navigation';
 import StatusLine from './ui/StatusLine.svelte';
 let {error='',onrefresh=()=>{}}:{error?:string;onrefresh?:()=>void|Promise<void>}=$props();
 let busy=$state(false),recoveryError=$state('');let en=$derived($locale==='en-US');
 async function undo(){const receipt=$latestCompletion;if(!receipt||busy)return;busy=true;recoveryError='';try{await restoreManual(receipt);await onrefresh();}catch(e){recoveryError=String(e);}finally{busy=false;}}
</script>
<div class="completion-feedback" data-testid="manual-completion-feedback" aria-live="polite">
 {#if error||recoveryError}<StatusLine message={error||recoveryError} onclear={()=>recoveryError=''}/>
 {:else if $latestCompletion}<span class="completion-copy" title={$latestCompletion.title}>{$latestCompletion.undone_at?(en?'Restored':'已恢复'):(en?'Completed':'已完成')} · {$latestCompletion.title}</span>
 {#if !$latestCompletion.undone_at}<button data-testid="manual-completion-undo" disabled={busy} onclick={undo}>{$latestCompletion.entity_kind==='waiting'?(en?'Restore waiting':'恢复为等待中'):(en?'Restore unfinished':'恢复为未完成')}</button>{/if}
 <button onclick={()=>navigateTo({view:'matters',section:'done'})}>{en?'Recent completions':'最近完成'}</button>{/if}
</div>
<style>.completion-feedback{min-height:42px;display:flex;align-items:center;gap:12px;min-width:0;font-size:14px;color:var(--color-muted)}.completion-copy{flex:1;min-width:0;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}.completion-feedback button{flex:0 0 auto;min-height:40px;border:0;background:none;color:var(--color-primary);font:inherit;cursor:pointer;padding:4px}.completion-feedback button:disabled{opacity:.55}@container(max-width:440px){.completion-feedback{flex-wrap:wrap}.completion-copy{flex-basis:100%}}</style>
