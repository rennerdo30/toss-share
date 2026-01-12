import 'package:flutter/material.dart';

class ConnectionStatusBanner extends StatelessWidget {
  final int connectedCount;
  final bool isSyncing;

  const ConnectionStatusBanner({
    super.key,
    required this.connectedCount,
    this.isSyncing = false,
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
