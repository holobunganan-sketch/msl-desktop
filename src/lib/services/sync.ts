export type SyncProbeKind = 'empty' | 'unrelated' | 'existing' | 'incomplete';
export type ConnectMode = 'initialize_from_local' | 'use_folder' | 'replace_with_local';

export function connectionActions(probe: { kind: SyncProbeKind }): ConnectMode[] {
  if (probe.kind === 'empty' || probe.kind === 'unrelated') return ['initialize_from_local'];
  if (probe.kind === 'existing') return ['use_folder', 'replace_with_local'];
  return [];
}

export function syncStatusLabel(
  status: { running: boolean; state: { phase: string } },
  locale: 'zh-CN' | 'en-US',
): string {
  const en = locale === 'en-US';
  if (status.running || status.state.phase === 'running') {
    return en ? 'Exchanging data with the sync folder' : '正在与同步文件夹交换数据';
  }
  if (status.state.phase === 'connected' || status.state.phase === 'idle') {
    return en ? 'Connected; waiting for the next sync' : '已连接；等待下一次同步';
  }
  if (status.state.phase === 'failed') return en ? 'Sync needs attention' : '同步需要处理';
  return en ? 'Sync folder not connected' : '尚未连接同步文件夹';
}
