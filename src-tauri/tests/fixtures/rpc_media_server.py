"""Disposable local-media server for ignored aria2c RPC integration tests."""
import functools
import http.server
import json
import pathlib
import subprocess
import sys
import urllib.parse

root = pathlib.Path(sys.argv[1])
root.mkdir(parents=True, exist_ok=True)
def ffmpeg(*args):
    subprocess.run(["ffmpeg", "-hide_banner", "-loglevel", "error", "-y", *args], check=True)

ffmpeg("-f", "lavfi", "-i", "testsrc2=size=320x180:rate=25", "-t", "4", "-an", "-c:v", "libx264", "-g", "25", str(root / "video.mp4"))
ffmpeg("-f", "lavfi", "-i", "sine=frequency=440", "-t", "4", "-vn", "-c:a", "aac", str(root / "audio.m4a"))
ffmpeg("-i", str(root / "video.mp4"), "-c", "copy", "-hls_time", "1", "-hls_list_size", "0", str(root / "index.m3u8"))

class Handler(http.server.SimpleHTTPRequestHandler):
    flaky_calls = 0
    def log_message(self, *args):
        pass

    def do_GET(self):
        path = urllib.parse.urlsplit(self.path).path
        if path == "/unknown.mp4":
            self.send_response(200)
            self.send_header("Content-Type", "video/mp4")
            self.end_headers()
            try:
                self.wfile.write((root / "video.mp4").read_bytes())
            except (BrokenPipeError, ConnectionResetError):
                pass
            return
        if path == "/flaky.ts":
            type(self).flaky_calls += 1
            if type(self).flaky_calls == 1:
                self.send_error(404)
                return
            self.path = "/index1.ts"
        if path in ("/broken.m3u8", "/flaky.m3u8"):
            replacement = "missing.ts" if path == "/broken.m3u8" else "flaky.ts"
            body = (root / "index.m3u8").read_text().replace("index1.ts", replacement).encode()
            self.send_response(200)
            self.send_header("Content-Type", "application/vnd.apple.mpegurl")
            self.send_header("Content-Length", str(len(body)))
            self.end_headers()
            self.wfile.write(body)
            return
        try:
            super().do_GET()
        except (BrokenPipeError, ConnectionResetError):
            pass

server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=str(root)))
print(json.dumps({"url": f"http://127.0.0.1:{server.server_port}"}), flush=True)
server.serve_forever()
