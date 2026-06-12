import json
import subprocess
import threading

def send_message(proc, msg):
    s = json.dumps(msg)
    payload = f"Content-Length: {len(s)}\r\n\r\n{s}"
    proc.stdin.write(payload.encode('utf-8'))
    proc.stdin.flush()

def read_messages(proc):
    while True:
        line = proc.stdout.readline()
        if not line:
            break
        line = line.decode('utf-8').strip()
        if line.startswith("Content-Length:"):
            length = int(line.split(":")[1].strip())
            proc.stdout.readline() # \r\n
            body = proc.stdout.read(length)
            msg = json.loads(body.decode('utf-8'))
            print("Received:", json.dumps(msg, indent=2))
            if msg.get("method") == "textDocument/publishDiagnostics":
                print("LSP test successful!")
                proc.terminate()
                break

p = subprocess.Popen(
    ["c:/Zeus/Zeus/zeus_compiler/target/release/zeus_compiler.exe", "lsp"],
    stdin=subprocess.PIPE,
    stdout=subprocess.PIPE,
    stderr=subprocess.PIPE
)

t = threading.Thread(target=read_messages, args=(p,))
t.start()

# Initialize
send_message(p, {"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})

# Send didOpen with code that has a ZIR taint leak and unbounded loop
code = """
fn authenticate(sensor: u64) {
    let secret secret_key: u64 = 5;
    while sensor < 10 { // unbounded
        if (secret_key > 0) { // taint leak
            println(1.0);
        }
    }
}
"""

send_message(p, {
    "jsonrpc": "2.0",
    "method": "textDocument/didOpen",
    "params": {
        "textDocument": {
            "uri": "file:///test.zs",
            "text": code
        }
    }
})

t.join()
