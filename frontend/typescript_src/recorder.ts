const API_URL = "/api";
const MIN_FREQ = 0;
const MAX_FREQ = 30_000_000;
const MAX_ZOOM = 14;
const REFRESH_INTERVAL_MS = 5000;
const LOG_REFRESH_INTERVAL_MS = 1000;

// --- DOM Elements ---
const apiStatusEl = document.getElementById('api-status') as HTMLSpanElement;
const createJobForm = document.getElementById('create-job-form') as HTMLFormElement;
const createJobBtn = document.getElementById('create-job-btn') as HTMLButtonElement;
const jobsTableBody = document.getElementById('jobs-table-body') as HTMLTableSectionElement;
const freqRangeEl = document.getElementById('freq-range') as HTMLDivElement;
const bandwidthEl = document.getElementById('bandwidth') as HTMLDivElement;
const warningEl = document.getElementById('warning') as HTMLDivElement;

// Form inputs
const recTypeInput = document.getElementById('rec_type') as HTMLSelectElement;
const frequencyInput = document.getElementById('frequency') as HTMLInputElement;
const zoomInput = document.getElementById('zoom') as HTMLInputElement;
const durationInput = document.getElementById('duration') as HTMLInputElement;
const intervalInput = document.getElementById('interval') as HTMLInputElement;

// Log Viewer
const logModal = document.getElementById('log-modal') as HTMLDivElement;
const logModalClose = document.getElementById('log-modal-close') as HTMLButtonElement;
const logTableBody = document.getElementById('log-table-body') as HTMLTableSectionElement;
const logModalTitle = document.getElementById('log-modal-title') as HTMLHeadingElement;

type RecordingType = 'png' | 'iq';

interface Log {
    timestamp: number;
    data: string;
}

type Logs = Log[];

interface RecorderSettings {
    rec_type: RecordingType;
    frequency: number;
    zoom?: number;
    duration: number;
    interval?: number | null;
}

interface Job {
    job_id: number;
    job_uid: string;
    running: boolean;
    started_at: number | null;
    next_run_start: number | null;
    logs: Logs;
    settings: RecorderSettings;
}

type JobList = Job[];

let logRefreshInterval: number | null = null;
let is_recording = false, start_error = false
let currentLogJobId: number | null = null;

function updateBandwidthInfo() {
    const { bandwidth, selection_freq_min, selection_freq_max, zoom_invalid, error_messages } = calcFreqRange(Number(frequencyInput.value) * 1000, Number(zoomInput.value), recTypeInput.value)
    
    if (error_messages.length > 0) {
        warningEl.innerHTML = error_messages.join('<br>');
        start_error = true
        createJobBtn.disabled = is_recording || start_error;
    } else {
        warningEl.innerHTML = '';
        start_error = false
        createJobBtn.disabled = is_recording || start_error;
    }
    if (!zoom_invalid) {
        bandwidthEl.textContent = "Bandwidth: " + format_freq(bandwidth);
        freqRangeEl.textContent = "Range: " + format_freq(selection_freq_min) + ' - ' + format_freq(selection_freq_max);
    }
    else {
        freqRangeEl.textContent = 'Range: ---- Hz - ---- Hz';
        bandwidthEl.textContent = 'Bandwidth: ---- Hz';
    }
}

function isNrValid(nr: number, nr_name: string) {
    let nr_valid = true, nr_error_messages = [];
    if (isNaN(nr)) {
        nr_error_messages.push(nr_name + " is not a number.");
        nr_valid = false;
    } 
    return { nr_valid: nr_valid, nr_error_messages: nr_error_messages };
}

