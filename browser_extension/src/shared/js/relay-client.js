/**
 * Toss Browser Extension - Relay Server WebSocket Client
 *
 * Handles WebSocket connection to the relay server,
 * authentication, and message relay.
 */

import {
  signAuthMessage,
  encrypt,
  decrypt,
  arrayBufferToBase64,
  base64ToUint8Array,
} from './crypto.js';

// WebSocket message types
export const WsMessageType = {
  AUTH: 'auth',
  AUTH_RESPONSE: 'auth_response',
  RELAY: 'relay',
  SEND: 'send',
  ERROR: 'error',
  PING: 'ping',
  PONG: 'pong',
};

// Connection states
export const ConnectionState = {
  DISCONNECTED: 'disconnected',
  CONNECTING: 'connecting',
  AUTHENTICATING: 'authenticating',
  CONNECTED: 'connected',
  ERROR: 'error',
};

/**
 * RelayClient - Manages WebSocket connection to the relay server
 */
export class RelayClient {
  constructor(options = {}) {
    this.url = options.url || 'wss://localhost:8080/api/v1/ws';
    this.identity = options.identity || null;
    this.sessionKey = options.sessionKey || null;
    this.ws = null;
    this.state = ConnectionState.DISCONNECTED;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = options.maxReconnectAttempts || 5;
    this.reconnectDelay = options.reconnectDelay || 1000;
    this.pingInterval = null;
    this.pingIntervalMs = options.pingIntervalMs || 30000;
    this.messageQueue = [];
    this.pendingMessages = new Map();
    this.messageIdCounter = 0;

    // Event handlers
    this.onStateChange = options.onStateChange || (() => {});
    this.onMessage = options.onMessage || (() => {});
    this.onClipboardUpdate = options.onClipboardUpdate || (() => {});
    this.onError = options.onError || (() => {});
    this.onDevicesUpdate = options.onDevicesUpdate || (() => {});
  }

  /**
   * Set device identity for authentication
   */
  setIdentity(identity) {
    this.identity = identity;
  }

  /**
   * Set session encryption key
   */
  setSessionKey(sessionKey) {
    this.sessionKey = sessionKey;
  }

  /**
   * Connect to the relay server
   */
  async connect() {
    if (this.state === ConnectionState.CONNECTING ||
        this.state === ConnectionState.CONNECTED) {
      return;
    }

    if (!this.identity) {
      throw new Error('Device identity not set');
    }

    this.setState(ConnectionState.CONNECTING);

    try {
      this.ws = new WebSocket(this.url);
      this.ws.binaryType = 'arraybuffer';

      this.ws.onopen = () => this.handleOpen();
      this.ws.onmessage = (event) => this.handleMessage(event);
      this.ws.onclose = (event) => this.handleClose(event);
      this.ws.onerror = (error) => this.handleError(error);
    } catch (error) {
      this.setState(ConnectionState.ERROR);
      this.onError(error);
      this.scheduleReconnect();
    }
  }

  /**
   * Disconnect from the relay server
   */
  disconnect() {
    this.stopPingInterval();
    this.reconnectAttempts = this.maxReconnectAttempts; // Prevent reconnect

    if (this.ws) {
      this.ws.close(1000, 'Client disconnect');
      this.ws = null;
    }

    this.setState(ConnectionState.DISCONNECTED);
  }

  /**
   * Handle WebSocket open event
   */
  async handleOpen() {
    this.setState(ConnectionState.AUTHENTICATING);
    this.reconnectAttempts = 0;

    try {
      await this.authenticate();
    } catch (error) {
      console.error('Authentication failed:', error);
      this.onError(error);
      this.disconnect();
    }
  }

  /**
   * Authenticate with the relay server
   */
  async authenticate() {
    const timestamp = Date.now();
    const signature = await signAuthMessage(
      this.identity.privateKey,
      this.identity.deviceId,
      timestamp
    );

    const authMessage = {
      type: WsMessageType.AUTH,
      device_id: this.identity.deviceId,
      timestamp,
      signature,
      public_key: this.identity.publicKeyRaw,
    };

    this.sendRaw(authMessage);
  }

  /**
   * Handle incoming WebSocket message
   */
  async handleMessage(event) {
    try {
      let message;

      if (typeof event.data === 'string') {
        message = JSON.parse(event.data);
      } else {
        // Binary message
        const text = new TextDecoder().decode(event.data);
        message = JSON.parse(text);
      }

      switch (message.type) {
        case WsMessageType.AUTH_RESPONSE:
          this.handleAuthResponse(message);
          break;

        case WsMessageType.RELAY:
          await this.handleRelayMessage(message);
          break;

        case WsMessageType.ERROR:
          console.error('Server error:', message.message);
          this.onError(new Error(message.message));
          break;

        case WsMessageType.PONG:
          // Pong received, connection is alive
          break;

        default:
          console.warn('Unknown message type:', message.type);
      }

      this.onMessage(message);
    } catch (error) {
      console.error('Error handling message:', error);
      this.onError(error);
    }
  }

