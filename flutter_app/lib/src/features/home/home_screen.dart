import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:window_manager/window_manager.dart';

import '../../core/providers/toss_provider.dart';
import '../../core/providers/devices_provider.dart';
import '../../core/providers/clipboard_provider.dart';
import '../../core/providers/settings_provider.dart';
import '../../core/services/toss_service.dart';
import '../../core/models/device.dart';
import '../../core/models/clipboard_item.dart';
import '../../shared/widgets/responsive_layout.dart';
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

    return ResponsiveBuilder(
      builder: (context, isMobile, isTablet, isDesktop) {
        // On tablet/desktop, the DesktopShell handles title bar and devices are in sidebar
        // On mobile, we show the full mobile layout
        if (isMobile) {
          return _MobileLayout(
            tossState: tossState,
            devices: devices,
            currentClipboard: currentClipboard,
            ref: ref,
          );
        }

        return _DesktopLayout(
          tossState: tossState,
          devices: devices,
          currentClipboard: currentClipboard,
          ref: ref,
        );
      },
    );
  }
}

/// Mobile layout - full screen with app bar, devices, and clipboard
class _MobileLayout extends StatelessWidget {
  final TossState tossState;
  final List<Device> devices;
  final ClipboardItem? currentClipboard;
  final WidgetRef ref;

  const _MobileLayout({
    required this.tossState,
    required this.devices,
    required this.currentClipboard,
    required this.ref,
  });

