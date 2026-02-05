import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:riverpod_annotation/riverpod_annotation.dart';

import '../services/websocket_service.dart';
import '../services/toss_service.dart';
import '../services/logging_service.dart';
import 'settings_provider.dart';

part 'websocket_provider.g.dart';

/// WebSocket connection state for UI
class WebSocketState {
  final WebSocketConnectionState connectionState;
  final bool isConnected;
  final bool isReconnecting;
  final String? lastError;
  final DateTime? connectedSince;
  final int reconnectAttempts;

  const WebSocketState({
    required this.connectionState,
    required this.isConnected,
    required this.isReconnecting,
    this.lastError,
    this.connectedSince,
    this.reconnectAttempts = 0,
  });

  factory WebSocketState.initial() => const WebSocketState(
        connectionState: WebSocketConnectionState.disconnected,
        isConnected: false,
        isReconnecting: false,
      );

  WebSocketState copyWith({
    WebSocketConnectionState? connectionState,
    bool? isConnected,
    bool? isReconnecting,
    String? lastError,
    DateTime? connectedSince,
    int? reconnectAttempts,
  }) {
    return WebSocketState(
      connectionState: connectionState ?? this.connectionState,
      isConnected: isConnected ?? this.isConnected,
      isReconnecting: isReconnecting ?? this.isReconnecting,
      lastError: lastError ?? this.lastError,
      connectedSince: connectedSince ?? this.connectedSince,
      reconnectAttempts: reconnectAttempts ?? this.reconnectAttempts,
    );
  }
}

/// Provider for WebSocket connection management
@Riverpod(keepAlive: true)
class WebSocket extends _$WebSocket {
  StreamSubscription<WebSocketConnectionState>? _stateSubscription;
  StreamSubscription<RelayMessage>? _messageSubscription;

  @override
  WebSocketState build() {
    ref.onDispose(() {
      _stateSubscription?.cancel();
      _messageSubscription?.cancel();
    });
    return WebSocketState.initial();
  }

  /// Start WebSocket connection
  ///
  /// Should be called when the app starts and relay URL is configured
  Future<void> connect() async {
    final settings = ref.read(settingsProvider);

    // Check if relay URL is configured
    if (settings.relayUrl == null || settings.relayUrl!.isEmpty) {
      LoggingService.debug(
          'WebSocketProvider: No relay URL configured, skipping WebSocket connection');
      return;
    }

    // Check if FFI is available
    if (!TossService.isFfiAvailable) {
      LoggingService.debug(
          'WebSocketProvider: FFI not available, skipping WebSocket connection');
      return;
    }

    final deviceId = TossService.deviceId;
    if (deviceId == null) {
      LoggingService.warn(
          'WebSocketProvider: Device ID not available, skipping WebSocket connection');
      return;
    }

    // Set up state listener
    _stateSubscription?.cancel();
    _stateSubscription =
        WebSocketService.instance.stateStream.listen(_handleStateChange);

    // Set up message listener
    _messageSubscription?.cancel();
    _messageSubscription =
        WebSocketService.instance.messageStream.listen(_handleMessage);

    // Set up error callback
    WebSocketService.instance.onError = (error) {
      state = state.copyWith(lastError: error);
    };

    // Connect
    try {
      await WebSocketService.instance.connect(
        relayUrl: settings.relayUrl!,
        deviceId: deviceId,
      );
    } catch (e) {
      LoggingService.error('WebSocketProvider: Failed to connect', e);
      state = state.copyWith(
        lastError: e.toString(),
      );
    }
  }

  /// Disconnect WebSocket
  void disconnect() {
    WebSocketService.instance.disconnect();
    _stateSubscription?.cancel();
    _stateSubscription = null;
    _messageSubscription?.cancel();
    _messageSubscription = null;
    state = WebSocketState.initial();
  }

  /// Reconnect WebSocket (e.g., after settings change)
  Future<void> reconnect() async {
    disconnect();
    await connect();
  }

  /// Handle WebSocket state changes
  void _handleStateChange(WebSocketConnectionState newState) {
    final isConnected = newState == WebSocketConnectionState.authenticated;
    final isReconnecting = newState == WebSocketConnectionState.reconnecting;

    state = state.copyWith(
      connectionState: newState,
      isConnected: isConnected,
      isReconnecting: isReconnecting,
      connectedSince: isConnected ? DateTime.now() : state.connectedSince,
      // Clear error on successful connection
      lastError: isConnected ? null : state.lastError,
    );

    debugPrint('WebSocketProvider: State changed to $newState');
  }

  /// Handle incoming relay messages
  void _handleMessage(RelayMessage message) {
    // Log received message
    LoggingService.debug(
        'WebSocketProvider: Received message from ${message.fromDevice}');

    // The actual clipboard processing is handled by the ClipboardMonitorService
    // through the Rust FFI event polling. WebSocket messages are received
    // by the Rust core and processed there, then emitted as TossEvents.
    //
    // This callback is for any Flutter-side processing if needed.
  }
}

/// Provider for whether WebSocket is the active sync method
@riverpod
bool isWebSocketActive(IsWebSocketActiveRef ref) {
  final wsState = ref.watch(webSocketProvider);
  return wsState.isConnected;
}

/// Provider for current sync method description
@riverpod
String syncMethodDescription(SyncMethodDescriptionRef ref) {
  final wsState = ref.watch(webSocketProvider);

  if (wsState.isConnected) {
    return 'Real-time (WebSocket)';
  } else if (wsState.isReconnecting) {
    return 'Reconnecting...';
  } else {
    return 'Polling';
  }
}