  /**
   * Handle authentication response
   */
  handleAuthResponse(message) {
    if (message.success) {
      this.setState(ConnectionState.CONNECTED);
      this.startPingInterval();
      this.flushMessageQueue();

      // Store JWT token if provided
      if (message.token) {
        this.authToken = message.token;
      }
    } else {
      this.setState(ConnectionState.ERROR);
      this.onError(new Error(message.error || 'Authentication failed'));
      this.disconnect();
    }
  }

  /**
   * Handle relay message (clipboard update from another device)
   */
  async handleRelayMessage(message) {
    const relayMessage = message.message;

    if (!relayMessage || !relayMessage.encrypted_payload) {
      console.warn('Invalid relay message format');
      return;
    }

    try {
      // Decrypt the payload if we have a session key
      let content;
      if (this.sessionKey) {
        const decrypted = await decrypt(
          this.sessionKey,
          relayMessage.encrypted_payload
        );
        content = JSON.parse(new TextDecoder().decode(decrypted));
      } else {
        // If no session key, payload might be unencrypted JSON
        content = JSON.parse(atob(relayMessage.encrypted_payload));
      }

      // Extract clipboard content
      const clipboardUpdate = {
        id: relayMessage.id,
        fromDevice: relayMessage.from_device,
        timestamp: relayMessage.timestamp,
        content,
      };

      this.onClipboardUpdate(clipboardUpdate);
    } catch (error) {
      console.error('Failed to process relay message:', error);
    }
  }

  /**
   * Handle WebSocket close event
   */
  handleClose(event) {
    this.stopPingInterval();
    this.ws = null;

    if (event.code !== 1000) {
      // Abnormal close, attempt reconnect
      this.setState(ConnectionState.DISCONNECTED);
      this.scheduleReconnect();
    } else {
      this.setState(ConnectionState.DISCONNECTED);
    }
  }

  /**
   * Handle WebSocket error
   */
  handleError(error) {
    console.error('WebSocket error:', error);
    this.onError(error);
  }

  /**
   * Schedule reconnection attempt
   */
  scheduleReconnect() {
    if (this.reconnectAttempts >= this.maxReconnectAttempts) {
      console.log('Max reconnection attempts reached');
      return;
    }

    const delay = this.reconnectDelay * Math.pow(2, this.reconnectAttempts);
    this.reconnectAttempts++;

    console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})`);

    setTimeout(() => {
      if (this.state === ConnectionState.DISCONNECTED) {
        this.connect();
      }
    }, delay);
  }

  /**
   * Send clipboard content to a specific device
   */
  async sendClipboard(toDeviceId, content) {
    const payload = {
      content_type: content.type || 'text',
      data: content.data,
      metadata: {
        text_preview: content.preview || content.data.substring(0, 200),
        size_bytes: content.data.length,
        timestamp: Date.now(),
      },
    };

    // Encrypt payload if we have a session key
    let encryptedPayload;
    if (this.sessionKey) {
      encryptedPayload = await encrypt(
        this.sessionKey,
        JSON.stringify(payload)
      );
    } else {
      // Base64 encode for transport
      encryptedPayload = btoa(JSON.stringify(payload));
    }

    const message = {
      type: WsMessageType.SEND,
      to_device: toDeviceId,
      encrypted_payload: encryptedPayload,
    };

    return this.send(message);
  }

  /**
   * Send message through WebSocket
   */
  send(message) {
    if (this.state !== ConnectionState.CONNECTED) {
      // Queue message for later delivery
      this.messageQueue.push(message);
      return Promise.resolve(false);
    }

    return this.sendRaw(message);
  }

  /**
   * Send raw message without queueing
   */
  sendRaw(message) {
    if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
      return false;
    }

    try {
      this.ws.send(JSON.stringify(message));
      return true;
    } catch (error) {
      console.error('Failed to send message:', error);
      return false;
    }
  }

  /**
   * Flush queued messages after connection
   */
  flushMessageQueue() {
    while (this.messageQueue.length > 0) {
      const message = this.messageQueue.shift();
      this.sendRaw(message);
    }
  }

  /**
   * Start ping interval to keep connection alive
   */
  startPingInterval() {
    this.stopPingInterval();
    this.pingInterval = setInterval(() => {
      if (this.state === ConnectionState.CONNECTED) {
        this.sendRaw({ type: WsMessageType.PING });
      }
    }, this.pingIntervalMs);
  }

  /**
   * Stop ping interval
   */
  stopPingInterval() {
    if (this.pingInterval) {
      clearInterval(this.pingInterval);
      this.pingInterval = null;
    }
  }

  /**
   * Update connection state
   */
  setState(newState) {
    if (this.state !== newState) {
      const oldState = this.state;
      this.state = newState;
      this.onStateChange(newState, oldState);
    }
  }

  /**
   * Get current connection state
   */
  getState() {
    return this.state;
  }

  /**
   * Check if connected
   */
  isConnected() {
    return this.state === ConnectionState.CONNECTED;
  }
}
