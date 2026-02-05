import 'package:flutter/material.dart';

import '../../../core/services/websocket_service.dart';

/// Sync method for real-time updates
enum SyncMethod {
  /// Polling-based sync (fallback)
  polling,

  /// WebSocket-based real-time sync
  websocket,

  /// WebSocket reconnecting
  reconnecting;

  String get displayName {
    switch (this) {
      case SyncMethod.polling:
        return 'Polling';
      case SyncMethod.websocket:
        return 'Real-time';
      case SyncMethod.reconnecting:
        return 'Reconnecting';
    }
  }

  IconData get icon {
    switch (this) {
      case SyncMethod.polling:
        return Icons.refresh;
      case SyncMethod.websocket:
        return Icons.bolt;
      case SyncMethod.reconnecting:
        return Icons.sync;
    }
  }

  Color getColor(ColorScheme colorScheme) {
    switch (this) {
      case SyncMethod.polling:
        return colorScheme.outline;
      case SyncMethod.websocket:
        return Colors.green;
      case SyncMethod.reconnecting:
        return Colors.orange;
    }
  }

  static SyncMethod fromWebSocketState(WebSocketConnectionState state) {
    switch (state) {
      case WebSocketConnectionState.authenticated:
        return SyncMethod.websocket;
      case WebSocketConnectionState.connecting:
      case WebSocketConnectionState.connected:
      case WebSocketConnectionState.reconnecting:
        return SyncMethod.reconnecting;
      case WebSocketConnectionState.disconnected:
        return SyncMethod.polling;
    }
  }
}

/// Connection type enum matching the Rust ConnectionType
enum ConnectionType {
  direct,
  stunReflexive,
  turnRelay,
  websocketRelay,
  unknown;

  static ConnectionType fromString(String value) {
    switch (value) {
      case 'direct':
        return ConnectionType.direct;
      case 'stun_reflexive':
        return ConnectionType.stunReflexive;
      case 'turn_relay':
        return ConnectionType.turnRelay;
      case 'websocket_relay':
        return ConnectionType.websocketRelay;
      default:
        return ConnectionType.unknown;
    }
  }

  String get displayName {
    switch (this) {
      case ConnectionType.direct:
        return 'Direct';
      case ConnectionType.stunReflexive:
        return 'NAT Traversal';
      case ConnectionType.turnRelay:
        return 'TURN Relay';
      case ConnectionType.websocketRelay:
        return 'Cloud Relay';
      case ConnectionType.unknown:
        return 'Unknown';
    }
  }

  IconData get icon {
    switch (this) {
      case ConnectionType.direct:
        return Icons.wifi;
      case ConnectionType.stunReflexive:
        return Icons.swap_horiz;
      case ConnectionType.turnRelay:
        return Icons.compare_arrows;
      case ConnectionType.websocketRelay:
        return Icons.cloud;
      case ConnectionType.unknown:
        return Icons.help_outline;
    }
  }
}

class ConnectionStatusBanner extends StatelessWidget {
  final int connectedCount;
  final bool isSyncing;
  final bool relayConfigured;
  final ConnectionType? connectionType;
  final SyncMethod syncMethod;

  const ConnectionStatusBanner({
    super.key,
    required this.connectedCount,
    this.isSyncing = false,
    this.relayConfigured = false,
    this.connectionType,
    this.syncMethod = SyncMethod.polling,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    final isConnected = connectedCount > 0;
    final backgroundColor = isConnected
        ? colorScheme.primaryContainer
        : colorScheme.surfaceContainerHighest;
    final foregroundColor = isConnected
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurfaceVariant;

    return Container(
      width: double.infinity,
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
      color: backgroundColor,
      child: Row(
        children: [
          // Status indicator
          Container(
            width: 8,
            height: 8,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: isConnected ? Colors.green : Colors.grey,
            ),
          ),
          const SizedBox(width: 12),

          // Status text
          Expanded(
            child: Text(
              isConnected
                  ? '$connectedCount device${connectedCount > 1 ? 's' : ''} connected'
                  : 'No devices connected',
              style: TextStyle(
                color: foregroundColor,
                fontWeight: FontWeight.w500,
              ),
            ),
          ),

          // Connection type indicator (when connected)
          if (isConnected && connectionType != null) ...[
            Tooltip(
              message: 'Connection: ${connectionType!.displayName}',
              child: Icon(
                connectionType!.icon,
                size: 16,
                color: foregroundColor.withValues(alpha: 0.7),
              ),
            ),
            const SizedBox(width: 8),
          ],

          // Sync method indicator (WebSocket vs Polling)
          Tooltip(
            message: 'Sync: ${syncMethod.displayName}',
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Container(
                  width: 6,
                  height: 6,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: syncMethod.getColor(colorScheme),
                  ),
                ),
                const SizedBox(width: 4),
                Icon(
                  syncMethod.icon,
                  size: 14,
                  color: foregroundColor.withValues(alpha: 0.7),
                ),
              ],
            ),
          ),
          const SizedBox(width: 8),

          // Relay indicator
          if (relayConfigured) ...[
            Tooltip(
              message: 'Cloud relay enabled',
              child: Icon(
                Icons.cloud_done,
                size: 16,
                color: foregroundColor.withValues(alpha: 0.7),
              ),
            ),
            const SizedBox(width: 8),
          ],

          // Sync indicator
          if (isSyncing)
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                SizedBox(
                  width: 14,
                  height: 14,
                  child: CircularProgressIndicator(
                    strokeWidth: 2,
                    color: foregroundColor,
                  ),
                ),
                const SizedBox(width: 8),
                Text(
                  'Syncing',
                  style: TextStyle(
                    color: foregroundColor,
                    fontSize: 12,
                  ),
                ),
              ],
            ),
        ],
      ),
    );
  }
}
