import hashlib
import json
from pathlib import Path
import subprocess
import sys
import zipfile

import numpy as np
from PIL import Image
import tifffile

COMMAND = [sys.executable, "-m", "bokehforge", "data", "process"]


def fixture(tmp_path, names=("scene/frame2.png", "scene/frame10.png"), layout="directory_stacks", transfer="srgb"):
    root = tmp_path / "data"
    files = []
    for i, name in enumerate(names):
        path = root / "raw/fixture" / name
        path.parent.mkdir(parents=True, exist_ok=True)
        Image.fromarray(np.full((8, 9, 3), 128 + i * 32, dtype=np.uint8)).save(path)
        files.append({"path": name, "sha256": hashlib.sha256(path.read_bytes()).hexdigest(), "size": path.stat().st_size})
    source = {"id": "fixture", "name": "Synthetic test fixture", "status": "available", "layout": layout,
              "transfer": transfer, "reference_type": "none", "files": files}
    lock = tmp_path / "lock.json"
    lock.write_text(json.dumps({"schema_version": 1, "datasets": [source]}))
    return root, lock


def run_convert(root, lock, *extra):
    return subprocess.run([*COMMAND, "--root", str(root), "--lock", str(lock),
                           "--dataset", "fixture", "--reserve-gib", "0", *extra], capture_output=True, text=True)


