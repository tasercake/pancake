from pathlib import Path
import sys, tomllib
sys.path.insert(0, '/work/pancake/scripts')
from dependency_policy import toml_value
source=tomllib.loads(Path('/work/pancake/Cargo.toml').read_text())
p=Path('/work/pancake/target/p0-sanitizer-probe/rust');(p/'src').mkdir(parents=True,exist_ok=True)
manifest={'package':{'name':'pancake-sanitizer-boundary-probe','version':'0.1.0','edition':'2024'},'workspace':{},'dependencies':{'pancake':{'path':'/work/pancake'},'cubecl':source['dependencies']['cubecl']},'profile':source.get('profile',{})}
for name,value in source.get('patch',{}).get('crates-io',{}).items():
 if 'path' in value:value['path']=str(Path('/work/pancake')/value['path'])
manifest['patch']={'crates-io':source.get('patch',{}).get('crates-io',{})}
s=''
for section,values in manifest.items():s+=f'\n[{section}]\n'+''.join(f'{k} = {toml_value(v)}\n' for k,v in values.items())
(p/'Cargo.toml').write_text(s)
