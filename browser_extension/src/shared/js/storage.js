/**
 * Toss Browser Extension - Storage Manager
 *
 * Handles persistent storage for settings, clipboard history,
 * device identity, and paired devices.
 */

// Storage keys
export const StorageKeys = {
  IDENTITY: 'toss_identity',
  SETTINGS: 'toss_settings',
  CLIPBOARD_HISTORY: 'toss_clipboard_history',
  PAIRED_DEVICES: 'toss_paired_devices',
  SESSION_KEY: 'toss_session_key',
  RELAY_URL: 'toss_relay_url',
};

// Default settings
export const DefaultSettings = {
  autoSync: true,
  targetDevice: null,
  maxHistoryItems: 50,
  notificationsEnabled: true,
  relayUrl: 'wss://localhost:8080/api/v1/ws',
  syncTextEnabled: true,
  syncImagesEnabled: true,
  syncUrlsEnabled: true,
  theme: 'system',
};

/**
 * Get browser storage API (works for both Chrome and Firefox)
 */
function getStorage() {
  if (typeof chrome !== 'undefined' && chrome.storage) {
    return chrome.storage;
  }
  if (typeof browser !== 'undefined' && browser.storage) {
    return browser.storage;
  }
  // Fallback to localStorage wrapper for development
  return {
    local: {
      get: (keys) => {
        return new Promise((resolve) => {
          const result = {};
          const keyList = Array.isArray(keys) ? keys : [keys];
          keyList.forEach((key) => {
            const value = localStorage.getItem(key);
            if (value) {
              try {
                result[key] = JSON.parse(value);
              } catch {
                result[key] = value;
              }
            }
          });
          resolve(result);
        });
      },
      set: (items) => {
        return new Promise((resolve) => {
          Object.entries(items).forEach(([key, value]) => {
            localStorage.setItem(key, JSON.stringify(value));
          });
          resolve();
        });
      },
      remove: (keys) => {
        return new Promise((resolve) => {
          const keyList = Array.isArray(keys) ? keys : [keys];
          keyList.forEach((key) => localStorage.removeItem(key));
          resolve();
        });
      },
    },
  };
}

/**
 * Storage Manager class
 */
export class StorageManager {
  constructor() {
    this.storage = getStorage();
  }

  /**
   * Get value from storage
   */
  async get(key) {
    const result = await this.storage.local.get(key);
    return result[key];
  }

  /**
   * Set value in storage
   */
  async set(key, value) {
    await this.storage.local.set({ [key]: value });
  }

  /**
   * Remove value from storage
   */
  async remove(key) {
    await this.storage.local.remove(key);
  }

  /**
   * Get device identity
   */
  async getIdentity() {
    return this.get(StorageKeys.IDENTITY);
  }

  /**
   * Save device identity
   */
  async saveIdentity(identity) {
    await this.set(StorageKeys.IDENTITY, identity);
  }

  /**
   * Get settings
   */
  async getSettings() {
    const settings = await this.get(StorageKeys.SETTINGS);
    return { ...DefaultSettings, ...settings };
  }

  /**
   * Update settings
   */
  async updateSettings(updates) {
    const current = await this.getSettings();
    const updated = { ...current, ...updates };
    await this.set(StorageKeys.SETTINGS, updated);
    return updated;
  }

  /**
   * Get clipboard history
   */
  async getClipboardHistory() {
    const history = await this.get(StorageKeys.CLIPBOARD_HISTORY);
    return history || [];
  }

  /**
   * Add item to clipboard history
   */
  async addToClipboardHistory(item) {
    const settings = await this.getSettings();
    let history = await this.getClipboardHistory();

    // Add new item at the beginning
    const newItem = {
      id: crypto.randomUUID(),
      timestamp: Date.now(),
      ...item,
    };
    history.unshift(newItem);

    // Trim to max size
    if (history.length > settings.maxHistoryItems) {
      history = history.slice(0, settings.maxHistoryItems);
    }

    await this.set(StorageKeys.CLIPBOARD_HISTORY, history);
    return newItem;
  }