  @override
  Widget build(BuildContext context) {
    final isDesktop = Platform.isWindows || Platform.isLinux || Platform.isMacOS;

    return Scaffold(
      appBar: AppBar(
        toolbarHeight: isDesktop ? 46 : null,
        title: GestureDetector(
          behavior: HitTestBehavior.translucent,
          onPanStart: isDesktop ? (_) => windowManager.startDragging() : null,
          child: const Text('Toss'),
        ),
        flexibleSpace: isDesktop
            ? GestureDetector(
                behavior: HitTestBehavior.translucent,
                onPanStart: (_) => windowManager.startDragging(),
                child: Container(),
              )
            : null,
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
              relayConfigured: ref.watch(settingsProvider).relayUrl != null,
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
                          await ref.read(currentClipboardProvider.notifier).refresh();
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
      floatingActionButton: _SendButton(
        devices: devices,
        tossState: tossState,
        ref: ref,
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
            _DetailRow(
              label: 'Device ID',
              value: device.id.length > 16
                  ? '${device.id.substring(0, 16)}...'
                  : device.id,
            ),
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

/// Desktop layout - clipboard focused with quick actions
class _DesktopLayout extends StatelessWidget {
  final TossState tossState;
  final List<Device> devices;
  final ClipboardItem? currentClipboard;
  final WidgetRef ref;

  const _DesktopLayout({
    required this.tossState,
    required this.devices,
    required this.currentClipboard,
    required this.ref,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        'Clipboard',
                        style: theme.textTheme.headlineSmall?.copyWith(
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                      const SizedBox(height: 4),
                      Text(
                        currentClipboard != null
                            ? 'Last updated ${_formatTimestamp(currentClipboard!.timestamp)}'
                            : 'No clipboard content',
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: theme.colorScheme.outline,
                        ),
                      ),
                    ],
                  ),
                ),
                // Quick actions
                _QuickActionButton(
                  icon: Icons.refresh,
                  tooltip: 'Refresh',
                  onPressed: () async {
                    await ref.read(currentClipboardProvider.notifier).refresh();
                  },
                ),
                const SizedBox(width: 8),
                _QuickActionButton(
                  icon: Icons.content_copy,
                  tooltip: 'Copy to clipboard',
                  onPressed: currentClipboard == null
                      ? null
                      : () async {
                          final clipboard = currentClipboard;
                          if (clipboard == null) return;
                          try {
                            // Copy the preview text to system clipboard
                            await Clipboard.setData(
                              ClipboardData(text: clipboard.preview),
                            );
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(content: Text('Copied to clipboard')),
                              );
                            }
                          } catch (e) {
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(content: Text('Failed to copy: $e')),
                              );
                            }
                          }
                        },
                ),
                const SizedBox(width: 8),
                _QuickActionButton(
                  icon: Icons.clear,
                  tooltip: 'Clear',
                  onPressed: currentClipboard == null
                      ? null
                      : () async {
                          try {
                            // Clear the system clipboard
                            await Clipboard.setData(const ClipboardData(text: ''));
                            // Refresh to update the UI
                            await ref.read(currentClipboardProvider.notifier).refresh();
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(content: Text('Clipboard cleared')),
                              );
                            }
                          } catch (e) {
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(content: Text('Failed to clear: $e')),
                              );
                            }
                          }
                        },
                ),
              ],
            ),

            const SizedBox(height: 24),

            // Clipboard preview (expanded)
            Expanded(
              child: ClipboardPreviewCard(
                item: currentClipboard,
                onRefresh: () async {
                  await ref.read(currentClipboardProvider.notifier).refresh();
                },
              ),
            ),

            const SizedBox(height: 16),

            // Send button row
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                if (devices.isEmpty)
                  Text(
                    'No devices paired',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.outline,
                    ),
                  )
                else
                  Text(
                    '${devices.where((d) => d.isOnline).length} of ${devices.length} devices online',
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: theme.colorScheme.outline,
                    ),
                  ),
                const SizedBox(width: 16),
                FilledButton.icon(
                  onPressed: devices.isEmpty || tossState.isSyncing
                      ? null
                      : () async {
                          try {
                            await ref.read(tossProvider.notifier).sendClipboard();
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(content: Text('Clipboard sent successfully!')),
                              );
                            }
                          } catch (e) {
                            if (context.mounted) {
                              ScaffoldMessenger.of(context).showSnackBar(
                                SnackBar(content: Text('Failed to send: $e')),
                              );
                            }
                          }
                        },
                  icon: tossState.isSyncing
                      ? const SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(
                            strokeWidth: 2,
                            color: Colors.white,
                          ),
                        )
                      : const Icon(Icons.send),
                  label: Text(tossState.isSyncing ? 'Sending...' : 'Send to all devices'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  String _formatTimestamp(DateTime timestamp) {
    final now = DateTime.now();
    final diff = now.difference(timestamp);

    if (diff.inSeconds < 60) return 'just now';
    if (diff.inMinutes < 60) return '${diff.inMinutes}m ago';
    if (diff.inHours < 24) return '${diff.inHours}h ago';
    return '${timestamp.month}/${timestamp.day} at ${timestamp.hour}:${timestamp.minute.toString().padLeft(2, '0')}';
  }
}

/// Quick action button for desktop header
class _QuickActionButton extends StatelessWidget {
  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;

  const _QuickActionButton({
    required this.icon,
    required this.tooltip,
    this.onPressed,
  });

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: IconButton.outlined(
        onPressed: onPressed,
        icon: Icon(icon, size: 20),
      ),
    );
  }
}

/// Send button for mobile layout
class _SendButton extends StatelessWidget {
  final List<Device> devices;
  final TossState tossState;
  final WidgetRef ref;

  const _SendButton({
    required this.devices,
    required this.tossState,
    required this.ref,
  });

  @override
  Widget build(BuildContext context) {
    return FloatingActionButton.extended(
      onPressed: devices.isEmpty || tossState.isSyncing
          ? null
          : () async {
              try {
                await ref.read(tossProvider.notifier).sendClipboard();
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
      icon: tossState.isSyncing
          ? const SizedBox(
              width: 18,
              height: 18,
              child: CircularProgressIndicator(
                strokeWidth: 2,
                color: Colors.white,
              ),
            )
          : const Icon(Icons.send),
      label: Text(tossState.isSyncing ? 'Sending...' : 'Send'),
    );
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
