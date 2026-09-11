<script lang="ts">
 import type {Snippet} from 'svelte';
 import {locale} from '$lib/i18n';
 import Modal from './Modal.svelte';
 import AppButton from './AppButton.svelte';
 let {open=$bindable(false),name,title,busy=false,error='',testid='typed-delete',children,onconfirm}:{open?:boolean;name:string;title:string;busy?:boolean;error?:string;testid?:string;children?:Snippet;onconfirm:(name:string)=>void}=$props();
 let confirmation=$state(''),composing=$state(false);
 let en=$derived($locale==='en-US');
 $effect(()=>{if(open){name;confirmation='';composing=false;}});
</script>
<Modal bind:open {title} dismissible={!busy}>
 <div class="delete-info">{#if children}{@render children()}{/if}</div>
 <label class="delete-label">{en?'Enter the full name to confirm:':'请输入完整名称确认：'}<strong>{name}</strong>
  <input data-testid={testid+'-name'} value={confirmation} oninput={e=>confirmation=e.currentTarget.value} oncompositionstart={()=>composing=true} oncompositionend={()=>composing=false} disabled={busy} autocomplete="off" spellcheck="false"/>
 </label>
 <div class="delete-error" role="status">{error}</div>
 {#snippet footer()}
  <AppButton variant="secondary" disabled={busy} onclick={()=>open=false}>{en?'Cancel':'取消'}</AppButton>
  <AppButton testid={testid+'-confirm'} variant="danger" loading={busy} disabled={composing||!name||confirmation.trim()!==name} onclick={()=>{if(!composing&&confirmation.trim()===name)onconfirm(confirmation.trim());}}>{en?'Delete permanently':'确认删除'}</AppButton>
 {/snippet}
</Modal>
<style>
 .delete-info{line-height:1.8;color:var(--color-muted);margin-bottom:18px}.delete-label{display:grid;gap:10px;font-size:15px}.delete-label strong{overflow-wrap:anywhere;font-weight:550;color:var(--color-text)}.delete-label input{width:100%;padding:8px 12px;font-size:16px}.delete-error{min-height:48px;max-height:120px;overflow:auto;white-space:pre-wrap;overflow-wrap:anywhere;font-size:14px;color:var(--color-danger);padding-top:10px}
</style>
