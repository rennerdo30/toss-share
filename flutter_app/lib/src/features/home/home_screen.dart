import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/providers/toss_provider.dart';
import '../../core/providers/devices_provider.dart';
import '../../core/providers/clipboard_provider.dart';
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
                          // TODO: Show device details
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
                        onRefresh: () {
                          // TODO: Refresh clipboard
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
            : () {
                // TODO: Send clipboard
                ScaffoldMessenger.of(context).showSnackBar(
                  const SnackBar(content: Text('Sending clipboard...')),
                );
              },
        icon: const Icon(Icons.send),
        label: const Text('Send'),
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
