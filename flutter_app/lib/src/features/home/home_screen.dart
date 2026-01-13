import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/providers/toss_provider.dart';
import '../../core/providers/devices_provider.dart';
import '../../core/providers/clipboard_provider.dart';
import '../../core/services/toss_service.dart';
import '../../core/models/device.dart';
import 'widgets/connection_status.dart';
import 'widgets/device_list.dart';
import 'widgets/clipboard_preview.dart';

class HomeScreen extends ConsumerWidget {
  const HomeScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tossState = ref.watch(tossProvider);
    final devices = ref.watch(devicesProvider);
    final currentClipboard = ref.watch(currentClipboardProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Toss'),
        actions: [
          IconButton(
            icon: const Icon(Icons.history),
            tooltip: 'Clipboard History',
            onPressed: () => context.push('/history'),
          ),
          IconButton(
            icon: const Icon(Icons.settings),
            tooltip: 'Settings',
            onPressed: () => context.push('/settings'),
          ),
        ],
      ),
      body: SafeArea(
        child: Column(
          children: [
            // Connection status banner
            ConnectionStatusBanner(
              connectedCount: devices.where((d) => d.isOnline).length,
              isSyncing: tossState.isSyncing,
            ),

            // Main content
            Expanded(
              child: Padding(
                padding: const EdgeInsets.all(16),
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    // Devices section
                    Row(
                      mainAxisAlignment: MainAxisAlignment.spaceBetween,
                      children: [
                        Text(
                          'Devices',
                          style: Theme.of(context).textTheme.titleMedium,
                        ),
                        TextButton.icon(
                          onPressed: () => context.push('/pairing'),
                          icon: const Icon(Icons.add, size: 18),
                          label: const Text('Add'),
                        ),
                      ],
                    ),
                    const SizedBox(height: 8),

                    // Device list
                    if (devices.isEmpty)
                      _EmptyDevicesCard(
                        onAddDevice: () => context.push('/pairing'),
                      )
                    else
                      DeviceList(
                        devices: devices,
                        onDeviceTap: (device) {
                          _showDeviceDetails(context, device, ref);
                        },
                      ),

                    const SizedBox(height: 24),

                    // Clipboard section
                    Text(
                      'Clipboard',
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                    const SizedBox(height: 8),

                    // Clipboard preview
                    Expanded(
                      child: ClipboardPreviewCard(
                        item: currentClipboard,
                        onRefresh: () async {
                          // Refresh clipboard from Rust core
                          final item = await TossService.getCurrentClipboard();
                          if (item != null) {
                            // Update provider with new clipboard content
                            // Note: This will be fully implemented once FFI bindings are available
                          }
                        },
                      ),
                    ),
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: devices.isEmpty
            ? null
            : () async {
                try {
                  await TossService.sendClipboard();
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      const SnackBar(content: Text('Clipboard sent successfully!')),
                    );
                  }
                } catch (e) {
                  if (context.mounted) {
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(content: Text('Failed to send clipboard: $e')),
                    );
                  }
                }
              },
        icon: const Icon(Icons.send),
        label: const Text('Send'),
      ),
    );
  }

  void _showDeviceDetails(BuildContext context, Device device, WidgetRef ref) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(device.name),
        content: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            _DetailRow(label: 'Device ID', value: device.id.substring(0, 16) + '...'),
            _DetailRow(
              label: 'Status',
              value: device.isOnline ? 'Online' : 'Offline',
            ),
            if (device.lastSeen != null)
              _DetailRow(
                label: 'Last Seen',
                value: _formatLastSeen(device.lastSeen!),
              ),
          ],
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Close'),
          ),
          TextButton(
            onPressed: () async {
              Navigator.pop(context);
              // Remove device
              await TossService.removeDevice(device.id);
              ref.read(devicesProvider.notifier).refresh();
              if (context.mounted) {
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Device removed')),
                );
              }
            },
            child: const Text('Remove', style: TextStyle(color: Colors.red)),
          ),
        ],
      ),
    );
  }

  String _formatLastSeen(DateTime lastSeen) {
    final now = DateTime.now();
    final diff = now.difference(lastSeen);

    if (diff.inMinutes < 1) return 'Just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    if (diff.inDays < 7) return '${diff.inDays}d ago';
    return '${lastSeen.month}/${lastSeen.day}/${lastSeen.year}';
  }
}

class _DetailRow extends StatelessWidget {
  final String label;
  final String value;

  const _DetailRow({required this.label, required this.value});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 80,
            child: Text(
              label,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: Theme.of(context).colorScheme.outline,
              ),
            ),
          ),
          Expanded(
            child: Text(
              value,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
          ),
        ],
      ),
    );
  }
}

class _EmptyDevicesCard extends StatelessWidget {
  final VoidCallback onAddDevice;

  const _EmptyDevicesCard({required this.onAddDevice});

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.devices_other,
              size: 48,
              color: Theme.of(context).colorScheme.outline,
            ),
            const SizedBox(height: 16),
            Text(
              'No devices paired',
              style: Theme.of(context).textTheme.titleSmall,
            ),
            const SizedBox(height: 8),
            Text(
              'Pair a device to start sharing your clipboard',
              style: Theme.of(context).textTheme.bodySmall,
              textAlign: TextAlign.center,
            ),
            const SizedBox(height: 16),
            ElevatedButton.icon(
              onPressed: onAddDevice,
              icon: const Icon(Icons.add),
              label: const Text('Add Device'),
            ),
          ],
        ),
      ),
    );
  }
}
