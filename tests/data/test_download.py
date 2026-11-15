import hashlib
import http.server
import json
import subprocess
import sys
import threading
from pathlib import Path

import pytest

COMMAND = [sys.executable, "-m", "bokehforge", "data", "download"]


def test_drive_download_uses_space_checked_http_stream(tmp_path, origin, monkeypatch):
    from bokehforge.data import download
    calls = []
    monkeypatch.setattr(download, "drive_url", lambda client, identity: origin[0])
    original_check = download.check_space
    def checked(path, needed, reserve):
        calls.append(needed)
        original_check(path, needed, reserve)
    monkeypatch.setattr(download, "check_space", checked)
    item = {"path": "source.bin", "drive_id": "fixture", "size": None, "sha256": None}
    target = tmp_path / "source.bin"
    with download.session() as client:
        assert download.acquire(client, target, item, 0, 1)
    assert target.read_bytes() == origin[1]["body"]
    assert len(origin[1]["body"]) in calls
    assert item["sha256"] == hashlib.sha256(target.read_bytes()).hexdigest()


@pytest.fixture
def origin():
    state = {"body": b"source-stack\x00" * 1024, "requests": [], "range": True}

    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            value = self.headers.get("Range")
            state["requests"].append(value)
            start = int(value.split("=")[1].split("-")[0]) if value and state["range"] else 0
            if state.get("range_416") and value and start >= len(state["body"]):
                self.send_response(416)
                self.send_header("Content-Range", f"bytes */{len(state['body'])}")
                self.send_header("Content-Length", "0")
                self.end_headers()
                return
            end = len(state['body'])-1
            if value and state['range'] and value.split('-')[-1]:
                end = min(end,int(value.split('-')[-1]))
            partial = bool(value and state['range'])
            self.send_response(206 if partial else 200)
            if partial:
                self.send_header("Content-Range", f"bytes {start}-{end}/{len(state['body'])}")
            self.send_header("Content-Length", str(end-start+1))
            if state.get('content_type'):
                self.send_header('Content-Type',state['content_type'])
            self.end_headers()
            self.wfile.write(state["body"][start:end+1])

        def log_message(self, *args):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield f"http://127.0.0.1:{server.server_port}/source.bin", state
    server.shutdown()
    thread.join()


def locked_source(tmp_path, origin, checksum=None):
    url, state = origin
    lock = {
        "schema_version": 1,
        "datasets": [{
            "id": "fixture", "name": "Local fixture", "status": "available",
            "layout": "directory_stacks", "transfer": "srgb",
            "files": [{"path": "source.bin", "url": url, "size": len(state["body"]),
                       "sha256": checksum or hashlib.sha256(state["body"]).hexdigest()}],
        }],
    }
    path = tmp_path / "sources.lock.json"
    path.write_text(json.dumps(lock))
    return path


def run_download(tmp_path, lock):
    return subprocess.run(
        [*COMMAND, "--root", str(tmp_path / "data"), "--lock", str(lock),
         "--dataset", "fixture", "--reserve-gib", "0", "--workers", "1"],
        capture_output=True, text=True,
    )


@pytest.mark.parametrize("flags", [[], ["--refresh-lock"], ["--refresh-lock", "--resolve-only"]])
def test_disabled_source_never_contacts_old_download_route(tmp_path, origin, flags):
    lock = locked_source(tmp_path, origin)
    config = tmp_path / "config.json"
    config.write_text(json.dumps({"datasets": [{"id": "fixture", "provider": "disabled",
                                               "reason": "Excluded by user"}]}))
    result = subprocess.run([*COMMAND, "--root", str(tmp_path / "data"),
        "--config", str(config), "--lock", str(lock), "--dataset", "fixture", "--reserve-gib", "0", *flags],
        capture_output=True, text=True)
    assert result.returncode == 0, result.stdout + result.stderr
    assert origin[1]["requests"] == []
    source = json.loads(lock.read_text())["datasets"][0]
    assert source["status"] == "disabled"
    assert source["files"] == []
    assert "url" not in source
    if not flags or "--resolve-only" not in flags:
        plan = json.loads((tmp_path / "data/.bokehforge/state/download-plan.json").read_text())
        assert plan["datasets"][0]["selected_paths"] == []


