/**
 * Toss Browser Extension - Popup UI Logic
 *
 * Handles user interactions in the popup and communicates
 * with the background service worker.
 */

// Get browser API
const browserAPI = typeof chrome !== 'undefined' ? chrome : browser;

// DOM Elements
let elements = {};

// Current state
const state = {
  connectionState: 'disconnected',
  identity: null,
  settings: null,
  pairedDevices: [],
  clipboardHistory: [],
};

/**
 * Initialize the popup
 */
async function initialize() {
  // Cache DOM elements
  cacheElements();

  // Setup event listeners
  setupEventListeners();

  // Load initial state
  await loadState();

  // Setup message listener for real-time updates
  browserAPI.runtime.onMessage.addListener(handleBackgroundMessage);
}

/**
 * Cache frequently used DOM elements
 */
function cacheElements() {
  elements = {
    // Header
    connectionStatus: document.getElementById('connectionStatus'),
    settingsBtn: document.getElementById('settingsBtn'),

    // Quick Actions
    sendClipboardBtn: document.getElementById('sendClipboardBtn'),
    connectBtn: document.getElementById('connectBtn'),

    // Tabs
    tabs: document.querySelectorAll('.tab'),
    historyTab: document.getElementById('historyTab'),
    devicesTab: document.getElementById('devicesTab'),

    // History
    historyList: document.getElementById('historyList'),
    clearHistoryBtn: document.getElementById('clearHistoryBtn'),

    // Devices
    devicesList: document.getElementById('devicesList'),
    addDeviceBtn: document.getElementById('addDeviceBtn'),

    // Pairing Modal
    pairingModal: document.getElementById('pairingModal'),
    closePairingModal: document.getElementById('closePairingModal'),
    pairingTabs: document.querySelectorAll('.pairing-tab'),
    scanCodePanel: document.getElementById('scanCodePanel'),
    showCodePanel: document.getElementById('showCodePanel'),
    pairingCodeInput: document.getElementById('pairingCodeInput'),
    deviceNameInput: document.getElementById('deviceNameInput'),
    deviceName: document.getElementById('deviceName'),
    pairBtn: document.getElementById('pairBtn'),
    myPairingCode: document.getElementById('myPairingCode'),
    codeExpiry: document.getElementById('codeExpiry'),
  };
}

/**
 * Setup event listeners
 */
function setupEventListeners() {
  // Settings button
  elements.settingsBtn.addEventListener('click', () => {
    browserAPI.runtime.openOptionsPage();
  });

  // Send clipboard button
  elements.sendClipboardBtn.addEventListener('click', handleSendClipboard);

  // Connect button
  elements.connectBtn.addEventListener('click', handleConnect);

  // Tab switching
  elements.tabs.forEach((tab) => {
    tab.addEventListener('click', () => switchTab(tab.dataset.tab));
  });

  // Clear history button
  elements.clearHistoryBtn.addEventListener('click', handleClearHistory);

  // Add device button
  elements.addDeviceBtn.addEventListener('click', openPairingModal);

  // Pairing modal
  elements.closePairingModal.addEventListener('click', closePairingModal);
  elements.pairingModal.querySelector('.modal-backdrop').addEventListener('click', closePairingModal);

  // Pairing tabs
  elements.pairingTabs.forEach((tab) => {
    tab.addEventListener('click', () => switchPairingTab(tab.dataset.pairingTab));
  });

  // Pairing code input
  elements.pairingCodeInput.addEventListener('input', handlePairingCodeInput);

  // Pair button
  elements.pairBtn.addEventListener('click', handlePair);
}

/**
 * Load initial state from background
 */
