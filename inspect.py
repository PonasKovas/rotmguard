import struct
import sys
import os
import json
import http.server
import socketserver
import webbrowser
from urllib.parse import urlparse, parse_qs

# Configuration
FILE_PATH = "/home/mykolas/Projects/rotmguard/packet_data-2026-01-23 14:00:21.275877450 +02:00"  # CHANGE THIS to your log file path
PORT = 8000
CHUNK_SIZE = 50  # How many packets to load per page

class PacketIndexer:
    def __init__(self, filepath):
        self.filepath = filepath
        self.offsets = []
        self.total_packets = 0
        print(f"[+] Indexing {filepath}... (this might take a moment)")
        self._build_index()
        print(f"[+] Indexing complete. Found {self.total_packets} packets.")

    def _build_index(self):
        """Scans the file to find where each packet begins."""
        self.offsets = []
        current_offset = 0
        filesize = os.path.getsize(self.filepath)

        with open(self.filepath, "rb") as f:
            while current_offset < filesize:
                try:
                    # We only need to read the header to find the next offset
                    # Header: 1 (dir) + 16 (time) + 4 (len) = 21 bytes
                    if current_offset + 21 > filesize:
                        break

                    f.seek(current_offset + 17) # Skip dir (1) + time (16) to get length
                    len_bytes = f.read(4)
                    if len(len_bytes) < 4: break

                    packet_len = struct.unpack('<I', len_bytes)[0]

                    self.offsets.append(current_offset)

                    # Calculate next offset: Header (21) + Data (packet_len)
                    current_offset += 21 + packet_len

                except Exception as e:
                    print(f"Error indexing at offset {current_offset}: {e}")
                    break

        self.total_packets = len(self.offsets)

    def get_packets(self, start_index, count):
        """Reads specific packets from the binary file."""
        results = []
        if start_index >= self.total_packets:
            return results

        end_index = min(start_index + count, self.total_packets)

        with open(self.filepath, "rb") as f:
            for i in range(start_index, end_index):
                offset = self.offsets[i]
                f.seek(offset)

                # Parse Direction (1 byte)
                dir_byte = f.read(1)
                direction = "C2S" if dir_byte == b'c' else "S2C"

                # Parse Time (128 bit LE)
                # Python struct doesn't have standard 128-bit support, use int.from_bytes
                time_bytes = f.read(16)
                time_ns = int.from_bytes(time_bytes, byteorder='little')

                # Parse Length (u32 LE)
                len_bytes = f.read(4)
                packet_len = struct.unpack('<I', len_bytes)[0]

                # Parse Data
                data = f.read(packet_len)

                # Format for JSON
                hex_data = " ".join(f"{b:02X}" for b in data)
                # Limit very long hex displays for performance
                if len(hex_data) > 100:
                    hex_display = hex_data[:100] + "..."
                else:
                    hex_display = hex_data

                results.append({
                    "id": i,
                    "dir": direction,
                    "time_ms": f"{time_ns / 1_000_000:.2f}", # Display as milliseconds
                    "len": packet_len,
                    "hex": hex_display,
                    "full_hex": hex_data # Send full data for detailed inspection
                })
        return results

# Initialize Indexer Global
indexer = None

class RequestHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        parsed = urlparse(self.path)

        # API Endpoint
        if parsed.path == "/api/packets":
            query = parse_qs(parsed.query)
            start = int(query.get('start', [0])[0])
            count = int(query.get('count', [CHUNK_SIZE])[0])

            data = indexer.get_packets(start, count)

            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps({
                "total": indexer.total_packets,
                "packets": data
            }).encode())
            return

        # Serve the HTML App
        if parsed.path == "/" or parsed.path == "/index.html":
            self.send_response(200)
            self.send_header('Content-type', 'text/html')
            self.end_headers()
            self.wfile.write(HTML_TEMPLATE.encode())
            return

        self.send_error(404)

    def log_message(self, format, *args):
        return # Silence console logging for cleaner output

