/**
 * Toss Browser Extension - Background Service Worker (Chrome MV3)
 *
 * Handles WebSocket connection, context menus, clipboard operations,
 * and message passing between popup and content scripts.
 */

import { generateDeviceIdentity } from '../shared/js/crypto.js';
import { RelayClient, ConnectionState } from '../shared/js/relay-client.js';
import { storage, DefaultSettings } from '../shared/js/storage.js';

// Global state
let relayClient = null;
const currentState = {
  connectionState: ConnectionState.DISCONNECTED,
  identity: null,
  settings: null,
  pairedDevices: [],
};

// Context menu IDs
const CONTEXT_MENU_SEND_TO_DEVICE = 'toss_send_to_device';
const CONTEXT_MENU_SEND_TO_ALL = 'toss_send_to_all';

/**
 * Initialize the extension
 */
async function initialize() {
  console.log('Toss extension initializing...');

  // Load or generate identity
  let identity = await storage.getIdentity();
  if (!identity) {
    console.log('Generating new device identity...');
    identity = await generateDeviceIdentity();
    identity.deviceName = 'Browser Extension';
    identity.platform = 'browser';
    await storage.saveIdentity(identity);
  }
  currentState.identity = identity;

  // Load settings
  currentState.settings = await storage.getSettings();

  // Load paired devices
  currentState.pairedDevices = await storage.getPairedDevices();

  // Initialize relay client
  await initializeRelayClient();

  // Setup context menus
  await setupContextMenus();

  // Listen for messages from popup/options
  chrome.runtime.onMessage.addListener(handleMessage);

  console.log('Toss extension initialized with device ID:', identity.deviceId.substring(0, 16) + '...');
}

/**
 * Initialize the relay client
 */
async function initializeRelayClient() {
  const relayUrl = await storage.getRelayUrl();
  const sessionKey = await storage.getSessionKey();

  relayClient = new RelayClient({
    url: relayUrl,
    identity: currentState.identity,
    sessionKey,
    onStateChange: handleConnectionStateChange,
    onClipboardUpdate: handleClipboardUpdate,
    onError: handleRelayError,
  });

  // Auto-connect if we have paired devices
  if (currentState.pairedDevices.length > 0 && currentState.settings.autoSync) {
    relayClient.connect();
  }
}

/**
 * Handle connection state changes
 */
function handleConnectionStateChange(newState, oldState) {
  console.log(`Connection state: ${oldState} -> ${newState}`);
  currentState.connectionState = newState;

  // Update badge
  updateBadge(newState);

  // Notify popup if open
  broadcastToPopup({
    type: 'CONNECTION_STATE_CHANGED',
    state: newState,
  });
}

/**
 * Handle clipboard updates from other devices
 */
async function handleClipboardUpdate(update) {
  console.log('Received clipboard update from:', update.fromDevice);

  const settings = await storage.getSettings();

  // Check if content type is enabled
  const contentType = update.content.content_type || 'text';
  if (contentType === 'text' && !settings.syncTextEnabled) {
    return;
  }
  if (contentType === 'image' && !settings.syncImagesEnabled) {
    return;
  }
  if (contentType === 'url' && !settings.syncUrlsEnabled) {
    return;
  }

  // Add to history
  const historyItem = await storage.addToClipboardHistory({
    type: contentType,
    data: update.content.data,
    preview: update.content.metadata?.text_preview || update.content.data.substring(0, 200),
    fromDevice: update.fromDevice,
    synced: true,
  });

  // Show notification if enabled
  if (settings.notificationsEnabled) {
    chrome.notifications.create({
      type: 'basic',
      iconUrl: 'icons/icon128.png',
      title: 'Clipboard Synced',
      message: `Received from ${getDeviceName(update.fromDevice)}: ${historyItem.preview.substring(0, 50)}...`,
    });
  }

  // Notify popup
  broadcastToPopup({
    type: 'CLIPBOARD_UPDATE',
    item: historyItem,
  });
}

/**
 * Handle relay client errors
 */
function handleRelayError(error) {
  console.error('Relay client error:', error);

  broadcastToPopup({
    type: 'ERROR',
    message: error.message,
  });
}

/**
 * Update extension badge based on connection state
 */