def test_linear_rgb_stack_preserves_shape_order_and_provenance(tmp_path):
    root, lock = fixture(tmp_path)
    result = run_convert(root, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    manifest = json.loads(next((root / "standardized/fixture").rglob("scene.json")).read_text())
    assert len(manifest["frames"]) == 2
    assert manifest["frames"][0]["source_path"] == "scene/frame2.png"
    output = root / "standardized/fixture" / manifest["scene_directory"] / manifest["frames"][0]["path"]
    image = tifffile.imread(output)
    assert image.shape == (8, 9, 3)
    assert image.dtype == np.float32
    assert np.allclose(image, ((128/255 + 0.055)/1.055)**2.4, atol=1e-7)
    assert manifest["reference_type"] == "none"
    assert manifest["split_role"] == "external_regression"


def test_conversion_rerun_preserves_all_output_bytes_and_mtimes(tmp_path):
    root, lock = fixture(tmp_path)
    assert run_convert(root, lock).returncode == 0
    before = {str(p.relative_to(root)): (hashlib.sha256(p.read_bytes()).hexdigest(), p.stat().st_mtime_ns)
              for p in (root / "standardized").rglob("*") if p.is_file()}
    result = run_convert(root, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    after = {str(p.relative_to(root)): (hashlib.sha256(p.read_bytes()).hexdigest(), p.stat().st_mtime_ns)
             for p in (root / "standardized").rglob("*") if p.is_file()}
    assert before == after


def test_corrupt_output_is_rebuilt_with_identical_bytes(tmp_path):
    root, lock = fixture(tmp_path)
    assert run_convert(root, lock).returncode == 0
    target = next((root / "standardized").rglob("*.tif"))
    checksum = hashlib.sha256(target.read_bytes()).hexdigest()
    target.write_bytes(b"corrupt")
    result = run_convert(root, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    assert hashlib.sha256(target.read_bytes()).hexdigest() == checksum


def test_changed_source_is_rejected_until_downloader_repairs_it(tmp_path):
    root, lock = fixture(tmp_path)
    (root / "raw/fixture/scene/frame2.png").write_bytes(b"corrupt")
    result = run_convert(root, lock)
    assert result.returncode != 0
    assert "checksum" in (result.stdout + result.stderr).lower()


def test_unknown_layout_fails_instead_of_guessing_frames(tmp_path):
    root, lock = fixture(tmp_path, layout="unknown_optical_contract")
    result = run_convert(root, lock)
    assert result.returncode != 0
    assert "No conversion adapter" in result.stderr
    assert not list((root / "standardized").rglob("scene.json"))


def test_converter_uses_only_native_files_in_download_plan(tmp_path):
    names = ("one/1.png", "one/2.png", "two/1.png", "two/2.png")
    root, lock = fixture(tmp_path, names=names)
    assert run_convert(root, lock).returncode == 0
    (root / ".bokehforge/state/download-plan.json").write_text(json.dumps({"schema_version": 1,
        "datasets": [{"id": "fixture", "selected_paths": list(names[:2])}]}))
    result = run_convert(root, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    index = json.loads((root / "standardized/index.json").read_text())
    assert len(index["scenes"]) == 1
    assert len(list((root / ".bokehforge/work/retired").rglob("scene.json"))) == 1


def test_disabled_source_retires_old_active_scenes(tmp_path):
    root, lock = fixture(tmp_path)
    assert run_convert(root, lock).returncode == 0
    config = tmp_path / "config.json"
    config.write_text(json.dumps({"datasets": [{"id": "fixture", "provider": "disabled", "reason": "Excluded by user"}]}))
    result = run_convert(root, lock, "--config", str(config))
    assert result.returncode == 0, result.stdout + result.stderr
    assert json.loads((root / "standardized/index.json").read_text())["scenes"] == []
    report = json.loads((root / ".bokehforge/reports/process-report.json").read_text())["datasets"][0]
    assert report["status"] == "disabled"
    assert len(list((root / ".bokehforge/work/retired").rglob("scene.json"))) == 1


def test_exposure_sequences_exclude_published_fused_outputs(tmp_path):
    names = ("MEFDatabase/source image sequences/Lamp/1.png", "MEFDatabase/source image sequences/Lamp/2.png",
             "MEFDatabase/fused images/Lamp_method1.png", "MEFDatabase/fused images/Lamp_method2.png")
    root, lock = fixture(tmp_path, names=names, layout="exposure_stacks")
    result = run_convert(root, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    scenes = list((root / "standardized").rglob("scene.json"))
    assert len(scenes) == 1
    manifest = json.loads(scenes[0].read_text())
    assert len(manifest["frames"]) == 2
    assert all("source image sequences" in x["source_path"] for x in manifest["frames"])


def test_archive_path_escape_is_rejected(tmp_path):
    root = tmp_path / "data"
    target = root / "raw/fixture/source.zip"
    target.parent.mkdir(parents=True)
    with zipfile.ZipFile(target, "w") as archive:
        archive.writestr("../escape.png", b"bad")
    lock = tmp_path / "lock.json"
    lock.write_text(json.dumps({"datasets": [{"id":"fixture", "status":"available", "layout":"archive_stacks", "files":[
        {"path":"source.zip", "sha256":hashlib.sha256(target.read_bytes()).hexdigest()}]}]}))
    result = run_convert(root, lock)
    assert result.returncode != 0
    assert "unsafe" in (result.stdout + result.stderr).lower()
    assert not (root / ".bokehforge/work/escape.png").exists()


def test_unpaired_input_is_quarantined(tmp_path):
    root, lock = fixture(tmp_path, names=("EDMF/Source_A/bg-A-1.png", "EDMF/Source_B/bg-B-1.png", "EDMF/Source_B/bg-B-2.png"), layout="edmf")
    result = run_convert(root, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    scenes = list((root / "standardized/fixture").rglob("scene.json"))
    assert len(scenes) == 1
    report = json.loads((root / ".bokehforge/reports/process-report.json").read_text())["datasets"][0]
    assert report["quarantined"] == 1


def test_complete_scene_cache_avoids_decoder_on_rerun(tmp_path, monkeypatch):
    from bokehforge.data import process
    root, lock = fixture(tmp_path)
    assert process.main(['--root',str(root),'--lock',str(lock),'--reserve-gib','0']) == 0
    def forbidden(*args):
        raise AssertionError('Already verified scene must not decode again')
    monkeypatch.setattr(process,'decode',forbidden)
    assert process.main(['--root',str(root),'--lock',str(lock),'--reserve-gib','0']) == 0


def test_failed_source_does_not_remain_in_active_index(tmp_path):
    root, lock = fixture(tmp_path)
    assert run_convert(root,lock).returncode == 0
    (root/'raw/fixture/scene/frame2.png').write_bytes(b'bad')
    assert run_convert(root,lock).returncode != 0
    assert json.loads((root/'standardized/index.json').read_text())['scenes'] == []


def test_float_headroom_and_negative_values_are_preserved(tmp_path):
    root, lock = fixture(tmp_path)
    value=json.loads(lock.read_text()); source=value['datasets'][0]; source['transfer']='linear'; source['files']=[]
    for index in range(2):
        path=root/f'raw/fixture/float/frame{index}.tif'; path.parent.mkdir(parents=True,exist_ok=True)
        pixels=np.array([[[-.1,.3,1.7]]],dtype=np.float32)
        tifffile.imwrite(path,pixels,photometric='rgb')
        source['files'].append({'path':f'float/frame{index}.tif','sha256':hashlib.sha256(path.read_bytes()).hexdigest()})
    lock.write_text(json.dumps(value))
    result=run_convert(root,lock)
    assert result.returncode==0, result.stdout+result.stderr
    output=next((root/'standardized').rglob('*.tif'))
    assert np.array_equal(tifffile.imread(output),pixels)


def test_parallel_conversion_matches_serial_pixel_files(tmp_path):
    root, lock=fixture(tmp_path)
    first=run_convert(root,lock,'--workers','1')
    assert first.returncode==0,first.stdout+first.stderr
    before={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'standardized').rglob('*.tif')}
    # Change output directory only; the same original inputs and pinned recipe remain.
    import shutil
    shutil.rmtree(root/'standardized')
    shutil.rmtree(root/'.bokehforge/work/conversion-state')
    second=run_convert(root,lock,'--workers','2')
    assert second.returncode==0,second.stdout+second.stderr
    after={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in (root/'standardized').rglob('*.tif')}
    assert before==after


def test_lfsd_all_focus_images_do_not_become_stack_frames(tmp_path):
    root, lock=fixture(tmp_path,names=('focus_stack.rar-scene/focus_stack/001/1.png','focus_stack.rar-scene/focus_stack/001/2.png',
                                    'all_focus_images.rar-scene/all_focus_images/001.png','all_focus_images.rar-scene/all_focus_images/002.png'),layout='lfsd')
    result=run_convert(root,lock)
    assert result.returncode==0,result.stdout+result.stderr
    scenes=list((root/'standardized/fixture').rglob('scene.json'))
    assert len(scenes)==1
    assert len(json.loads(scenes[0].read_text())['frames'])==2


def test_lfsd_flat_archive_is_split_by_refocus_scene_prefix(tmp_path):
    root,lock=fixture(tmp_path,names=('focus_stack/1__refocus_00.jpg','focus_stack/1__refocus_01.jpg',
                                   'focus_stack/2__refocus_00.jpg','focus_stack/2__refocus_01.jpg'),layout='lfsd')
    result=run_convert(root,lock)
    assert result.returncode==0,result.stdout+result.stderr
    assert len(list((root/'standardized/fixture').rglob('scene.json')))==2


def test_metadata_change_reuses_bit_identical_encoded_pixels(tmp_path, monkeypatch):
    from bokehforge.data import process
    root, lock=fixture(tmp_path)
    assert process.main(['--root',str(root),'--lock',str(lock),'--reserve-gib','0'])==0
    value=json.loads(lock.read_text());value['datasets'][0]['rights_evidence']='Updated source attribution'
    lock.write_text(json.dumps(value))
    def forbidden(*args,**kwargs): raise AssertionError('Unchanged pixels must not be encoded again')
    monkeypatch.setattr(process.tifffile,'imwrite',forbidden)
    assert process.main(['--root',str(root),'--lock',str(lock),'--reserve-gib','0'])==0


def test_fod_groups_each_sample_id_in_flat_directory(tmp_path):
    root,lock=fixture(tmp_path,names=('fs_6/000000_00All.png','fs_6/000000_01All.png',
                                   'fs_6/000001_00All.png','fs_6/000001_01All.png'),layout='fod500')
    result=run_convert(root,lock)
    assert result.returncode==0,result.stdout+result.stderr
    assert len(list((root/'standardized/fixture').rglob('scene.json')))==2


def test_realmff_reference_is_not_a_source_frame(tmp_path):
    root,lock=fixture(tmp_path,names=('RealMFF/imageA/001_A.png','RealMFF/imageB/001_B.png','RealMFF/Fusion/001_F.png'),layout='realmff')
    result=run_convert(root,lock)
    assert result.returncode==0,result.stdout+result.stderr
    value=json.loads(next((root/'standardized/fixture').rglob('scene.json')).read_text())
    assert len(value['frames'])==2
    assert len(value['references'])==1


def test_converter_uses_configured_default_root(tmp_path):
    root, lock=fixture(tmp_path)
    config=tmp_path/'config.json';config.write_text(json.dumps({'defaults':{'data_root':'data','reserve_gib':0}}))
    result=subprocess.run([*COMMAND,'--config',str(config),'--lock',str(lock)],capture_output=True,text=True)
    assert result.returncode==0,result.stdout+result.stderr
    assert (root/'standardized/index.json').exists()


def test_conversion_policy_can_change_without_repinning_source_bytes(tmp_path):
    root,lock=fixture(tmp_path)
    config=tmp_path/'config.json';config.write_text(json.dumps({'datasets':[{'id':'fixture','transfer':'linear'}]}))
    result=run_convert(root,lock,'--config',str(config))
    assert result.returncode==0,result.stdout+result.stderr
    image=tifffile.imread(next((root/'standardized').rglob('*.tif')))
    assert np.allclose(image,128/255,atol=1e-7)


def test_unselected_auxiliary_archive_is_not_required_for_focal_conversion(tmp_path):
    root,lock=fixture(tmp_path)
    value=json.loads(lock.read_text());value['datasets'][0]['conversion_inputs']=['scene/*.png']
    value['datasets'][0]['files'].append({'path':'auxiliary.bin','sha256':'0'*64})
    lock.write_text(json.dumps(value))
    result=run_convert(root,lock)
    assert result.returncode==0,result.stdout+result.stderr


def test_revised_grouping_retires_obsolete_generated_scene(tmp_path):
    root,lock=fixture(tmp_path,names=('focus_stack/1__refocus_00.jpg','focus_stack/1__refocus_01.jpg',
                                   'focus_stack/2__refocus_00.jpg','focus_stack/2__refocus_01.jpg'))
    assert run_convert(root,lock).returncode==0
    config=tmp_path/'config.json';config.write_text(json.dumps({'datasets':[{'id':'fixture','layout':'lfsd'}]}))
    result=run_convert(root,lock,'--config',str(config))
    assert result.returncode==0,result.stdout+result.stderr
    assert len(list((root/'standardized/fixture').rglob('scene.json')))==2
    assert len(list((root/'.bokehforge/work/retired/fixture').rglob('scene.json')))==1
