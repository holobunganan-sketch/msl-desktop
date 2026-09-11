<script lang="ts">
 import '../src/lib/styles/app.css';
 import AppButton from '../src/lib/components/ui/AppButton.svelte';
 import ToastHost from '../src/lib/components/ui/ToastHost.svelte';
 import Modal from '../src/lib/components/ui/Modal.svelte';
 import StatusLine from '../src/lib/components/ui/StatusLine.svelte';
 import TypedDeleteDialog from '../src/lib/components/ui/TypedDeleteDialog.svelte';
 import {addToast,dismissToast} from '../src/lib/stores/toast';
 let loading=$state(false),open=$state(false),toast=0;
 let deleteOpen=$state(false),deleted=$state(false);
</script>
<main style="display:flex;flex-direction:column;height:700px">
 <div><button id="loading" onclick={()=>loading=!loading}>loading</button><button id="notify" onclick={()=>toast=addToast('测试通知'.repeat(150),'error',0)}>notify</button><button id="dismiss" onclick={()=>dismissToast(toast)}>dismiss</button><button id="modal" onclick={()=>open=true}>modal</button></div>
 <ToastHost/>
 <StatusLine message={loading?'长错误信息'.repeat(400):''}/>
 <section id="anchor" style="flex:1;min-height:0"><AppButton testid="subject" {loading}>保存</AppButton><AppButton testid="neighbor">下一步</AppButton></section>
 <button id="delete-open" onclick={()=>deleteOpen=true}>Delete</button><span id="deleted">{deleted?'deleted':'retained'}</span>
 <TypedDeleteDialog bind:open={deleteOpen} name="测试专家" title="删除专家" onconfirm={()=>deleted=true}/>
 <Modal bind:open title="测试弹窗"><input aria-label="名称"/><button id="last">末尾</button></Modal>
</main>
