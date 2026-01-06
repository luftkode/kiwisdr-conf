const DOWNLOAD_URL = '/recorder/download/';

// List of file extensions that can be viewed in the browser
const viewableExtensions = ['txt', 'png'];

interface FileEntry {
    name: string;
    type: string;
    mtime: string;
    size: number;
}

function formatFileSize(bytes: number): string {
    if (bytes < 0) {
        return "Negative"
    }
    else if (bytes === 0) {
        return "0 B"
    }
    else if (bytes < 1_000) {
        return bytes.toFixed(0) + " B"
    }
    else if (bytes < 1_000_000) {
        return (bytes / 1_000).toFixed(1) + " kB"
    }
    else if (bytes < 1_000_000_000) {
        return (bytes / 1_000_000).toFixed(1) + " MB"
    }
    else {
        return (bytes / 1_000_000_000).toFixed(1) + " GB"
    }
}

async function fetchFileList(): Promise<FileEntry[]> {
    const response = await fetch(DOWNLOAD_URL);
    if (!response.ok) throw new Error('Failed to fetch files');
    const data: FileEntry[] = await response.json();
    return data;
}

async function updateFileList() {
    try {
        let files: FileEntry[] = await fetchFileList();
        if (!files || files.length === 0) {
            (document.getElementById('file-table')! as HTMLTableElement).innerHTML = "<tr><td colspan='4'>No files found.</td></tr>";
            return;
        }

        // Sort by date descending (newest first)
        files.sort((a, b) => new Date(b.mtime).getTime() - new Date(a.mtime).getTime());

        // Populate the table
        const tbody = document.querySelector('#file-table tbody')!;
        files.forEach(file => {
            if (!(file.type === "file")) return;
            const tr = document.createElement('tr'); 
            const formattedDate = new Date(file.mtime).toLocaleString(undefined, { hour12: false });
            const fileName = String(file.name);
            // Check file extension for viewable types
            const ext = file.name.split('.').pop()?.toLowerCase() ?? '';
            const viewButton = viewableExtensions.includes(ext)
                ? `<a href="${DOWNLOAD_URL + encodeURIComponent(fileName)}" target="_blank"><button>View</button></a>`
                : '';

            tr.innerHTML = `
                <td>${fileName}</td>
                <td>${formattedDate}</td>
                <td>${formatFileSize(file.size)}</td>
                <td>
                    <div class="button-group">
                    ${viewButton}
                    <a href="${DOWNLOAD_URL + encodeURIComponent(fileName)}" download><button>Download</button></a>
                    </div>
                </td>
            `;

            tbody.appendChild(tr);
        });
    } 
    catch (err) {
        console.error('Error loading file list:', err);
        const tbody = document.querySelector('#file-table tbody');
        const message = err instanceof Error ? err.message : String(err);
        if (tbody) {
            tbody.innerHTML = `<tr><td colspan="4">Error: ${message}</td></tr>`;
        }
    }
}

updateFileList();