def test_verified_rerun_performs_no_network_or_rewrite(tmp_path, origin):
    lock = locked_source(tmp_path, origin)
    result = run_download(tmp_path, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    target = tmp_path / "data/raw/fixture/source.bin"
    first = target.stat().st_mtime_ns
    requests = len(origin[1]["requests"])
    result = run_download(tmp_path, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    assert target.stat().st_mtime_ns == first
    assert len(origin[1]["requests"]) == requests


def test_corrupt_file_is_repaired_from_pinned_source(tmp_path, origin):
    lock = locked_source(tmp_path, origin)
    assert run_download(tmp_path, lock).returncode == 0
    target = tmp_path / "data/raw/fixture/source.bin"
    target.write_bytes(b"corrupt")
    result = run_download(tmp_path, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    assert target.read_bytes() == origin[1]["body"]


@pytest.mark.parametrize("range_supported", [True, False])
def test_partial_download_resumes_or_restarts_safely(tmp_path, origin, range_supported):
    origin[1]["range"] = range_supported
    lock = locked_source(tmp_path, origin)
    part = tmp_path / "data/raw/fixture/source.bin.part"
    part.parent.mkdir(parents=True)
    part.write_bytes(origin[1]["body"][:77])
    result = run_download(tmp_path, lock)
    assert result.returncode == 0, result.stdout + result.stderr
    assert part.with_suffix("").read_bytes() == origin[1]["body"]
    assert origin[1]["requests"][0] == "bytes=77-"


def test_checksum_failure_never_publishes_final_file(tmp_path, origin):
    lock = locked_source(tmp_path, origin, checksum="0" * 64)
    result = run_download(tmp_path, lock)
    assert result.returncode != 0
    assert not (tmp_path / "data/raw/fixture/source.bin").exists()
    assert "checksum" in (result.stdout + result.stderr).lower()


def test_refresh_preserves_other_dataset_pins(tmp_path, origin):
    lock = locked_source(tmp_path, origin)
    value = json.loads(lock.read_text())
    preserved = {"id": "other", "status": "unavailable", "reason": "fixture", "files": []}
    value["datasets"].append(preserved)
    lock.write_text(json.dumps(value))
    config = tmp_path / "config.json"
    config.write_text(json.dumps({"datasets": [{"id":"fixture", "provider":"files", "files":value['datasets'][0]['files']}]}))
    result = subprocess.run([*COMMAND, "--root", str(tmp_path / "data"), "--lock", str(lock),
                             "--config", str(config), "--dataset", "fixture", "--refresh-lock", "--resolve-only"], capture_output=True, text=True)
    assert result.returncode == 0, result.stdout + result.stderr
    assert preserved in json.loads(lock.read_text())["datasets"]


def test_probe_uses_downloader_without_creating_dataset_payload(tmp_path, origin):
    result=subprocess.run([*COMMAND,'--root',str(tmp_path/'data'),'--probe-url',origin[0]],capture_output=True,text=True)
    assert result.returncode==0,result.stdout+result.stderr
    assert json.loads(result.stdout)['status']==200
    assert not (tmp_path/'data/raw').exists()


def test_gzip_drive_confirmation_is_decoded_before_parsing():
    import gzip
    import requests
    from bokehforge.data import download
    class Handler(http.server.BaseHTTPRequestHandler):
        def do_GET(self):
            self.send_response(200)
            if self.path=='/confirm':
                body=gzip.compress(b'<form id="download-form" action="/binary"><input name="confirm" value="yes"></form>')
                self.send_header('Content-Type','text/html'); self.send_header('Content-Encoding','gzip')
            else:
                body=b'archive'; self.send_header('Content-Type','application/zip')
            self.send_header('Content-Length',str(len(body))); self.end_headers(); self.wfile.write(body)
        def log_message(self,*args): pass
    server=http.server.ThreadingHTTPServer(('127.0.0.1',0),Handler)
    thread=threading.Thread(target=server.serve_forever,daemon=True); thread.start()
    class Client:
        def get(self,url,**kwargs):
            path='/confirm' if 'drive.google.com' in url else '/binary'
            return requests.get(f'http://127.0.0.1:{server.server_port}'+path,**kwargs)
    try:
        assert '/binary' in download.drive_url(Client(),'fixture')
    finally:
        server.shutdown(); thread.join()


def test_public_drive_listing_keeps_more_than_fifty_entries():
    from bokehforge.data import download
    assert hasattr(download, 'parse_drive_listing'), 'Public Drive listing parser is missing'
    html='<html><title>near</title>'+''.join(
        f'<div class="flip-entry" id="entry-file{i}"><a href="https://drive.google.com/file/d/file{i}/view"><div class="flip-entry-title">{i:05d}.png</div></a></div>'
        for i in range(150))+'</html>'
    entries=download.parse_drive_listing(html)
    assert len(entries)==150
    assert entries[-1]=={'id':'file149','name':'00149.png','folder':False}


def test_html_error_page_is_not_accepted_as_archive(tmp_path, origin):
    origin[1]['content_type']='text/html'
    lock=locked_source(tmp_path,origin)
    value=json.loads(lock.read_text()); value['datasets'][0]['files'][0]['path']='source.zip'
    value['datasets'][0]['files'][0]['sha256']=None
    lock.write_text(json.dumps(value))
    result=run_download(tmp_path,lock)
    assert result.returncode != 0
    assert not (tmp_path/'data/raw/fixture/source.zip').exists()


def test_complete_partial_is_published_without_network(tmp_path,origin):
    lock=locked_source(tmp_path,origin)
    part=tmp_path/'data/raw/fixture/source.bin.part';part.parent.mkdir(parents=True);part.write_bytes(origin[1]['body'])
    result=run_download(tmp_path,lock)
    assert result.returncode==0,result.stdout+result.stderr
    assert origin[1]['requests']==[]
    assert part.with_suffix('').read_bytes()==origin[1]['body']


def test_unhashed_complete_partial_restarts_after_http_416(tmp_path, origin):
    origin[1]["range_416"] = True
    from bokehforge.data import download
    part = tmp_path / "data/raw/fixture/source.bin.part"
    part.parent.mkdir(parents=True)
    part.write_bytes(origin[1]["body"])
    item = {"path": "source.bin", "url": origin[0], "size": None, "sha256": None}
    with download.session() as client:
        assert download.acquire(client, part.with_suffix(""), item, 0, 1)
    assert part.with_suffix("").read_bytes() == origin[1]["body"]
    assert origin[1]["requests"][:2] == [f"bytes={len(origin[1]['body'])}-", None]


def test_configuration_controls_default_storage_root(tmp_path,origin):
    lock=locked_source(tmp_path,origin)
    value=json.loads(lock.read_text())
    config=tmp_path/'config.json';config.write_text(json.dumps({'defaults':{'data_root':'configured-data','reserve_gib':0},'datasets':value['datasets']}))
    result=subprocess.run([*COMMAND,'--config',str(config),'--lock',str(lock),'--resolve-only'],capture_output=True,text=True)
    assert result.returncode==0,result.stdout+result.stderr
    assert (tmp_path/'configured-data/.bokehforge/locks/data.lock').exists()


def test_global_budget_excludes_whole_source_and_preserves_cached_originals(tmp_path, origin):
    lock = locked_source(tmp_path, origin)
    value = json.loads(lock.read_text())
    first = value["datasets"][0]
    other = {**first, "id": "other", "files": [{**first["files"][0]}]}
    value["datasets"].append(other)
    lock.write_text(json.dumps(value))
    size = len(origin[1]["body"])
    config = tmp_path / "config.json"
    config.write_text(json.dumps({"defaults": {"max_download_bytes": size}, "datasets": [first, other]}))
    cached = tmp_path / "data/raw/other/source.bin"
    cached.parent.mkdir(parents=True)
    cached.write_bytes(origin[1]["body"])
    result = subprocess.run([*COMMAND, "--root", str(tmp_path / "data"), "--lock", str(lock),
                             "--config", str(config), "--reserve-gib", "0"], capture_output=True, text=True)
    assert result.returncode == 0, result.stdout + result.stderr
    plan = json.loads((tmp_path / "data/.bokehforge/state/download-plan.json").read_text())
    assert plan["selected_bytes"] == size
    assert plan["datasets"][0]["selected_paths"] == ["source.bin"]
    assert plan["datasets"][1]["selected_paths"] == []
    assert not cached.exists()
    assert (tmp_path / "data/.bokehforge/cache/raw/other/source.bin").read_bytes() == origin[1]["body"]
    assert len(origin[1]["requests"]) == 1
    configuration = json.loads(config.read_text())
    configuration["defaults"]["max_download_bytes"] = size * 2
    config.write_text(json.dumps(configuration))
    again = subprocess.run([*COMMAND, "--root", str(tmp_path / "data"), "--lock", str(lock),
                            "--config", str(config), "--reserve-gib", "0"], capture_output=True, text=True)
    assert again.returncode == 0, again.stdout + again.stderr
    assert cached.read_bytes() == origin[1]["body"]
    assert len(origin[1]["requests"]) == 1


def test_archive_budget_never_thins_an_archive(tmp_path):
    from bokehforge.data import download
    source = {"id": "fixture", "provider": "figshare", "status": "available",
              "files": [{"path": f"stack-{i}.zip", "size": 60} for i in range(3)]}
    config = {"datasets": [{"id": "fixture", "acquisition_max_bytes": 100}]}
    plan = download.build_download_plan({"datasets": [source]}, config, 500)
    assert plan["selected_bytes"] == 60
    assert plan["datasets"][0]["selected_paths"] == ["stack-0.zip"]


def test_temporary_resolution_failure_is_retried_on_rerun(tmp_path,origin):
    lock=locked_source(tmp_path,origin)
    value=json.loads(lock.read_text()); source=value['datasets'][0]
    config=tmp_path/'config.json';config.write_text(json.dumps({'datasets':[{**source,'provider':'files'}]}))
    value['datasets'][0]={**source,'status':'resolve_failed','reason':'temporary outage','files':[]}
    lock.write_text(json.dumps(value))
    result=subprocess.run([*COMMAND,'--root',str(tmp_path/'data'),'--lock',str(lock),'--config',str(config),
                           '--dataset','fixture','--reserve-gib','0'],capture_output=True,text=True)
    assert result.returncode==0,result.stdout+result.stderr
    assert (tmp_path/'data/raw/fixture/source.bin').read_bytes()==origin[1]['body']
