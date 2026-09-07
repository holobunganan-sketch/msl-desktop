<script lang="ts">
  import {locale,t,translateStatus} from "$lib/i18n";
  import {proposalStatuses} from "$lib/services/proposalPayload";
  let {kind,value,disabled=false,onchange}:{kind:string;value:string;disabled?:boolean;onchange:(value:string|null)=>void}=$props();
</script>
<label><span>{t("common.status",{},$locale)}</span>
  <select data-testid={`proposal-${kind}-status`} {value} {disabled} onchange={event=>onchange(event.currentTarget.value||null)}>
    <option value="">{t("scope.statusUnchanged",{},$locale)}</option>
    {#if value&&!proposalStatuses[kind]?.includes(value)}<option value={value} disabled>{value} · {$locale==="en-US"?"Choose a valid status":"请选择有效状态"}</option>{/if}
    {#each proposalStatuses[kind]??[] as status}<option value={status}>{translateStatus(status,$locale)}</option>{/each}
  </select>
</label>
<style>
  label{display:grid;gap:6px;min-width:0;font-size:14px;color:var(--color-muted)}
  select{width:100%;min-width:0;min-height:42px;padding:9px 10px;border:1px solid var(--color-border);border-radius:9px;background:var(--color-surface);color:var(--color-text);font:inherit;font-size:15px}
</style>
