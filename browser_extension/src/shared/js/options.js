/**
 * Toss Browser Extension - Options Page Logic
 */

// Get browser API
const browserAPI = typeof chrome !== 'undefined' ? chrome : browser;

// DOM Elements
let elements = {};
let settings = {};
let saveTimeout = null;

/**
 * Initialize the options page
 */
async function initialize() {
  cacheElements();
  setupEventListeners();
  await loadSettings();
}

/**
 * Cache DOM elements
 */
function cacheElements() {
  elements = {
    // Connection
    relayUrl: document.getElementById('relayUrl'),
    deviceId: document.getElementById('deviceId'),

    // Sync
    autoSync: document.getElementById('autoSync'),
    targetDevice: document.getElementById('targetDevice'),

    // Content Types
    syncText: document.getElementById('syncText'),
    syncImages: document.getElementById('syncImages'),
    syncUrls: document.getElementById('syncUrls'),

    // Notifications
    notifications: document.getElementById('notifications'),

    // History
    maxHistory: document.getElementById('maxHistory'),
    clearHistory: document.getElementById('clearHistory'),

    // Data Management
    exportData: document.getElementById('exportData'),
    importData: document.getElementById('importData'),
    importFile: document.getElementById('importFile'),
    resetExtension: document.getElementById('resetExtension'),

    // Status
    saveStatus: document.getElementById('saveStatus'),
  };
}

/**
 * Setup event listeners
 */
function setupEventListeners() {
  // Auto-save on change
  const autoSaveInputs = [
    'relayUrl', 'autoSync', 'targetDevice', 'syncText',
    'syncImages', 'syncUrls', 'notifications', 'maxHistory'
  ];

  autoSaveInputs.forEach(id => {
    const el = elements[id];
    if (el) {
      el.addEventListener('change', handleSettingChange);
      if (el.type === 'text' || el.type === 'number') {
        el.addEventListener('input', handleSettingChange);
      }
    }
  });

  // Action buttons
  elements.clearHistory.addEventListener('click', handleClearHistory);
  elements.exportData.addEventListener('click', handleExportData);
  elements.importData.addEventListener('click', () => elements.importFile.click());
  elements.importFile.addEventListener('change', handleImportData);
  elements.resetExtension.addEventListener('click', handleResetExtension);
}

/**
 * Load settings from storage
 */
async function loadSettings() {
  try {
    // Get state from background
    const state = await sendMessage({ type: 'GET_STATE' });

    if (state?.identity) {
      elements.deviceId.textContent = state.identity.deviceId.substring(0, 32) + '...';
    }

    // Get settings
    const response = await sendMessage({ type: 'GET_SETTINGS' });
    settings = response?.settings || {};

    // Populate form
    elements.relayUrl.value = settings.relayUrl || 'wss://localhost:8080/api/v1/ws';
    elements.autoSync.checked = settings.autoSync !== false;
    elements.syncText.checked = settings.syncTextEnabled !== false;
    elements.syncImages.checked = settings.syncImagesEnabled !== false;
    elements.syncUrls.checked = settings.syncUrlsEnabled !== false;
    elements.notifications.checked = settings.notificationsEnabled !== false;
    elements.maxHistory.value = settings.maxHistoryItems || 50;

    // Populate target device dropdown
    const devicesResponse = await sendMessage({ type: 'GET_PAIRED_DEVICES' });
    const devices = devicesResponse?.devices || [];

    elements.targetDevice.innerHTML = '<option value="">All devices</option>';
    devices.forEach(device => {
      const option = document.createElement('option');
      option.value = device.deviceId;
      option.textContent = device.name || `Device ${device.deviceId.substring(0, 8)}`;
      if (settings.targetDevice === device.deviceId) {
        option.selected = true;
      }
      elements.targetDevice.appendChild(option);
    });

  } catch (error) {
    console.error('Failed to load settings:', error);
    showSaveStatus('Failed to load settings', 'error');
  }
}

/**
 * Handle setting change
 */
function handleSettingChange() {
  // Debounce save
  clearTimeout(saveTimeout);
  saveTimeout = setTimeout(saveSettings, 500);
}

/**
 * Save settings to storage
 */
