<script>
 import '../src/lib/styles/app.css';
 import ProviderSettings from '../src/lib/components/ProviderSettings.svelte';
 import QaView from '../src/lib/components/QaView.svelte';
 const qa=new URLSearchParams(location.search).has('qa');
</script>
<div class="view-body" style="height:760px;container-type:inline-size;padding:20px">{#if qa}<QaView/>{:else}<ProviderSettings/>{/if}</div>
