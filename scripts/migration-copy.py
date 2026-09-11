"""Offline read-only backup; never inspects provider records or starts the app."""
import os,sqlite3
from pathlib import Path
root=Path(__file__).resolve().parents[1]
target=Path(os.environ.get(
    'MSL_MIGRATION_COPY_ROOT',
    root/'.test-runtime'/'stability'/'formal-migration-copy'
))
assert Path(os.environ['APPDATA']).resolve()==(root/'.test-runtime'/'stability'/'appdata').resolve()
source=Path(os.environ['USERPROFILE'])/'AppData'/'Roaming'/'MSLDesktop'/'msl-desktop.db'
assert source.is_file()
target.mkdir(parents=True,exist_ok=True)
destination=target/os.environ.get('MSL_MIGRATION_COPY_NAME','migration.db')
assert not destination.exists(),'Do not overwrite an earlier audit copy'
with sqlite3.connect(source.as_uri()+'?mode=ro',uri=True) as old,sqlite3.connect(destination) as copy:
    old.backup(copy)
print('Created disposable offline database snapshot. No records or keys displayed.')