  /**
   * Remove item from clipboard history
   */
  async removeFromClipboardHistory(itemId) {
    let history = await this.getClipboardHistory();
    history = history.filter((item) => item.id !== itemId);
    await this.set(StorageKeys.CLIPBOARD_HISTORY, history);
  }

  /**
   * Clear clipboard history
   */
  async clearClipboardHistory() {
    await this.set(StorageKeys.CLIPBOARD_HISTORY, []);
  }

  /**
   * Get paired devices
   */
  async getPairedDevices() {
    const devices = await this.get(StorageKeys.PAIRED_DEVICES);
    return devices || [];
  }

  /**
   * Add paired device
   */
  async addPairedDevice(device) {
    const devices = await this.getPairedDevices();
    const existing = devices.findIndex((d) => d.deviceId === device.deviceId);

    if (existing >= 0) {
      devices[existing] = { ...devices[existing], ...device };
    } else {
      devices.push({
        ...device,
        addedAt: Date.now(),
      });
    }

    await this.set(StorageKeys.PAIRED_DEVICES, devices);
  }

  /**
   * Remove paired device
   */
  async removePairedDevice(deviceId) {
    let devices = await this.getPairedDevices();
    devices = devices.filter((d) => d.deviceId !== deviceId);
    await this.set(StorageKeys.PAIRED_DEVICES, devices);
  }

  /**
   * Update paired device
   */
  async updatePairedDevice(deviceId, updates) {
    const devices = await this.getPairedDevices();
    const index = devices.findIndex((d) => d.deviceId === deviceId);

    if (index >= 0) {
      devices[index] = { ...devices[index], ...updates };
      await this.set(StorageKeys.PAIRED_DEVICES, devices);
    }
  }

  /**
   * Get session key
   */
  async getSessionKey() {
    const keyBase64 = await this.get(StorageKeys.SESSION_KEY);
    if (!keyBase64) {
      return null;
    }

    // Convert base64 to Uint8Array
    const binary = atob(keyBase64);
    const bytes = new Uint8Array(binary.length);
    for (let i = 0; i < binary.length; i++) {
      bytes[i] = binary.charCodeAt(i);
    }
    return bytes;
  }

  /**
   * Save session key
   */
  async saveSessionKey(keyBytes) {
    // Convert Uint8Array to base64
    let binary = '';
    for (let i = 0; i < keyBytes.length; i++) {
      binary += String.fromCharCode(keyBytes[i]);
    }
    const keyBase64 = btoa(binary);
    await this.set(StorageKeys.SESSION_KEY, keyBase64);
  }

  /**
   * Clear session key
   */
  async clearSessionKey() {
    await this.remove(StorageKeys.SESSION_KEY);
  }

  /**
   * Get relay URL
   */
  async getRelayUrl() {
    const url = await this.get(StorageKeys.RELAY_URL);
    return url || DefaultSettings.relayUrl;
  }

  /**
   * Save relay URL
   */
  async saveRelayUrl(url) {
    await this.set(StorageKeys.RELAY_URL, url);
  }

  /**
   * Export all data for backup
   */
  async exportData() {
    const identity = await this.getIdentity();
    const settings = await this.getSettings();
    const history = await this.getClipboardHistory();
    const devices = await this.getPairedDevices();

    return {
      version: 1,
      exportedAt: Date.now(),
      identity,
      settings,
      clipboardHistory: history,
      pairedDevices: devices,
    };
  }

  /**
   * Import data from backup
   */
  async importData(data) {
    if (data.version !== 1) {
      throw new Error('Unsupported backup version');
    }

    if (data.identity) {
      await this.saveIdentity(data.identity);
    }
    if (data.settings) {
      await this.set(StorageKeys.SETTINGS, data.settings);
    }
    if (data.clipboardHistory) {
      await this.set(StorageKeys.CLIPBOARD_HISTORY, data.clipboardHistory);
    }
    if (data.pairedDevices) {
      await this.set(StorageKeys.PAIRED_DEVICES, data.pairedDevices);
    }
  }
}

// Singleton instance
export const storage = new StorageManager();
