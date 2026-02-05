//! WebSocket service for real-time clipboard sync
//!
//! Provides WebSocket connection to the relay server for instant clipboard
//! updates instead of polling.

import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:math';

import 'package:toss/src/rust/api.dart' as api;

import 'logging_service.dart';
import 'toss_service.dart';

/// Connection state for WebSocket
enum WebSocketConnectionState {
  /// Not connected, not trying to connect
  disconnected,

  /// Attempting to connect
  connecting,

  /// Connected but not yet authenticated
  connected,

  /// Connected and authenticated, ready for messages
  authenticated,

  /// Connection failed, will retry
  reconnecting,
}

/// WebSocket message types matching the relay server protocol
class WsMessageType {
  static const String auth = 'auth';
  static const String relay = 'relay';
  static const String send = 'send';
  static const String authResponse = 'auth_response';
  static const String error = 'error';
}

/// Relay message received from WebSocket
class RelayMessage {
  final String id;
  final String fromDevice;
  final String toDevice;
  final String encryptedPayload;
  final int timestamp;

  const RelayMessage({
    required this.id,
    required this.fromDevice,
    required this.toDevice,
    required this.encryptedPayload,
    required this.timestamp,
  });

  factory RelayMessage.fromJson(Map<String, dynamic> json) {
    return RelayMessage(
      id: json['id'] as String,
      fromDevice: json['from_device'] as String,
      toDevice: json['to_device'] as String,
      encryptedPayload: json['encrypted_payload'] as String,
      timestamp: json['timestamp'] as int,
    );
  }
}

/// Callback types for WebSocket events
typedef OnRelayMessageCallback = void Function(RelayMessage message);
typedef OnConnectionStateCallback = void Function(WebSocketConnectionState state);
typedef OnErrorCallback = void Function(String error);

/// Service for managing WebSocket connection to the relay server
class WebSocketService {
  WebSocketService._();

  static final WebSocketService _instance = WebSocketService._();
  static WebSocketService get instance => _instance;

  /// Current WebSocket connection
  WebSocket? _socket;

  /// Current connection state
  WebSocketConnectionState _state = WebSocketConnectionState.disconnected;
  WebSocketConnectionState get state => _state;

  /// Stream controller for state changes
  final _stateController = StreamController<WebSocketConnectionState>.broadcast();
  Stream<WebSocketConnectionState> get stateStream => _stateController.stream;

  /// Stream controller for relay messages
  final _messageController = StreamController<RelayMessage>.broadcast();
  Stream<RelayMessage> get messageStream => _messageController.stream;

  /// Callbacks
  OnRelayMessageCallback? onRelayMessage;
  OnConnectionStateCallback? onConnectionStateChange;
  OnErrorCallback? onError;

  /// Connection configuration
  String? _relayUrl;
  String? _deviceId;

  /// Reconnection settings
  int _reconnectAttempts = 0;
  static const int _maxReconnectAttempts = 10;
  static const Duration _initialReconnectDelay = Duration(seconds: 1);
  static const Duration _maxReconnectDelay = Duration(seconds: 60);
  Timer? _reconnectTimer;
  bool _shouldReconnect = true;

  /// Ping/pong for keep-alive
  Timer? _pingTimer;
  static const Duration _pingInterval = Duration(seconds: 30);

  /// Check if connected and authenticated
  bool get isConnected => _state == WebSocketConnectionState.authenticated;

  /// Connect to the WebSocket endpoint
  ///
  /// [relayUrl] - Base URL of the relay server (e.g., 'https://relay.example.com')
  /// [deviceId] - This device's ID
  Future<void> connect({
    required String relayUrl,
    required String deviceId,
  }) async {
    // Store configuration
    _relayUrl = relayUrl;
    _deviceId = deviceId;
    _shouldReconnect = true;

    await _connect();
  }

  /// Internal connect method
  Future<void> _connect() async {
    if (_relayUrl == null || _deviceId == null) {
      LoggingService.warn('WebSocketService: Cannot connect - missing configuration');
      return;
    }

    // Don't connect if already connecting or connected
    if (_state == WebSocketConnectionState.connecting ||
        _state == WebSocketConnectionState.connected ||
        _state == WebSocketConnectionState.authenticated) {
      LoggingService.debug('WebSocketService: Already connected or connecting');
      return;
    }

    _setState(WebSocketConnectionState.connecting);
    LoggingService.info('WebSocketService: Connecting to $_relayUrl');

    try {
      // Convert HTTP(S) URL to WS(S) URL
      final wsUrl = _buildWsUrl(_relayUrl!);
      LoggingService.debug('WebSocketService: WebSocket URL: $wsUrl');

      // Connect with timeout
      _socket = await WebSocket.connect(wsUrl).timeout(
        const Duration(seconds: 10),
        onTimeout: () {
          throw TimeoutException('WebSocket connection timed out');
        },
      );

      _setState(WebSocketConnectionState.connected);
      LoggingService.info('WebSocketService: Connected, authenticating...');

      // Set up message listener
      _socket!.listen(
        _handleMessage,
        onError: _handleError,
        onDone: _handleDone,
        cancelOnError: false,
      );

      // Send authentication message
      await _authenticate();

      // Reset reconnect attempts on successful connection
      _reconnectAttempts = 0;

      // Start ping timer
      _startPingTimer();
    } catch (e) {
      LoggingService.error('WebSocketService: Connection failed', e);
      _setState(WebSocketConnectionState.disconnected);
      _scheduleReconnect();
    }
  }

