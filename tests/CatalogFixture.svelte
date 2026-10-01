<script>
 import '../src/lib/styles/app.css';
 import ProviderSettings from '../src/lib/components/ProviderSettings.svelte';
 import QaView from '../src/lib/components/QaView.svelte';
 const qa=new URLSearchParams(location.search).has('qa');
</script>
<div class="app-shell"><div class="content-scroll" style="height:100%;overflow:auto"><div class="view-body" style="min-height:100%;height:100%;container-type:inline-size;padding:20px">{#if qa}<QaView/>{:else}<ProviderSettings/>{/if}</div></div></div>
