#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Mock gateway server that receives alarm notifications."""
import sys
import io
sys.stdout = io.TextIOWrapper(sys.stdout.buffer, encoding='utf-8', errors='replace')
sys.stderr = io.TextIOWrapper(sys.stderr.buffer, encoding='utf-8', errors='replace')
from http.server import HTTPServer, BaseHTTPRequestHandler
import json
import datetime

class GatewayHandler(BaseHTTPRequestHandler):
    def do_POST(self):
        content_length = int(self.headers.get('Content-Length', 0))
        body = self.rfile.read(content_length) if content_length else b''

        now = datetime.datetime.now().strftime('%H:%M:%S')
        print(f"\n{'='*50}")
        print(f"[{now}] NOTIFICATION RECEIVED at {self.path}")
        print(f"  X-Alarm-Id:   {self.headers.get('X-Alarm-Id', '-')}")
        print(f"  X-Alarm-Name: {self.headers.get('X-Alarm-Name', '-')}")
        try:
            parsed = json.loads(body) if body else None
            print(f"  Body: {json.dumps(parsed, ensure_ascii=False)}")
        except Exception:
            print(f"  Body (raw): {body}")
        print(f"{'='*50}\n")

        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.end_headers()
        self.wfile.write(b'{"status":"ok"}')

    def log_message(self, format, *args):
        pass  # suppress default access log

if __name__ == '__main__':
    port = 9090
    server = HTTPServer(('0.0.0.0', port), GatewayHandler)
    print(f"Mock gateway listening on http://127.0.0.1:{port}/notify")
    print("Waiting for alarm notifications...\n")
    server.serve_forever()