  /// Build WebSocket URL from HTTP URL
  String _buildWsUrl(String httpUrl) {
    var url = httpUrl;

    // Remove trailing slash
    if (url.endsWith('/')) {
      url = url.substring(0, url.length - 1);
    }

    // Convert http(s) to ws(s)
    if (url.startsWith('https://')) {
      url = 'wss://${url.substring(8)}';
    } else if (url.startsWith('http://')) {
      url = 'ws://${url.substring(7)}';
    } else if (!url.startsWith('ws://') && !url.startsWith('wss://')) {
      // Assume secure WebSocket if no protocol specified
      url = 'wss://$url';
    }

    // Add WebSocket endpoint path
    return '$url/api/v1/ws';
  }

  /// Send authentication message
  Future<void> _authenticate() async {
    if (_socket == null || _deviceId == null) return;

    final timestamp = DateTime.now().millisecondsSinceEpoch ~/ 1000;

    // Get signature from Rust core
    String signature;
    try {
      signature = await _getAuthSignature(_deviceId!, timestamp);
    } catch (e) {
      LoggingService.error('WebSocketService: Failed to get auth signature', e);
      _disconnect();
      return;
    }

    final authMessage = {
      'type': WsMessageType.auth,
      'device_id': _deviceId,
      'timestamp': timestamp,
      'signature': signature,
    };

    _sendJson(authMessage);
  }

  /// Get authentication signature from Rust core
  Future<String> _getAuthSignature(String deviceId, int timestamp) async {
    // The message format expected by the server: "auth:{device_id}:{timestamp}"
    // We need to sign this with our private key
    // This should be implemented in the Rust FFI
    try {
      final signature = await _signAuthMessage(deviceId, timestamp);
      return signature;
    } catch (e) {
      LoggingService.error('WebSocketService: Failed to sign auth message', e);
      rethrow;
    }
  }

  /// Sign the authentication message using Rust FFI
  /// This is a placeholder - needs to be implemented in api.dart
  Future<String> _signAuthMessage(String deviceId, int timestamp) async {
    // Import the API and call the signing function
    // For now, we'll use a placeholder that calls into the Rust core
    try {
      // This needs to be added to the Rust FFI API
      // The signature is: sign("auth:{device_id}:{timestamp}")
      final message = 'auth:$deviceId:$timestamp';
      final signature = TossService.isFfiAvailable
          ? await _signMessageViaFfi(message)
          : throw Exception('FFI not available');
      return signature;
    } catch (e) {
      LoggingService.error('WebSocketService: Signing failed', e);
      rethrow;
    }
  }

  /// Sign a message using the Rust FFI
  Future<String> _signMessageViaFfi(String message) async {
    // Call the Rust FFI signing function
    return api.signMessage(message: message);
  }

  /// Handle incoming WebSocket message
  void _handleMessage(dynamic data) {
    if (data is! String) {
      LoggingService.warn('WebSocketService: Received non-string message');
      return;
    }

    try {
      final json = jsonDecode(data) as Map<String, dynamic>;
      final type = json['type'] as String?;

      switch (type) {
        case WsMessageType.authResponse:
          _handleAuthResponse(json);
          break;
        case WsMessageType.relay:
          _handleRelayMessage(json);
          break;
        case WsMessageType.error:
          _handleErrorMessage(json);
          break;
        default:
          LoggingService.debug('WebSocketService: Unknown message type: $type');
      }
    } catch (e) {
      LoggingService.error('WebSocketService: Failed to parse message', e);
    }
  }

  /// Handle authentication response
  void _handleAuthResponse(Map<String, dynamic> json) {
    final success = json['success'] as bool? ?? false;
    final error = json['error'] as String?;

    if (success) {
      _setState(WebSocketConnectionState.authenticated);
      LoggingService.info('WebSocketService: Authenticated successfully');
    } else {
      LoggingService.error('WebSocketService: Authentication failed: $error');
      onError?.call('Authentication failed: ${error ?? "unknown error"}');
      _disconnect();
      // Don't reconnect on auth failure - likely a configuration issue
      _shouldReconnect = false;
    }
  }

  /// Handle relay message (clipboard content from another device)
  void _handleRelayMessage(Map<String, dynamic> json) {
    final messageData = json['message'] as Map<String, dynamic>?;
    if (messageData == null) {
      LoggingService.warn('WebSocketService: Relay message missing message field');
      return;
    }

    try {
      final message = RelayMessage.fromJson(messageData);
      LoggingService.debug(
        'WebSocketService: Received relay message from ${message.fromDevice}'
      );

      // Emit to stream
      _messageController.add(message);

      // Call callback if set
      onRelayMessage?.call(message);
    } catch (e) {
      LoggingService.error('WebSocketService: Failed to parse relay message', e);
    }
  }

