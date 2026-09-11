// Read filenames only. This intentionally never scans credential/data contents.
import {execFileSync} from 'node:child_process';
const args=process.argv.slice(2);
const dangerous=/(^|\/)(\.test-runtime|msl-desktop-data|MSLDesktop|MSLDesktop\.sync|backups|attachments|cache|logs|\.(backup|restore|retention)-[^/]+)(\/|$)|\.(db|sqlite3?)(-(wal|shm))?$|\.(mslbackup|bak|partial-backup)$|(^|\/)(backup-settings|backup-state|sync-settings|sync-state|restore-pending|restore-cleanup-warning|last-restore)\.json$|(^|\/)\.env(\.|$)/i;
const tracked=execFileSync('git',['ls-files','-z'],{encoding:'utf8'}).split('\0').filter(Boolean);
const pending=execFileSync('git',['ls-files','--others','--exclude-standard','-z'],{encoding:'utf8'}).split('\0').filter(Boolean);
const risky=[...new Set([...tracked,...pending].filter(p=>dangerous.test(p)&&!p.endsWith('.env.example')))];
let historical=[];
if(args.includes('--history')) historical=[...new Set(execFileSync('git',['log','--all','--format=','--name-only'],{encoding:'utf8',maxBuffer:20*1024*1024}).split(/\r?\n/).filter(p=>dangerous.test(p)&&!p.endsWith('.env.example')))];
console.log(JSON.stringify({checkedTracked:tracked.length,checkedUntracked:pending.length,riskyPaths:risky,historicalRiskPaths:historical,scope:'Filename-only check. Review docs and images for personal information separately. No keys or file bodies were read.'},null,2));
if(risky.length||historical.length)process.exitCode=1;
