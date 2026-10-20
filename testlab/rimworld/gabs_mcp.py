#!/usr/bin/env python3
"""Minimal MCP stdio client for the local GABS game bridge (Issue #9 lab).

Speaks JSON-RPC 2.0 over the gabs binary's stdio per MCP 2026-07-28
(stateless per-request metadata): initialize -> notifications/initialized
-> tools/call. The gabs process is the MCP server; the game bridge tools
mirror in as `rimworld_<tool>` (dot/normalization varies; we list and
fuzzy-match). One-shot usage:
    gabs_mcp.py call games_connect '{"gameId":"rimworld"}'
    gabs_mcp.py list
    gabs_mcp.py rimworld_list_languages '{}'
"""
import json
import subprocess
import sys
import time

GABS = "/Users/danielviktorovich/.gabs-tools/gabs-v1.1.2-darwin-arm64/gabs"
GAME = "rimworld"


class McpClient:
    def __init__(self):
        self.proc = subprocess.Popen(
            [GABS, "server", "stdio"], stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL, text=True, bufsize=1,
        )
        self.msg_id = 0

    def _send(self, obj):
        self.proc.stdin.write(json.dumps(obj) + "\n")
        self.proc.stdin.flush()

    def request(self, method, params=None, timeout=90):
        self.msg_id += 1
        rid = self.msg_id
        req = {"jsonrpc": "2.0", "id": rid, "method": method,
               "params": {"_meta": {"progressToken": rid}, **(params or {})}}
        self._send(req)
        deadline = time.time() + timeout
        while time.time() < deadline:
            line = self.proc.stdout.readline()
            if not line:
                raise RuntimeError("gabs mcp closed stdout")
            try:
                msg = json.loads(line)
            except json.JSONDecodeError:
                continue
            if msg.get("id") == rid:
                if "error" in msg:
                    raise RuntimeError(f"{method}: {msg['error']}")
                return msg.get("result")
        raise TimeoutError(f"{method} timed out")

    def notify(self, method):
        self._send({"jsonrpc": "2.0", "method": method})

    def init(self):
        r = self.request("initialize", {
            "protocolVersion": "2026-07-28",
            "capabilities": {},
            "clientInfo": {"name": "rimloc-testlab", "version": "1.0.0"},
        })
        self.notify("notifications/initialized")
        return r

    def tools_list(self):
        tools, cursor = [], None
        while True:
            params = {"cursor": cursor} if cursor else {}
            r = self.request("tools/list", params)
            tools += r.get("tools", [])
            cursor = r.get("nextCursor")
            if not cursor:
                return tools

    def call(self, name, args, timeout=180):
        return self.request("tools/call", {"name": name, "arguments": args}, timeout)

    def close(self):
        try:
            self.proc.terminate()
        except Exception:
            pass


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    cmd = sys.argv[1]
    c = McpClient()
    try:
        info = c.init()
        print("# server:", info.get("serverInfo", {}).get("name"),
              info.get("serverInfo", {}).get("version"),
              "proto", info.get("protocolVersion"), file=sys.stderr)
        # Attach the game bridge FIRST (mirrored tools appear per-session).
        if cmd != "nop":
            try:
                c.call("games_connect", {"gameId": GAME}, timeout=120)
            except Exception as e:
                print("# connect:", e, file=sys.stderr)
        if cmd == "list":
            for t in c.tools_list():
                print(t["name"])
            return 0
        if cmd == "call":
            name, raw = sys.argv[2], (sys.argv[3] if len(sys.argv) > 3 else "{}")
            res = c.call(name, json.loads(raw))
            print(json.dumps(res, ensure_ascii=False, indent=1)[:200000])
            return 0
        # default: fuzzy tool call  gabs_mcp.py <tool> '<json>'
        args = json.loads(sys.argv[2]) if len(sys.argv) > 2 else {}
        wanted = cmd.replace("rimworld/", "rimworld_").replace(".", "_").replace("-", "_")
        names = [t["name"] for t in c.tools_list()]
        cand = [n for n in names if n.lower().replace("_", "").replace(".", "")
                == wanted.lower().replace("_", "").replace(".", "")]
        if not cand:
            cand = [n for n in names if wanted.lower() in n.lower()]
        if not cand:
            print(f"tool not found: {cmd}; known:", file=sys.stderr)
            print("\n".join(names), file=sys.stderr)
            return 3
        res = c.call(cand[0], args)
        print(json.dumps(res, ensure_ascii=False, indent=1)[:200000])
        return 0
    finally:
        c.close()


if __name__ == "__main__":
    sys.exit(main())
