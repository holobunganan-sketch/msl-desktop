<script lang="ts">
 import Modal from './Modal.svelte';
 import {locale} from '$lib/i18n';
 let {message='',error=true,onretry,onclear}:{message?:string;error?:boolean;onretry?:()=>void;onclear?:()=>void}=$props();
 let open=$state(false);
 let en=$derived($locale==='en-US');
</script>
<div class="status-line" class:error aria-live="polite">
 {#if message}<button type="button" class="status-text" title={message} onclick={()=>open=true}>{message}</button><button type="button" class="status-detail" onclick={()=>open=true}>{en?'Details':'详情'}</button>{#if onretry}<button type="button" onclick={onretry}>{en?'Retry':'重试'}</button>{/if}{#if onclear}<button type="button" aria-label={en?'Dismiss':'关闭'} onclick={onclear}>×</button>{/if}{/if}
</div>
<Modal bind:open title={error?(en?'Error details':'错误详情'):(en?'Details':'提示详情')}><p class="full-message">{message}</p></Modal>
<style>
 .status-line{height:28px;min-height:28px;display:flex;align-items:center;gap:8px;min-width:0;width:100%;font-size:13px;color:var(--color-muted)}.status-line.error{color:var(--color-danger)}.status-line button{color:inherit;background:none!important;border:0!important;box-shadow:none!important;padding:0!important;line-height:1.5;font:inherit!important;cursor:pointer;flex-shrink:0}.status-line .status-text{overflow:hidden;white-space:nowrap;text-overflow:ellipsis;text-align:left;min-width:0;flex:1}.status-detail{width:44px}.full-message{white-space:pre-wrap;overflow-wrap:anywhere;line-height:1.75}
</style>