async function saveSettings() {
  try {
    const updatedSettings = {
      relayUrl: elements.relayUrl.value,
      autoSync: elements.autoSync.checked,
      targetDevice: elements.targetDevice.value || null,
      syncTextEnabled: elements.syncText.checked,
      syncImagesEnabled: elements.syncImages.checked,
      syncUrlsEnabled: elements.syncUrls.checked,
      notificationsEnabled: elements.notifications.checked,
      maxHistoryItems: parseInt(elements.maxHistory.value) || 50,
    };

    await sendMessage({
      type: 'UPDATE_SETTINGS',
      data: { settings: updatedSettings },
    });

    settings = updatedSettings;
    showSaveStatus('Settings saved', 'success');
  } catch (error) {
    console.error('Failed to save settings:', error);
    showSaveStatus('Failed to save settings', 'error');
  }
}

/**
 * Handle clear history
 */
async function handleClearHistory() {
  if (!confirm('Are you sure you want to clear all clipboard history?')) {
    return;
  }

  try {
    await sendMessage({ type: 'CLEAR_HISTORY' });
    showSaveStatus('History cleared', 'success');
  } catch (error) {
    console.error('Failed to clear history:', error);
    showSaveStatus('Failed to clear history', 'error');
  }
}

/**
 * Handle export data
 */
async function handleExportData() {
  try {
    // Get all data from storage
    const state = await sendMessage({ type: 'GET_STATE' });
    const historyResponse = await sendMessage({ type: 'GET_HISTORY' });
    const settingsResponse = await sendMessage({ type: 'GET_SETTINGS' });
    const devicesResponse = await sendMessage({ type: 'GET_PAIRED_DEVICES' });

    const exportData = {
      version: 1,
      exportedAt: new Date().toISOString(),
      identity: state?.identity,
      settings: settingsResponse?.settings,
      clipboardHistory: historyResponse?.history || [],
      pairedDevices: devicesResponse?.devices || [],
    };

    // Download as JSON file
    const blob = new Blob([JSON.stringify(exportData, null, 2)], { type: 'application/json' });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = `toss-backup-${new Date().toISOString().split('T')[0]}.json`;
    a.click();
    URL.revokeObjectURL(url);

    showSaveStatus('Data exported', 'success');
  } catch (error) {
    console.error('Failed to export data:', error);
    showSaveStatus('Failed to export data', 'error');
  }
}

/**
 * Handle import data
 */
async function handleImportData(event) {
  const file = event.target.files?.[0];
  if (!file) return;

  try {
    const text = await file.text();
    const data = JSON.parse(text);

    if (data.version !== 1) {
      throw new Error('Unsupported backup version');
    }

    if (!confirm('This will replace your current settings and paired devices. Continue?')) {
      return;
    }

    // Import settings
    if (data.settings) {
      await sendMessage({
        type: 'UPDATE_SETTINGS',
        data: { settings: data.settings },
      });
    }

    // Import devices
    if (data.pairedDevices) {
      for (const device of data.pairedDevices) {
        await sendMessage({
          type: 'ADD_PAIRED_DEVICE',
          data: { device },
        });
      }
    }

    // Reload settings
    await loadSettings();
    showSaveStatus('Data imported', 'success');
  } catch (error) {
    console.error('Failed to import data:', error);
    showSaveStatus('Failed to import data: ' + error.message, 'error');
  }

  // Reset file input
  event.target.value = '';
}

/**
 * Handle reset extension
 */
async function handleResetExtension() {
  if (!confirm('This will delete all data including your device identity and paired devices. This cannot be undone. Continue?')) {
    return;
  }

  if (!confirm('Are you really sure? You will need to re-pair all your devices.')) {
    return;
  }

  try {
    // Clear all storage
    if (browserAPI.storage?.local) {
      await browserAPI.storage.local.clear();
    }

    showSaveStatus('Extension reset. Reloading...', 'success');

    // Reload extension
    setTimeout(() => {
      browserAPI.runtime.reload();
    }, 1500);
  } catch (error) {
    console.error('Failed to reset extension:', error);
    showSaveStatus('Failed to reset extension', 'error');
  }
}

/**
 * Show save status message
 */
function showSaveStatus(message, type = '') {
  elements.saveStatus.textContent = message;
  elements.saveStatus.className = 'save-status visible ' + type;

  setTimeout(() => {
    elements.saveStatus.classList.remove('visible');
  }, 3000);
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

// Initialize on DOM load
document.addEventListener('DOMContentLoaded', initialize);