async function loadState() {
  try {
    // Get current state
    const stateResponse = await sendMessage({ type: 'GET_STATE' });
    if (stateResponse) {
      state.connectionState = stateResponse.connectionState;
      state.identity = stateResponse.identity;
      state.settings = stateResponse.settings;
      state.pairedDevices = stateResponse.pairedDevices;
    }

    // Get clipboard history
    const historyResponse = await sendMessage({ type: 'GET_HISTORY' });
    if (historyResponse?.history) {
      state.clipboardHistory = historyResponse.history;
    }

    // Update UI
    updateConnectionStatus();
    updateHistoryList();
    updateDevicesList();
    updateQuickActions();
  } catch (error) {
    console.error('Failed to load state:', error);
  }
}

/**
 * Handle messages from background
 */
function handleBackgroundMessage(message) {
  switch (message.type) {
  case 'CONNECTION_STATE_CHANGED':
    state.connectionState = message.state;
    updateConnectionStatus();
    updateQuickActions();
    break;

  case 'CLIPBOARD_UPDATE':
    state.clipboardHistory.unshift(message.item);
    updateHistoryList();
    break;

  case 'CLIPBOARD_SENT':
    // Optionally show confirmation
    break;

  case 'ERROR':
    showError(message.message);
    break;
  }
}

/**
 * Update connection status display
 */
function updateConnectionStatus() {
  const statusEl = elements.connectionStatus;
  const textEl = statusEl.querySelector('.status-text');

  // Remove existing classes
  statusEl.classList.remove('connected', 'connecting', 'disconnected', 'error');

  switch (state.connectionState) {
  case 'connected':
    statusEl.classList.add('connected');
    textEl.textContent = 'Connected';
    elements.connectBtn.textContent = 'Disconnect';
    break;

  case 'connecting':
  case 'authenticating':
    statusEl.classList.add('connecting');
    textEl.textContent = 'Connecting...';
    elements.connectBtn.textContent = 'Cancel';
    break;

  case 'error':
    statusEl.classList.add('error');
    textEl.textContent = 'Error';
    elements.connectBtn.textContent = 'Retry';
    break;

  default:
    statusEl.classList.add('disconnected');
    textEl.textContent = 'Disconnected';
    elements.connectBtn.textContent = 'Connect';
  }
}

/**
 * Update quick action buttons
 */
function updateQuickActions() {
  const isConnected = state.connectionState === 'connected';
  const hasDevices = state.pairedDevices.length > 0;

  elements.sendClipboardBtn.disabled = !isConnected || !hasDevices;
}

/**
 * Update clipboard history list
 */
