import 'package:flutter/material.dart';

import '../../../core/services/websocket_service.dart';
import '../../../shared/constants/layout_constants.dart';
import '../../../shared/theme/app_theme.dart';

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

  Color getColor(ColorScheme colorScheme, AppStatusColors statusColors) {
    switch (this) {
      case SyncMethod.polling:
        return colorScheme.outline;
      case SyncMethod.websocket:
        return statusColors.online;
      case SyncMethod.reconnecting:
        return statusColors.warning;
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
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final statusColors = AppStatusColors.of(context);

    final isConnected = connectedCount > 0;
    final backgroundColor = isConnected
        ? colorScheme.primaryContainer
        : colorScheme.surfaceContainerHighest;
    final foregroundColor = isConnected
        ? colorScheme.onPrimaryContainer
        : colorScheme.onSurfaceVariant;
    final statusText = isConnected
        ? '$connectedCount device${connectedCount > 1 ? 's' : ''} connected'
        : 'No devices connected';

    return Semantics(
      liveRegion: true,
      label: '$statusText. Sync: ${syncMethod.displayName}',
      child: Container(
        width: double.infinity,
        padding: const EdgeInsets.symmetric(
          horizontal: LayoutConstants.defaultPadding,
          vertical: LayoutConstants.gutter,
        ),
        color: backgroundColor,
        child: Row(
          children: [
            // Status indicator
            Container(
              width: LayoutConstants.smallPadding,
              height: LayoutConstants.smallPadding,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: isConnected ? statusColors.online : statusColors.offline,
              ),
            ),
            const SizedBox(width: LayoutConstants.gutter),

            // Status text
            Expanded(
              child: Text(
                statusText,
                style: theme.textTheme.bodyMedium?.copyWith(
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
                  size: LayoutConstants.smallIconSize,
                  color: foregroundColor.withValues(alpha: 0.7),
                ),
              ),
              const SizedBox(width: LayoutConstants.smallPadding),
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
                      color: syncMethod.getColor(colorScheme, statusColors),
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
            const SizedBox(width: LayoutConstants.smallPadding),

            // Relay indicator
            if (relayConfigured) ...[
              Tooltip(
                message: 'Cloud relay enabled',
                child: Icon(
                  Icons.cloud_done,
                  size: LayoutConstants.smallIconSize,
                  color: foregroundColor.withValues(alpha: 0.7),
                ),
              ),
              const SizedBox(width: LayoutConstants.smallPadding),
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
                  const SizedBox(width: LayoutConstants.smallPadding),
                  Text(
                    'Syncing',
                    style: theme.textTheme.labelMedium?.copyWith(
                      color: foregroundColor,
                    ),
                  ),
                ],
              ),
          ],
        ),
      ),
    );
  }
}