function updateBadge(state) {
  let color, text;

  switch (state) {
  case ConnectionState.CONNECTED:
    color = '#22c55e'; // green
    text = '';
    break;
  case ConnectionState.CONNECTING:
  case ConnectionState.AUTHENTICATING:
    color = '#f59e0b'; // yellow
    text = '...';
    break;
  case ConnectionState.ERROR:
    color = '#ef4444'; // red
    text = '!';
    break;
  default:
    color = '#6b7280'; // gray
    text = '';
  }

  chrome.action.setBadgeBackgroundColor({ color });
  chrome.action.setBadgeText({ text });
}

/**
 * Setup context menus
 */
async function setupContextMenus() {
  // Remove existing menus
  await chrome.contextMenus.removeAll();

  // Create parent menu
  chrome.contextMenus.create({
    id: CONTEXT_MENU_SEND_TO_DEVICE,
    title: 'Send to Device',
    contexts: ['selection', 'link', 'image'],
  });

  // Create "Send to All" option
  chrome.contextMenus.create({
    id: CONTEXT_MENU_SEND_TO_ALL,
    parentId: CONTEXT_MENU_SEND_TO_DEVICE,
    title: 'All Devices',
    contexts: ['selection', 'link', 'image'],
  });

  // Add separator
  chrome.contextMenus.create({
    id: 'separator',
    parentId: CONTEXT_MENU_SEND_TO_DEVICE,
    type: 'separator',
    contexts: ['selection', 'link', 'image'],
  });

  // Add paired devices
  const devices = await storage.getPairedDevices();
  for (const device of devices) {
    chrome.contextMenus.create({
      id: `device_${device.deviceId}`,
      parentId: CONTEXT_MENU_SEND_TO_DEVICE,
      title: device.name || `Device ${device.deviceId.substring(0, 8)}`,
      contexts: ['selection', 'link', 'image'],
    });
  }

  // Add handler
  chrome.contextMenus.onClicked.addListener(handleContextMenuClick);
}

/**
 * Handle context menu clicks
 */
async function handleContextMenuClick(info, tab) {
  let content = null;
  let contentType = 'text';

  // Determine content based on what was clicked
  if (info.selectionText) {
    content = info.selectionText;
    contentType = detectContentType(content);
  } else if (info.linkUrl) {
    content = info.linkUrl;
    contentType = 'url';
  } else if (info.srcUrl) {
    content = info.srcUrl;
    contentType = 'image';
  }

  if (!content) {
    console.warn('No content to send');
    return;
  }

  // Determine target device(s)
  let targetDevices = [];

  if (info.menuItemId === CONTEXT_MENU_SEND_TO_ALL) {
    targetDevices = currentState.pairedDevices.map((d) => d.deviceId);
  } else if (info.menuItemId.startsWith('device_')) {
    const deviceId = info.menuItemId.replace('device_', '');
    targetDevices = [deviceId];
  }

  if (targetDevices.length === 0) {
    console.warn('No target devices');
    return;
  }

  // Send to devices
  await sendClipboardContent(content, contentType, targetDevices);
}

/**
 * Send clipboard content to devices
 */
async function sendClipboardContent(data, type, deviceIds) {
  if (!relayClient || !relayClient.isConnected()) {
    console.error('Not connected to relay server');
    return false;
  }

  const content = {
    type,
    data,
    preview: data.substring(0, 200),
  };

  // Add to local history
  await storage.addToClipboardHistory({
    ...content,
    synced: true,
    sentTo: deviceIds,
  });

  // Send to each device
  for (const deviceId of deviceIds) {
    try {
      await relayClient.sendClipboard(deviceId, content);
      console.log(`Sent clipboard to device: ${deviceId.substring(0, 16)}...`);
    } catch (error) {
      console.error(`Failed to send to device ${deviceId}:`, error);
    }
  }

  // Notify popup
  broadcastToPopup({
    type: 'CLIPBOARD_SENT',
    content,
    deviceIds,
  });

  return true;
}

/**
 * Detect content type from string
 */
function detectContentType(content) {
  if (/^https?:\/\//i.test(content)) {
    return 'url';
  }
  return 'text';
}

/**
 * Get device name by ID
 */
function getDeviceName(deviceId) {
  const device = currentState.pairedDevices.find((d) => d.deviceId === deviceId);
  return device?.name || `Device ${deviceId.substring(0, 8)}`;
}

/**
 * Broadcast message to popup
 */
function broadcastToPopup(message) {
  chrome.runtime.sendMessage(message).catch(() => {
    // Popup not open, ignore error
  });
}

/**
 * Handle messages from popup/options pages
 */