# HTML/JS Frontend (Embedded for single-file convenience)
HTML_TEMPLATE = """
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Packet Inspector</title>
    <style>
        body { font-family: 'Segoe UI', Tahoma, Geneva, Verdana, sans-serif; background: #1e1e1e; color: #d4d4d4; margin: 0; display: flex; flex-direction: column; height: 100vh; }
        header { background: #007acc; padding: 10px 20px; color: white; display: flex; justify-content: space-between; align-items: center; }
        #controls { padding: 10px; background: #252526; border-bottom: 1px solid #333; display: flex; gap: 10px; align-items: center; }
        button { background: #0e639c; color: white; border: none; padding: 5px 15px; cursor: pointer; }
        button:disabled { background: #444; cursor: not-allowed; }
        button:hover:not(:disabled) { background: #1177bb; }

        #table-container { flex: 1; overflow-y: auto; padding: 0; }
        table { width: 100%; border-collapse: collapse; table-layout: fixed; }
        th { background: #2d2d2d; text-align: left; padding: 8px; position: sticky; top: 0; border-bottom: 1px solid #444; }
        td { padding: 5px 8px; border-bottom: 1px solid #333; font-family: 'Consolas', monospace; font-size: 13px; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }

        /* Column Widths */
        .col-id { width: 60px; color: #569cd6; }
        .col-dir { width: 60px; font-weight: bold; }
        .col-time { width: 100px; color: #b5cea8; }
        .col-len { width: 80px; color: #ce9178; }
        .col-data { color: #9cdcfe; }

        tr:hover { background: #2a2d2e; }

        .dir-c2s { color: #4ec9b0; }
        .dir-s2c { color: #c586c0; }

        /* Modal for details */
        #detail-view { display: none; position: fixed; top: 50%; left: 50%; transform: translate(-50%, -50%); width: 80%; height: 70%; background: #252526; border: 1px solid #007acc; padding: 20px; box-shadow: 0 0 20px rgba(0,0,0,0.5); z-index: 100; flex-direction: column; }
        #detail-content { flex: 1; overflow: auto; white-space: pre-wrap; word-break: break-all; font-family: monospace; background: #1e1e1e; padding: 10px; border: 1px solid #333; margin-top: 10px;}
        #overlay { display: none; position: fixed; top: 0; left: 0; width: 100%; height: 100%; background: rgba(0,0,0,0.5); z-index: 99; }
    </style>
</head>
<body>

<header>
    <span>Packet Log Inspector</span>
    <span id="status">Loading...</span>
</header>

<div id="controls">
    <button onclick="prevPage()">Previous</button>
    <span id="page-indicator">Page 1</span>
    <button onclick="nextPage()">Next</button>
    <input type="number" id="jump-input" placeholder="Go to ID" style="width: 80px; padding: 4px;">
    <button onclick="jumpTo()">Go</button>
</div>

<div id="table-container">
    <table>
        <thead>
            <tr>
                <th class="col-id">ID</th>
                <th class="col-dir">Dir</th>
                <th class="col-time">Time (ms)</th>
                <th class="col-len">Len</th>
                <th class="col-data">Data (Hex)</th>
            </tr>
        </thead>
        <tbody id="packet-body">
            <!-- Rows injected here -->
        </tbody>
    </table>
</div>

<div id="overlay" onclick="closeModal()"></div>
<div id="detail-view">
    <div style="display:flex; justify-content:space-between;">
        <h3 style="margin:0">Packet Details</h3>
        <button onclick="closeModal()">Close</button>
    </div>
    <div id="detail-content"></div>
</div>

<script>
    let totalPackets = 0;
    let pageSize = 50;
    let currentPage = 0;
    let allData = [];

    async function loadData(start) {
        document.getElementById('status').innerText = 'Fetching...';
        try {
            const res = await fetch(`/api/packets?start=${start}&count=${pageSize}`);
            const json = await res.json();
            totalPackets = json.total;
            allData = json.packets;
            renderTable(json.packets);
            updateControls();
            document.getElementById('status').innerText = `Total Packets: ${totalPackets}`;
        } catch (e) {
            console.error(e);
            document.getElementById('status').innerText = 'Error loading data';
        }
    }

    function renderTable(packets) {
        const tbody = document.getElementById('packet-body');
        tbody.innerHTML = '';
        packets.forEach(p => {
            const tr = document.createElement('tr');
            tr.innerHTML = `
                <td class="col-id">${p.id}</td>
                <td class="col-dir ${p.dir === 'C2S' ? 'dir-c2s' : 'dir-s2c'}">${p.dir}</td>
                <td class="col-time">${p.time_ms}</td>
                <td class="col-len">${p.len}</td>
                <td class="col-data">${p.hex}</td>
            `;
            tr.onclick = () => showDetails(p);
            tbody.appendChild(tr);
        });
    }

    function updateControls() {
        document.getElementById('page-indicator').innerText = `Page ${currentPage + 1} / ${Math.ceil(totalPackets / pageSize)}`;
    }

    function nextPage() {
        if ((currentPage + 1) * pageSize < totalPackets) {
            currentPage++;
            loadData(currentPage * pageSize);
        }
    }

    function prevPage() {
        if (currentPage > 0) {
            currentPage--;
            loadData(currentPage * pageSize);
        }
    }

    function jumpTo() {
        const id = parseInt(document.getElementById('jump-input').value);
        if (!isNaN(id) && id >= 0 && id < totalPackets) {
            currentPage = Math.floor(id / pageSize);
            loadData(currentPage * pageSize);
        }
    }

    function showDetails(packet) {
        document.getElementById('detail-content').innerText = packet.full_hex;
        document.getElementById('detail-view').style.display = 'flex';
        document.getElementById('overlay').style.display = 'block';
    }

    function closeModal() {
        document.getElementById('detail-view').style.display = 'none';
        document.getElementById('overlay').style.display = 'none';
    }

    // Initial load
    loadData(0);
</script>

</body>
</html>
"""

def main():
    if not os.path.exists(FILE_PATH):
        print(f"Error: File '{FILE_PATH}' not found.")
        print("Please edit the FILE_PATH variable at the top of the script.")
        return

    global indexer
    indexer = PacketIndexer(FILE_PATH)

    # Start Server
    with socketserver.TCPServer(("", PORT), RequestHandler) as httpd:
        url = f"http://localhost:{PORT}"
        print(f"\n[+] Server started at {url}")
        print("[+] Press Ctrl+C to stop.")
        webbrowser.open(url)
        try:
            httpd.serve_forever()
        except KeyboardInterrupt:
            print("\n[+] Stopping server...")
            httpd.server_close()

if __name__ == "__main__":
    main()