function updateHistoryList() {
  const list = elements.historyList;

  if (state.clipboardHistory.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"></path>
          <rect x="8" y="2" width="8" height="4" rx="1" ry="1"></rect>
        </svg>
        <p>No clipboard history yet</p>
        <p class="hint">Copy something to get started</p>
      </div>
    `;
    elements.clearHistoryBtn.hidden = true;
    return;
  }

  elements.clearHistoryBtn.hidden = false;

  list.innerHTML = state.clipboardHistory
    .map((item) => createHistoryItemHTML(item))
    .join('');

  // Add click handlers
  list.querySelectorAll('.history-item').forEach((el) => {
    const itemId = el.dataset.id;

    el.addEventListener('click', () => handleHistoryItemClick(itemId));

    el.querySelector('.copy-btn')?.addEventListener('click', (e) => {
      e.stopPropagation();
      handleCopyHistoryItem(itemId);
    });

    el.querySelector('.send-btn')?.addEventListener('click', (e) => {
      e.stopPropagation();
      handleSendHistoryItem(itemId);
    });

    el.querySelector('.delete-btn')?.addEventListener('click', (e) => {
      e.stopPropagation();
      handleDeleteHistoryItem(itemId);
    });
  });
}

/**
 * Create HTML for a history item
 */
function createHistoryItemHTML(item) {
  const icon = getContentTypeIcon(item.type);
  const timeAgo = formatTimeAgo(item.timestamp);
  const preview = escapeHtml(item.preview || item.data?.substring(0, 100) || '');
  const source = item.synced
    ? item.fromDevice
      ? `From ${item.fromDevice.substring(0, 8)}...`
      : 'Sent'
    : 'Local';

  return `
    <div class="history-item" data-id="${item.id}">
      <div class="history-item-icon">${icon}</div>
      <div class="history-item-content">
        <div class="history-item-preview">${preview}</div>
        <div class="history-item-meta">
          <span>${timeAgo}</span>
          <span>${source}</span>
        </div>
      </div>
      <div class="history-item-actions">
        <button class="icon-btn copy-btn" title="Copy">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect>
            <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path>
          </svg>
        </button>
        <button class="icon-btn send-btn" title="Send">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <line x1="22" y1="2" x2="11" y2="13"></line>
            <polygon points="22 2 15 22 11 13 2 9 22 2"></polygon>
          </svg>
        </button>
        <button class="icon-btn delete-btn" title="Delete">
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polyline points="3 6 5 6 21 6"></polyline>
            <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
          </svg>
        </button>
      </div>
    </div>
  `;
}

/**
 * Get icon for content type
 */
function getContentTypeIcon(type) {
  const icons = {
    text: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path><polyline points="14 2 14 8 20 8"></polyline><line x1="16" y1="13" x2="8" y2="13"></line><line x1="16" y1="17" x2="8" y2="17"></line><polyline points="10 9 9 9 8 9"></polyline></svg>',
    url: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M10 13a5 5 0 0 0 7.54.54l3-3a5 5 0 0 0-7.07-7.07l-1.72 1.71"></path><path d="M14 11a5 5 0 0 0-7.54-.54l-3 3a5 5 0 0 0 7.07 7.07l1.71-1.71"></path></svg>',
    image: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"></rect><circle cx="8.5" cy="8.5" r="1.5"></circle><polyline points="21 15 16 10 5 21"></polyline></svg>',
    file: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M13 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"></path><polyline points="13 2 13 9 20 9"></polyline></svg>',
  };
  return icons[type] || icons.text;
}

/**
 * Update devices list
 */
function updateDevicesList() {
  const list = elements.devicesList;

  if (state.pairedDevices.length === 0) {
    list.innerHTML = `
      <div class="empty-state">
        <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
          <rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect>
          <line x1="8" y1="21" x2="16" y2="21"></line>
          <line x1="12" y1="17" x2="12" y2="21"></line>
        </svg>
        <p>No paired devices</p>
        <p class="hint">Add a device to start syncing</p>
      </div>
    `;
    return;
  }

  list.innerHTML = state.pairedDevices
    .map((device) => createDeviceItemHTML(device))
    .join('');

  // Add click handlers for device items
  list.querySelectorAll('.device-item').forEach((el) => {
    const deviceId = el.dataset.id;

    el.querySelector('.remove-device-btn')?.addEventListener('click', (e) => {
      e.stopPropagation();
      handleRemoveDevice(deviceId);
    });
  });
}

/**
 * Create HTML for a device item
 */
function createDeviceItemHTML(device) {
  const icon = getDeviceIcon(device.platform);
  const status = device.isOnline ? 'online' : 'offline';
  const statusText = device.isOnline ? 'Online' : 'Offline';

  return `
    <div class="device-item" data-id="${device.deviceId}">
      <div class="device-icon">${icon}</div>
      <div class="device-info">
        <div class="device-name">${escapeHtml(device.name || 'Unknown Device')}</div>
        <div class="device-status ${status}">${statusText}</div>
      </div>
      <button class="icon-btn remove-device-btn" title="Remove">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <line x1="18" y1="6" x2="6" y2="18"></line>
          <line x1="6" y1="6" x2="18" y2="18"></line>
        </svg>
      </button>
    </div>
  `;
}

/**
 * Get icon for device platform
 */
function getDeviceIcon(platform) {
  const icons = {
    macos: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect><line x1="8" y1="21" x2="16" y2="21"></line><line x1="12" y1="17" x2="12" y2="21"></line></svg>',
    windows: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect><line x1="8" y1="21" x2="16" y2="21"></line><line x1="12" y1="17" x2="12" y2="21"></line></svg>',
    linux: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="2" y="3" width="20" height="14" rx="2" ry="2"></rect><line x1="8" y1="21" x2="16" y2="21"></line><line x1="12" y1="17" x2="12" y2="21"></line></svg>',
    ios: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="2" width="14" height="20" rx="2" ry="2"></rect><line x1="12" y1="18" x2="12.01" y2="18"></line></svg>',
    android: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="5" y="2" width="14" height="20" rx="2" ry="2"></rect><line x1="12" y1="18" x2="12.01" y2="18"></line></svg>',
    browser: '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"></circle><line x1="2" y1="12" x2="22" y2="12"></line><path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"></path></svg>',
  };
  return icons[platform?.toLowerCase()] || icons.browser;
}

/**
 * Switch between tabs
 */
function switchTab(tabName) {
  elements.tabs.forEach((tab) => {
    tab.classList.toggle('active', tab.dataset.tab === tabName);
  });

  elements.historyTab.classList.toggle('active', tabName === 'history');
  elements.devicesTab.classList.toggle('active', tabName === 'devices');
}

/**
 * Handle connect button click
 */
async function handleConnect() {
  if (state.connectionState === 'connected') {
    await sendMessage({ type: 'DISCONNECT' });
  } else {
    await sendMessage({ type: 'CONNECT' });
  }
}

/**
 * Handle send clipboard button click
 */
async function handleSendClipboard() {
  try {
    // Read from system clipboard
    const text = await navigator.clipboard.readText();
    if (!text) {
      showError('Clipboard is empty');
      return;
    }

    // Detect content type
    const type = /^https?:\/\//i.test(text) ? 'url' : 'text';

    // Get target devices
    const deviceIds = state.pairedDevices.map((d) => d.deviceId);

    await sendMessage({
      type: 'SEND_CLIPBOARD',
      data: { content: text, type, deviceIds },
    });
  } catch (error) {
    showError('Failed to read clipboard: ' + error.message);
  }
}

/**
 * Handle history item click
 */
function handleHistoryItemClick(itemId) {
  const item = state.clipboardHistory.find((i) => i.id === itemId);
  if (item) {
    handleCopyHistoryItem(itemId);
  }
}

/**
 * Copy history item to clipboard
 */
async function handleCopyHistoryItem(itemId) {
  const item = state.clipboardHistory.find((i) => i.id === itemId);
  if (item) {
    try {
      await navigator.clipboard.writeText(item.data || item.preview);
    } catch {
      await sendMessage({ type: 'COPY_TO_CLIPBOARD', data: { content: item.data || item.preview } });
    }
  }
}

/**
 * Send history item to devices
 */
async function handleSendHistoryItem(itemId) {
  const item = state.clipboardHistory.find((i) => i.id === itemId);
  if (item) {
    const deviceIds = state.pairedDevices.map((d) => d.deviceId);
    await sendMessage({
      type: 'SEND_CLIPBOARD',
      data: { content: item.data || item.preview, type: item.type, deviceIds },
    });
  }
}

/**
 * Delete history item
 */
async function handleDeleteHistoryItem(itemId) {
  await sendMessage({ type: 'DELETE_HISTORY_ITEM', data: { itemId } });
  state.clipboardHistory = state.clipboardHistory.filter((i) => i.id !== itemId);
  updateHistoryList();
}

/**
 * Clear all history
 */
async function handleClearHistory() {
  await sendMessage({ type: 'CLEAR_HISTORY' });
  state.clipboardHistory = [];
  updateHistoryList();
}

/**
 * Handle remove device
 */
async function handleRemoveDevice(deviceId) {
  await sendMessage({ type: 'REMOVE_PAIRED_DEVICE', data: { deviceId } });
  state.pairedDevices = state.pairedDevices.filter((d) => d.deviceId !== deviceId);
  updateDevicesList();
  updateQuickActions();
}

/**
 * Open pairing modal
 */
function openPairingModal() {
  elements.pairingModal.classList.remove('hidden');
  elements.pairingCodeInput.value = '';
  elements.pairingCodeInput.focus();
  elements.pairBtn.disabled = true;

  // Generate and show pairing code
  generatePairingCode();
}

/**
 * Close pairing modal
 */
function closePairingModal() {
  elements.pairingModal.classList.add('hidden');
}

/**
 * Switch pairing tab
 */
function switchPairingTab(tabName) {
  elements.pairingTabs.forEach((tab) => {
    tab.classList.toggle('active', tab.dataset.pairingTab === tabName);
  });

  elements.scanCodePanel.classList.toggle('active', tabName === 'scan');
  elements.showCodePanel.classList.toggle('active', tabName === 'show');

  if (tabName === 'show') {
    generatePairingCode();
  }
}

/**
 * Generate pairing code
 */
async function generatePairingCode() {
  const response = await sendMessage({ type: 'GET_PAIRING_CODE' });
  if (response?.code) {
    elements.myPairingCode.textContent = response.code;
    startCodeExpiryTimer();
  }
}

/**
 * Start code expiry timer
 */
function startCodeExpiryTimer() {
  let seconds = 300; // 5 minutes
  const updateTimer = () => {
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    elements.codeExpiry.textContent = `${mins}:${secs.toString().padStart(2, '0')}`;
    seconds--;
    if (seconds >= 0 && !elements.pairingModal.classList.contains('hidden')) {
      setTimeout(updateTimer, 1000);
    } else if (seconds < 0) {
      generatePairingCode();
    }
  };
  updateTimer();
}

/**
 * Handle pairing code input
 */
function handlePairingCodeInput(e) {
  const value = e.target.value.replace(/\D/g, '').substring(0, 6);
  e.target.value = value;
  elements.pairBtn.disabled = value.length !== 6;

  if (value.length === 6) {
    elements.deviceNameInput.hidden = false;
  }
}

/**
 * Handle pair button click
 */
async function handlePair() {
  const code = elements.pairingCodeInput.value;
  const name = elements.deviceName.value || 'Unknown Device';

  elements.pairBtn.disabled = true;
  elements.pairBtn.textContent = 'Pairing...';

  try {
    const response = await sendMessage({
      type: 'PAIR_WITH_CODE',
      data: { code, deviceInfo: { name } },
    });

    if (response?.success) {
      closePairingModal();
      // Refresh devices list
      const devicesResponse = await sendMessage({ type: 'GET_PAIRED_DEVICES' });
      if (devicesResponse?.devices) {
        state.pairedDevices = devicesResponse.devices;
        updateDevicesList();
        updateQuickActions();
      }
    } else {
      showError(response?.error || 'Pairing failed');
    }
  } catch (error) {
    showError('Pairing failed: ' + error.message);
  } finally {
    elements.pairBtn.disabled = false;
    elements.pairBtn.textContent = 'Pair';
  }
}

/**
 * Send message to background
 */
function sendMessage(message) {
  return new Promise((resolve, reject) => {
    browserAPI.runtime.sendMessage(message, (response) => {
      if (browserAPI.runtime.lastError) {
        reject(new Error(browserAPI.runtime.lastError.message));
      } else {
        resolve(response);
      }
    });
  });
}

/**
 * Show error notification
 */
function showError(message) {
  console.error(message);
  // TODO: Show toast notification
}

/**
 * Format timestamp as relative time
 */
function formatTimeAgo(timestamp) {
  const seconds = Math.floor((Date.now() - timestamp) / 1000);

  if (seconds < 60) {
    return 'Just now';
  }
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}m ago`;
  }
  if (seconds < 86400) {
    return `${Math.floor(seconds / 3600)}h ago`;
  }
  return `${Math.floor(seconds / 86400)}d ago`;
}

/**
 * Escape HTML to prevent XSS
 */
function escapeHtml(text) {
  const div = document.createElement('div');
  div.textContent = text;
  return div.innerHTML;
}

// Initialize on DOM load
document.addEventListener('DOMContentLoaded', initialize);