function escapeHtml(unsafe: string) {
    return unsafe.replace(/&/g, "&amp;")
                 .replace(/</g, "&lt;")
                 .replace(/>/g, "&gt;")
                 .replace(/"/g, "&quot;")
                 .replace(/'/g, "&#039;");
}

function isZoomValid(zoom: number) {
    let zoom_valid = true, zoom_error_messages = [];
    const { nr_valid, nr_error_messages } = isNrValid(zoom, "Zoom")
    zoom_error_messages.push(...nr_error_messages)
    if (!nr_valid) {
        zoom_valid = false;
    } 
    else {
        if (zoom < 0) {
            zoom_error_messages.push(`Zoom is too low: ${zoom}. Minimum is 0.`);
            zoom_valid = false;
        }
        else if (zoom > MAX_ZOOM) {
            zoom_error_messages.push(`Zoom is too high: ${zoom}. Maximum is ${MAX_ZOOM}.`);
            zoom_valid = false;
        }
    }
    return {zoom_valid: zoom_valid, zoom_error_messages: zoom_error_messages};
}

function calcFreqRange(center_freq_hz: number, zoom: number, mode: string) {
    let bandwidth = 0, selection_freq_min = 0, selection_freq_max = 0, freq_range_invalid = false, error_messages = [];

    const { nr_valid, nr_error_messages } = isNrValid(center_freq_hz, "Frequency")
    error_messages.push(...nr_error_messages)
    if (!nr_valid) {
        return { bandwidth: 0, selection_freq_min: 0, selection_freq_max: 0, freq_range_invalid: null, zoom_invalid: false, error_messages: error_messages };
    }

    if (mode == "png") {
        const {zoom_valid, zoom_error_messages} = isZoomValid(zoom);
        error_messages.push(...zoom_error_messages);
        if (!zoom_valid) {
            return { bandwidth: 0, selection_freq_min: 0, selection_freq_max: 0, freq_range_invalid: null, zoom_invalid: true, error_messages: error_messages };
        }

        bandwidth = (MAX_FREQ - MIN_FREQ) / Math.pow(2, zoom);
    }
    else if (mode == "iq") {
        bandwidth = 12_000
    }
    else {
        error_messages.push(`Invalid type: ${mode}`)
        return { bandwidth: 0, selection_freq_min: 0, selection_freq_max: 0, freq_range_invalid: null, zoom_invalid: false, error_messages: error_messages };
    }

    selection_freq_max = center_freq_hz + bandwidth / 2;
    selection_freq_min = center_freq_hz - bandwidth / 2;

    if (selection_freq_max > MAX_FREQ) {
        error_messages.push("Frequency range exceeds MAX_FREQ " + format_freq(MAX_FREQ)+ ". Selected max = " + format_freq(selection_freq_max));
        freq_range_invalid = true;
    }
    if (selection_freq_min < MIN_FREQ) {
        error_messages.push("Frequency range below MIN_FREQ " + format_freq(MIN_FREQ) + ". Selected min = " + format_freq(selection_freq_min));
        freq_range_invalid = true;
    }

    return { bandwidth: bandwidth, selection_freq_min: selection_freq_min, selection_freq_max: selection_freq_max, freq_range_invalid: freq_range_invalid, zoom_invalid: false, error_messages: error_messages };
}

function format_freq(freq_hz: number) {
    if (Math.abs(freq_hz) < 1000) {
        let freq_hz_str = freq_hz.toFixed(0)
        return `${freq_hz_str} Hz`
    }
    else if (Math.abs(freq_hz) >= 1000 && Math.abs(freq_hz) < 1_000_000) {
        let freq_khz = (freq_hz / 1000).toFixed(1)
        return `${freq_khz} kHz`
    }
    else {
        let freq_mhz = (freq_hz / 1_000_000).toFixed(1)
        return `${freq_mhz} MHz`
    }
}

function formatTime(unixTime: number | null) {
    if (unixTime == null) {
        return "None"
    }
    const date = new Date((unixTime * 1000));
    return date.toLocaleString(undefined, { hour12: false })
}

async function getAllJobStatus() {
    try {
        const response = await fetch(`${API_URL}/recorder/status`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const joblist: JobList = await response.json();
        renderJobList(joblist);
    }
    catch (err) {
        console.error("Failed to fetch recorder status:", err);
        checkApiStatus()
    }
}

async function fetchAndRenderLogs(jobId: number) {
    try {
        const response = await fetch(`${API_URL}/recorder/status/${jobId}`);
        if (!response.ok) {
            throw new Error(`HTTP error! status: ${response.status}`);
        }
        const status: Job = await response.json();
        const logs: Logs = status.logs;

        if (currentLogJobId !== jobId) {
            currentLogJobId = jobId;
        }


        logModalTitle.textContent = `Logs for Job ${jobId}`;
        logTableBody.innerHTML = ''; 

        if (logs.length === 0) {
            logTableBody.innerHTML = `<tr><td colspan="2" style="text-align:center;">No logs available for this job.</td></tr>`;
        } else {
            logs.forEach(log => {
                const tr = document.createElement('tr');
                const date = formatTime(log.timestamp);
                
                tr.innerHTML = `
                    <td style="white-space: nowrap;">${date}</td>
                    <td>${escapeHtml(log.data)}</td>
                `;
                logTableBody.appendChild(tr); 
            });
        }
    } catch (err) {
        console.error(`Failed to fetch logs for job ${jobId}:`, err);
    }
}

async function renderJobList(jobs: JobList) {
    jobsTableBody.innerHTML = '';

    if (jobs.length == 0) {
        jobsTableBody.innerHTML = `<tr><td colspan="10" style="text-align:center;">No active jobs found.</td></tr>`;
        return;
    }

    for (const job of jobs) {
        const tr = document.createElement('tr');
        tr.setAttribute('data-job-id', `${job.job_id}`);
        
        const statusText: string = job.running ? 'Recording' : 'Stoped';
        const statusColor: string = job.running ? 'var(--green)' : 'var(--accent-color)';
        
        let settingsHTML = `Type: ${job.settings.rec_type}<br>`;
        settingsHTML += `Freq: ${format_freq(job.settings.frequency)}<br>`;
        settingsHTML += `Duration: ${job.settings.duration}s`;

        if (job.settings.rec_type === 'png') {
            settingsHTML += `<br>Zoom: ${job.settings.zoom}`;
        }
        if (job.settings.interval) {
            settingsHTML += `<br>Interval: ${job.settings.interval}s`;
        }
        
        tr.innerHTML = `
            <td>${job.job_uid}</td>
            <td style="color: ${statusColor}; font-weight: bold;">${statusText}</td>
            <td style="white-space: nowrap;">${settingsHTML}</td>
            <td>${formatTime(job.started_at)}</td>
            <td>${formatTime(job.next_run_start)}</td>
            <td>
                <div class="button-group">
                    <button class="btn-stop" data-job-id="${job.job_id}" ${!job.running ? 'disabled' : ''}>Stop</button>
                    <button class="btn-logs" data-job-id="${job.job_id}">Logs</button>
                    <button class="btn-remove" data-job-id="${job.job_id}">Remove</button>
                </div>
            </td>
        `;
        jobsTableBody.appendChild(tr);
    }
}

async function handleCreateJob(event: SubmitEvent) {
    event.preventDefault();
    const rec_type = recTypeInput.value as RecordingType;
    const frequency = Math.round(parseFloat(frequencyInput.value) * 1000);
    const zoom = rec_type === 'png' ? parseInt(zoomInput.value, 10) : undefined;
    const duration = parseInt(durationInput.value, 10);
    const intervalVal = parseInt(intervalInput.value, 10);
    const interval = isNaN(intervalVal) ? null : intervalVal;

    const body = { rec_type, frequency, zoom, duration, interval };

    try {
        const response = await fetch(`${API_URL}/recorder/start`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify(body)
        });
        const data = await response.json();
        console.log(data);
        await getAllJobStatus();
    } catch (err) {
        warningEl.innerHTML = `Failed to start recorder. Error: ${err}`;
        checkApiStatus()
    }
}

async function removeJob(jobId: number) {
    try {
        const response = await fetch(`${API_URL}/recorder/${jobId}`, {
            method: 'DELETE',
        });
        if (!response.ok) {
            throw new Error(`Failed to remove job: ${response.statusText}`);
        }
        // Assuming success, refresh the job list
        await getAllJobStatus();
        console.log(`Job ${jobId} removed successfully.`);
    } catch (err) {
        console.error(`Error removing job ${jobId}:`, err);
        warningEl.innerHTML = `Failed to remove job ${jobId}. Error: ${err}`;
        checkApiStatus()
    }
}

function handleJobActions(event: Event) {
    const target = event.target as HTMLElement;
    const button = target.closest('button');

    if (button) {
        const jobIdAttr = button.getAttribute('data-job-id');
        if (jobIdAttr) {
            const jobId = parseInt(jobIdAttr, 10);
            
            // Check if the clicked button is the 'Remove' button
            if (button.classList.contains('btn-remove')) {
                if (confirm(`Are you sure you want to remove Job ID ${jobId}?`)) {
                    removeJob(jobId); 
                }
            }
            else if (button.classList.contains('btn-logs')) {
                showJobLogs(jobId);
            }
        }
    }
}

async function showJobLogs(jobId: number) {
    // 1. Stop any existing interval
    if (logRefreshInterval !== null) {
        clearInterval(logRefreshInterval);
    }

    currentLogJobId = jobId;

    await fetchAndRenderLogs(jobId);
    
    logRefreshInterval = setInterval(() => {
        if (currentLogJobId !== null) {
            fetchAndRenderLogs(currentLogJobId);
        }
    }, LOG_REFRESH_INTERVAL_MS) as unknown as number;

    logModal.style.display = 'block';
}

async function checkApiStatus() {
    try {
        const response = await fetch(`${API_URL}/`);
        if (!response.ok) throw new Error(`HTTP error! status: ${response.status}`);
        
        const text = await response.text();
        apiStatusEl.textContent = `API Status: ${text}`;
        apiStatusEl.className = 'online';
    } catch (error) {
        console.error('API status check failed:', error);
        apiStatusEl.textContent = 'API Status: OFFLINE';
        apiStatusEl.className = 'offline';
    }
}

document.addEventListener('DOMContentLoaded', () => {
    checkApiStatus();
    getAllJobStatus();
    setInterval(getAllJobStatus, REFRESH_INTERVAL_MS);

    // Update bandwidth info when freq or zoom changes
    frequencyInput.addEventListener('input', updateBandwidthInfo);
    zoomInput.addEventListener('change', updateBandwidthInfo);
    recTypeInput.addEventListener('change', updateBandwidthInfo); // Added listener for rec_type change
    createJobForm.addEventListener('submit', handleCreateJob)

    jobsTableBody.addEventListener('click', handleJobActions);

    if (logModalClose) {
        logModalClose.addEventListener('click', () => {
            logModal.style.display = 'none';
            // STOP the refresh interval when closing
            if (logRefreshInterval !== null) {
                clearInterval(logRefreshInterval);
                logRefreshInterval = null;
            }
            currentLogJobId = null;
        });
    }
    if (logModal) {
        window.addEventListener('click', (event) => {
            if (event.target === logModal) {
                logModal.style.display = 'none';
                // STOP the refresh interval when closing
                if (logRefreshInterval !== null) {
                    clearInterval(logRefreshInterval);
                    logRefreshInterval = null;
                }
                currentLogJobId = null;
            }
        });
    }

    updateBandwidthInfo();
});
