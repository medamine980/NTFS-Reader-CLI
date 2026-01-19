# NTFS Reader CLI

A command-line wrapper for the [ntfs-reader](https://crates.io/crates/ntfs-reader) Rust library, providing easy access to NTFS Master File Table (MFT) and USN Journal functionality from any programming language.

## Features

- **MFT Reading**: List all files on an NTFS volume instantly (in-memory scan)
- **Pattern Matching**: Filter files using glob patterns (`*.pdf`) or regex
- **ADS Detection**: Enumerate Alternate Data Streams (hidden NTFS streams) for file tagging
- **USN Journal Monitoring**: Track file system changes in real-time
- **Multiple Output Formats**: JSON, CSV, or Bincode (binary, 3-5x faster parsing than JSON)
- **Cross-language**: Call from Python, Node.js, Go, or any language that can execute shell commands

## Requirements

- **Windows OS** (NTFS is Windows-specific)
- **Administrator privileges** (required for direct volume access)
- **Rust** (for building from source)

## Installation

### Build from Source

```powershell
git clone https://github.com/medamine980/NTFS-Reader-CLI
cd ntfs-reader-cli
cargo build --release
```

The executable will be at `target/release/ntfs-reader-cli.exe`

## Usage

### List All Files from MFT

```powershell
# List all files on C: drive (JSON output)
ntfs-reader-cli list-files --volume C:

# List only directories
ntfs-reader-cli list-files --volume C: --directories-only

# Filter by path (substring match)
ntfs-reader-cli list-files --volume C: --filter "Program Files"

# Filter by glob pattern - all PDF files
ntfs-reader-cli list-files --volume C: --filter "*.pdf"

# Filter by glob pattern - all .txt files in Documents
ntfs-reader-cli list-files --volume C: --filter "*\\Documents\\*.txt"

# Filter by regex - files ending with .pdf, .doc, or .docx
ntfs-reader-cli list-files --volume C: --filter "\\.(pdf|docx?)$"

# Limit results
ntfs-reader-cli list-files --volume C: --limit 100

# Pretty JSON output
ntfs-reader-cli list-files --volume C: --output json-pretty

# CSV output
ntfs-reader-cli list-files --volume C: --output csv

# Bincode output (binary format, 3-5x faster to parse than JSON)
ntfs-reader-cli list-files --volume C: --output bincode > files.bin
```

### Monitor USN Journal

```powershell
# Monitor new file system changes (starts from current position)
ntfs-reader-cli journal --volume C:

# Read from the beginning of the journal
ntfs-reader-cli journal --volume C: --from-start

# Get last 10 events
ntfs-reader-cli journal --volume C: --max-events 10

# Get last 50 events as pretty JSON
ntfs-reader-cli journal --volume C: --max-events 50 --output json-pretty

# Continuous monitoring (outputs events as they happen)
ntfs-reader-cli journal --volume C: --continuous

# Monitor only file creation events (reason mask: 0x00000100)
ntfs-reader-cli journal --volume C: --reason-mask 256 --continuous

# Monitor continuously but stop after 20 events
ntfs-reader-cli journal --volume C: --continuous --max-events 20
```

### Get Specific File Info

```powershell
# Get info for MFT record number 5 (root directory)
ntfs-reader-cli file-info --volume C: --record 5
```

## Output Format

### MFT Files (JSON)

```json
[
  {
    "name": "example.txt",
    "path": "C:\\Users\\Documents\\example.txt",
    "is_directory": false,
    "size": 1024,
    "created": "2024-01-15T10:30:00Z",
    "modified": "2024-01-15T14:20:00Z",
    "accessed": "2024-01-15T14:20:00Z",
    "alternate_data_streams": [
      {
        "name": "tags",
        "size": 128
      }
    ]
  }
]
```

**Note:** Files with Alternate Data Streams (ADS) will include them in the `alternate_data_streams` array. This is perfect for implementing file tagging systems using NTFS ADS.

### Journal Events (JSON)

```json
[
  {
    "usn": 12345678,
    "timestamp_ms": 1705328400000,
    "file_id": "Normal(281474976710656)",
    "parent_id": "Normal(281474976710655)",
    "reason": 256,
    "reason_str": "USN_REASON_FILE_CREATE",
    "path": "C:\\Users\\Documents\\newfile.txt"
  }
]
```

## Common USN Reason Masks

| Reason | Hex | Decimal | Description |
|--------|-----|---------|-------------|
| FILE_CREATE | 0x00000100 | 256 | File created |
| FILE_DELETE | 0x00000200 | 512 | File deleted |
| DATA_OVERWRITE | 0x00000001 | 1 | File data overwritten |
| DATA_EXTEND | 0x00000002 | 2 | File data extended |
| RENAME_NEW_NAME | 0x00002000 | 8192 | File renamed (new name) |
| RENAME_OLD_NAME | 0x00001000 | 4096 | File renamed (old name) |

Use bitwise OR to combine multiple reasons: `256 | 512 = 768` (create or delete)

## Integration Examples

### Python

```python
import subprocess
import json

# List all files
result = subprocess.run(
    ['ntfs-reader-cli', 'list-files', '--volume', 'C:', '--output', 'json'],
    capture_output=True,
    text=True
)
files = json.loads(result.stdout)

for file in files[:10]:
    print(f"{file['path']} - {file['size']} bytes")
```

### Node.js

```javascript
const { execSync } = require('child_process');

// Monitor journal
const output = execSync('ntfs-reader-cli journal --volume C: --max-events 10', {
  encoding: 'utf-8'
});

const events = JSON.parse(output);
events.forEach(event => {
  console.log(`${event.reason_str}: ${event.path}`);
});
```

### PowerShell

```powershell
# List files and parse JSON
$files = ntfs-reader-cli list-files --volume C: --output json | ConvertFrom-Json

# Filter large files
$largeFiles = $files | Where-Object { $_.size -gt 1GB }
$largeFiles | Format-Table name, size, path
```

### Go

```go
package main

import (
    "encoding/json"
    "os/exec"
)

type FileRecord struct {
    Name        string  `json:"name"`
    Path        string  `json:"path"`
    IsDirectory bool    `json:"is_directory"`
    Size        uint64  `json:"size"`
}

func main() {
    cmd := exec.Command("ntfs-reader-cli", "list-files", "--volume", "C:", "--limit", "100")
    output, _ := cmd.Output()
    
    var files []FileRecord
    json.Unmarshal(output, &files)
    
    for _, file := range files {
        println(file.Path)
    }
}
```

## Performance

- **MFT Scan**: Typically scans entire C: drive (hundreds of thousands of files) in 3-10 seconds
- **Journal Reading**: Near real-time with minimal overhead
- **Memory**: Loads entire MFT into memory (typically 50-200 MB)
- **Parsing Speed**: 
  - **Bincode**: ~3-5x faster than JSON (binary format)
  - **JSON**: Standard, human-readable
  - **CSV**: Good for spreadsheet imports

### Output Format Comparison

| Format | Speed | Size | Use Case |
|--------|-------|------|----------|
| Bincode | ⚡ Fastest | Smallest | High-performance apps, frequent queries |
| JSON | 🐢 Slower | Larger | Debugging, cross-platform, human-readable |
| CSV | 🐢 Slower | Medium | Spreadsheets, data analysis |

## Limitations

- **Windows only**: NTFS is a Windows file system
- **Requires admin**: Direct volume access needs elevation
- **No file content**: Only reads metadata, not file contents
- **Locked files**: Some system files may be inaccessible even with admin rights

## Troubleshooting

### "Access Denied" Error
- Make sure to run as Administrator
- Right-click Command Prompt/PowerShell → "Run as Administrator"

### Volume Path Issues
The tool accepts multiple volume path formats:
- `C:` - Drive letter
- `C:\` - Drive with backslash  
- `\\.\C:` - Device path (for MFT)
- `\\?\C:` - Extended path (for Journal)

All formats are automatically normalized.

## License

Dual-licensed under MIT OR Apache-2.0 (same as ntfs-reader)

## Credits

Built on top of the excellent [ntfs-reader](https://github.com/kikijiki/ntfs-reader) library by kikijiki.