  /// Handle error message from server
  void _handleErrorMessage(Map<String, dynamic> json) {
    final message = json['message'] as String? ?? 'Unknown error';
    LoggingService.warn('WebSocketService: Server error: $message');
    onError?.call(message);
  }

  /// Handle WebSocket error
  void _handleError(dynamic error) {
    LoggingService.error('WebSocketService: WebSocket error', error);
    onError?.call(error.toString());
  }

  /// Handle WebSocket connection closed
  void _handleDone() {
    LoggingService.info('WebSocketService: Connection closed');
    _stopPingTimer();
    _socket = null;

    if (_state != WebSocketConnectionState.disconnected) {
      _setState(WebSocketConnectionState.disconnected);
      _scheduleReconnect();
    }
  }

  /// Schedule a reconnection attempt with exponential backoff
  void _scheduleReconnect() {
    if (!_shouldReconnect) {
      LoggingService.debug('WebSocketService: Reconnection disabled');
      return;
    }

    if (_reconnectAttempts >= _maxReconnectAttempts) {
      LoggingService.warn(
        'WebSocketService: Max reconnection attempts reached ($_maxReconnectAttempts)'
      );
      onError?.call('Failed to reconnect after $_maxReconnectAttempts attempts');
      return;
    }

    _reconnectAttempts++;
    _setState(WebSocketConnectionState.reconnecting);

    // Calculate delay with exponential backoff and jitter
    final baseDelay = _initialReconnectDelay.inMilliseconds *
        pow(2, _reconnectAttempts - 1).toInt();
    final maxDelay = _maxReconnectDelay.inMilliseconds;
    final delay = min(baseDelay, maxDelay);

    // Add jitter (±20%)
    final jitter = (delay * 0.2 * (Random().nextDouble() * 2 - 1)).toInt();
    final finalDelay = Duration(milliseconds: delay + jitter);

    LoggingService.info(
      'WebSocketService: Reconnecting in ${finalDelay.inSeconds}s '
      '(attempt $_reconnectAttempts/$_maxReconnectAttempts)'
    );

    _reconnectTimer?.cancel();
    _reconnectTimer = Timer(finalDelay, () {
      _connect();
    });
  }

  /// Start ping timer for keep-alive
  void _startPingTimer() {
    _stopPingTimer();
    _pingTimer = Timer.periodic(_pingInterval, (_) {
      if (_socket != null && _state == WebSocketConnectionState.authenticated) {
        try {
          // WebSocket ping is handled by the Dart WebSocket class automatically
          // But we can send a custom ping if needed
          LoggingService.debug('WebSocketService: Ping');
        } catch (e) {
          LoggingService.warn('WebSocketService: Ping failed: $e');
        }
      }
    });
  }

  /// Stop ping timer
  void _stopPingTimer() {
    _pingTimer?.cancel();
    _pingTimer = null;
  }

  /// Send JSON message
  void _sendJson(Map<String, dynamic> json) {
    if (_socket == null) {
      LoggingService.warn('WebSocketService: Cannot send - not connected');
      return;
    }

    try {
      final encoded = jsonEncode(json);
      _socket!.add(encoded);
    } catch (e) {
      LoggingService.error('WebSocketService: Failed to send message', e);
    }
  }

  /// Send clipboard content to a specific device
  void sendToDevice(String toDevice, String encryptedPayload) {
    if (_state != WebSocketConnectionState.authenticated) {
      LoggingService.warn('WebSocketService: Cannot send - not authenticated');
      return;
    }

    final message = {
      'type': WsMessageType.send,
      'to_device': toDevice,
      'encrypted_payload': encryptedPayload,
    };

    _sendJson(message);
  }

  /// Update connection state and notify listeners
  void _setState(WebSocketConnectionState newState) {
    if (_state == newState) return;

    _state = newState;
    _stateController.add(newState);
    onConnectionStateChange?.call(newState);

    LoggingService.debug('WebSocketService: State changed to $newState');
  }

  /// Disconnect from WebSocket
  void _disconnect() {
    _stopPingTimer();
    _reconnectTimer?.cancel();
    _reconnectTimer = null;

    if (_socket != null) {
      try {
        _socket!.close();
      } catch (e) {
        LoggingService.warn('WebSocketService: Error closing socket: $e');
      }
      _socket = null;
    }

    _setState(WebSocketConnectionState.disconnected);
  }

  /// Disconnect and stop reconnection attempts
  void disconnect() {
    LoggingService.info('WebSocketService: Disconnecting');
    _shouldReconnect = false;
    _disconnect();
  }

  /// Reconnect (e.g., after settings change)
  Future<void> reconnect() async {
    LoggingService.info('WebSocketService: Manual reconnect requested');
    _shouldReconnect = true;
    _reconnectAttempts = 0;
    _disconnect();
    await _connect();
  }

  /// Reset reconnect attempts (e.g., when user manually triggers reconnect)
  void resetReconnectAttempts() {
    _reconnectAttempts = 0;
  }

  /// Dispose of resources
  void dispose() {
    disconnect();
    _stateController.close();
    _messageController.close();
  }
}