function handleMessage(request, sender, sendResponse) {
  const handlers = {
    GET_STATE: () => ({
      connectionState: currentState.connectionState,
      identity: currentState.identity,
      settings: currentState.settings,
      pairedDevices: currentState.pairedDevices,
    }),

    CONNECT: async () => {
      if (relayClient) {
        await relayClient.connect();
      }
      return { success: true };
    },

    DISCONNECT: async () => {
      if (relayClient) {
        relayClient.disconnect();
      }
      return { success: true };
    },

    SEND_CLIPBOARD: async (data) => {
      const { content, type, deviceIds } = data;
      const success = await sendClipboardContent(content, type, deviceIds);
      return { success };
    },

    GET_HISTORY: async () => {
      const history = await storage.getClipboardHistory();
      return { history };
    },

    CLEAR_HISTORY: async () => {
      await storage.clearClipboardHistory();
      return { success: true };
    },

    DELETE_HISTORY_ITEM: async (data) => {
      await storage.removeFromClipboardHistory(data.itemId);
      return { success: true };
    },

    GET_SETTINGS: async () => {
      const settings = await storage.getSettings();
      return { settings };
    },

    UPDATE_SETTINGS: async (data) => {
      const settings = await storage.updateSettings(data.settings);
      currentState.settings = settings;

      // Update relay URL if changed
      if (data.settings.relayUrl && relayClient) {
        relayClient.url = data.settings.relayUrl;
        if (relayClient.isConnected()) {
          relayClient.disconnect();
          await relayClient.connect();
        }
      }

      return { settings };
    },

    GET_PAIRED_DEVICES: async () => {
      const devices = await storage.getPairedDevices();
      return { devices };
    },

    ADD_PAIRED_DEVICE: async (data) => {
      await storage.addPairedDevice(data.device);
      currentState.pairedDevices = await storage.getPairedDevices();
      await setupContextMenus();
      return { success: true };
    },

    REMOVE_PAIRED_DEVICE: async (data) => {
      await storage.removePairedDevice(data.deviceId);
      currentState.pairedDevices = await storage.getPairedDevices();
      await setupContextMenus();
      return { success: true };
    },

    PAIR_WITH_CODE: async (data) => {
      // Handle pairing with 6-digit code from main app
      const { code, deviceInfo } = data;
      // TODO: Implement pairing protocol
      return { success: false, error: 'Pairing not yet implemented' };
    },

    GET_PAIRING_CODE: async () => {
      // Generate pairing code for main app to scan
      const identity = currentState.identity;
      const code = Math.floor(Math.random() * 1000000).toString().padStart(6, '0');

      return {
        code,
        deviceId: identity.deviceId,
        deviceName: identity.deviceName,
        publicKey: identity.publicKeyRaw,
      };
    },

    COPY_TO_CLIPBOARD: async (data) => {
      // Copy content to clipboard using offscreen document
      await copyToClipboard(data.content);
      return { success: true };
    },
  };

  const handler = handlers[request.type];
  if (handler) {
    if (handler.constructor.name === 'AsyncFunction') {
      handler(request.data)
        .then(sendResponse)
        .catch((error) => sendResponse({ error: error.message }));
      return true; // Keep channel open for async response
    } else {
      sendResponse(handler(request.data));
    }
  } else {
    sendResponse({ error: 'Unknown message type' });
  }

  return false;
}

/**
 * Copy content to clipboard using offscreen document (Chrome MV3)
 */
async function copyToClipboard(content) {
  // For MV3, we need to use an offscreen document for clipboard operations
  // or inject a content script. For simplicity, we'll use the navigator.clipboard API
  // which requires a user gesture, so this might fail in background context.
  try {
    await navigator.clipboard.writeText(content);
  } catch (error) {
    console.error('Failed to copy to clipboard:', error);
    // Fallback: send message to active tab to perform copy
    const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
    if (tab?.id) {
      chrome.tabs.sendMessage(tab.id, {
        type: 'COPY_TO_CLIPBOARD',
        content,
      });
    }
  }
}

/**
 * Handle extension install/update
 */
chrome.runtime.onInstalled.addListener(async (details) => {
  console.log('Extension installed/updated:', details.reason);

  if (details.reason === 'install') {
    // First install - open options page
    chrome.runtime.openOptionsPage();
  }

  await initialize();
});

/**
 * Handle browser startup
 */
chrome.runtime.onStartup.addListener(async () => {
  console.log('Browser started');
  await initialize();
});

// Initialize on service worker load
initialize().catch(console.error);
