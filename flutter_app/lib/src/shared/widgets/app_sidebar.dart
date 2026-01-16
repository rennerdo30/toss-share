import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';

import '../../core/providers/devices_provider.dart';
import '../../core/providers/toss_provider.dart';
import '../../core/models/device.dart';
import 'responsive_layout.dart';

/// Navigation item for the sidebar
class SidebarNavItem {
  final String route;
  final String label;
  final IconData icon;
  final IconData? selectedIcon;

  const SidebarNavItem({
    required this.route,
    required this.label,
    required this.icon,
    this.selectedIcon,
  });
}

/// App sidebar widget with devices and navigation
class AppSidebar extends ConsumerWidget {
  final String currentRoute;
  final bool isCollapsed;
  final VoidCallback? onToggleCollapse;

  const AppSidebar({
    super.key,
    required this.currentRoute,
    this.isCollapsed = false,
    this.onToggleCollapse,
  });

  static const List<SidebarNavItem> navItems = [
    SidebarNavItem(
      route: '/',
      label: 'Home',
      icon: Icons.home_outlined,
      selectedIcon: Icons.home,
    ),
    SidebarNavItem(
      route: '/history',
      label: 'History',
      icon: Icons.history_outlined,
      selectedIcon: Icons.history,
    ),
    SidebarNavItem(
      route: '/settings',
      label: 'Settings',
      icon: Icons.settings_outlined,
      selectedIcon: Icons.settings,
    ),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final theme = Theme.of(context);
    final devices = ref.watch(devicesProvider);
    final tossState = ref.watch(tossProvider);
    final onlineCount = devices.where((d) => d.isOnline).length;

    final width = isCollapsed
        ? Breakpoints.collapsedSidebarWidth
        : Breakpoints.sidebarWidth;

    return AnimatedContainer(
      duration: const Duration(milliseconds: 200),
      width: width,
      decoration: BoxDecoration(
        color: theme.colorScheme.surface,
        border: Border(
          right: BorderSide(
            color: theme.colorScheme.outlineVariant,
            width: 1,
          ),
        ),
      ),
      child: Column(
        children: [
          // Connection status header
          _ConnectionStatusHeader(
            isCollapsed: isCollapsed,
            onlineCount: onlineCount,
            totalCount: devices.length,
            isSyncing: tossState.isSyncing,
          ),

          const Divider(height: 1),

          // Devices section
          if (!isCollapsed) ...[
            _SectionHeader(
              title: 'Devices',
              trailing: IconButton(
                icon: const Icon(Icons.add, size: 18),
                tooltip: 'Add Device',
                onPressed: () => context.push('/pairing'),
                visualDensity: VisualDensity.compact,
              ),
            ),
            Expanded(
              flex: 2,
              child: devices.isEmpty
                  ? _EmptyDevicesMessage(
                      onAddDevice: () => context.push('/pairing'),
                    )
                  : _DevicesList(
                      devices: devices,
                      onDeviceTap: (device) => _showDeviceActions(context, device, ref),
                    ),
            ),
            const Divider(height: 1),
          ],

          // Navigation section
          if (!isCollapsed)
            _SectionHeader(title: 'Navigation'),

          // Nav items
          Expanded(
            flex: isCollapsed ? 1 : 0,
            child: ListView(
              shrinkWrap: !isCollapsed,
              padding: const EdgeInsets.symmetric(vertical: 8),
              children: navItems.map((item) {
                final isSelected = currentRoute == item.route;
                return _NavItemTile(
                  item: item,
                  isSelected: isSelected,
                  isCollapsed: isCollapsed,
                  onTap: () => context.go(item.route),
                );
              }).toList(),
            ),
          ),

          // Add device button at bottom (collapsed mode)
          if (isCollapsed) ...[
            const Spacer(),
            Padding(
              padding: const EdgeInsets.all(8),
              child: IconButton(
                icon: const Icon(Icons.add),
                tooltip: 'Add Device',
                onPressed: () => context.push('/pairing'),
              ),
            ),
          ],

          // Collapse toggle
          if (onToggleCollapse != null) ...[
            const Divider(height: 1),
            _CollapseToggle(
              isCollapsed: isCollapsed,
              onToggle: onToggleCollapse!,
            ),
          ],
        ],
      ),
    );
  }

  void _showDeviceActions(BuildContext context, Device device, WidgetRef ref) {
    showModalBottomSheet(
      context: context,
      builder: (context) => SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              leading: const Icon(Icons.send),
              title: const Text('Send clipboard to device'),
              enabled: device.isOnline,
              onTap: device.isOnline
                  ? () async {
                      Navigator.pop(context);
                      try {
                        await ref.read(tossProvider.notifier).sendClipboard();
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('Sent to ${device.name}')),
                          );
                        }
                      } catch (e) {
                        if (context.mounted) {
                          ScaffoldMessenger.of(context).showSnackBar(
                            SnackBar(content: Text('Failed to send: $e')),
                          );
                        }
                      }
                    }
                  : null,
            ),
            ListTile(
              leading: const Icon(Icons.edit),
              title: const Text('Edit name'),
              onTap: () {
                Navigator.pop(context);
                _showRenameDialog(context, device, ref);
              },
            ),
            ListTile(
              leading: const Icon(Icons.copy),
              title: const Text('Copy device ID'),
              onTap: () async {
                Navigator.pop(context);
                await Clipboard.setData(ClipboardData(text: device.id));
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Device ID copied')),
                  );
                }
              },
            ),
            SwitchListTile(
              secondary: const Icon(Icons.sync),
              title: const Text('Sync enabled'),
              subtitle: Text(device.syncEnabled ? 'Receiving from this device' : 'Ignoring this device'),
              value: device.syncEnabled,
              onChanged: (value) {
                ref.read(devicesProvider.notifier).toggleDeviceSync(device.id, value);
                Navigator.pop(context);
                ScaffoldMessenger.of(context).showSnackBar(
                  SnackBar(
                    content: Text(value
                        ? 'Sync enabled for ${device.name}'
                        : 'Sync disabled for ${device.name}'),
                  ),
                );
              },
            ),
            ListTile(
              leading: Icon(Icons.delete, color: Theme.of(context).colorScheme.error),
              title: Text('Remove device',
                style: TextStyle(color: Theme.of(context).colorScheme.error)),
              onTap: () async {
                Navigator.pop(context);
                await ref.read(devicesProvider.notifier).removeDevice(device.id);
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(content: Text('Device removed')),
                  );
                }
              },
            ),
          ],
        ),
      ),
    );
  }

  void _showRenameDialog(BuildContext context, Device device, WidgetRef ref) {
    final controller = TextEditingController(text: device.name);
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Rename Device'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            labelText: 'Device name',
            border: OutlineInputBorder(),
          ),
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          FilledButton(
            onPressed: () async {
              final newName = controller.text.trim();
              if (newName.isNotEmpty && newName != device.name) {
                await ref.read(devicesProvider.notifier).renameDevice(device.id, newName);
              }
              if (context.mounted) {
                Navigator.pop(context);
              }
            },
            child: const Text('Rename'),
          ),
        ],
      ),
    );
  }
}

class _ConnectionStatusHeader extends StatelessWidget {
  final bool isCollapsed;
  final int onlineCount;
  final int totalCount;
  final bool isSyncing;

  const _ConnectionStatusHeader({
    required this.isCollapsed,
    required this.onlineCount,
    required this.totalCount,
    required this.isSyncing,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isConnected = onlineCount > 0;
    final statusColor = isConnected ? Colors.green : theme.colorScheme.outline;

    if (isCollapsed) {
      return Padding(
        padding: const EdgeInsets.symmetric(vertical: 12),
        child: Tooltip(
          message: isConnected
              ? '$onlineCount device${onlineCount != 1 ? 's' : ''} online'
              : 'No devices online',
          child: Container(
            width: 12,
            height: 12,
            decoration: BoxDecoration(
              color: statusColor,
              shape: BoxShape.circle,
            ),
          ),
        ),
      );
    }

    return Padding(
      padding: const EdgeInsets.all(16),
      child: Row(
        children: [
          Container(
            width: 10,
            height: 10,
            decoration: BoxDecoration(
              color: statusColor,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  isConnected ? 'Connected' : 'Offline',
                  style: theme.textTheme.labelLarge?.copyWith(
                    fontWeight: FontWeight.w600,
                  ),
                ),
                Text(
                  '$onlineCount of $totalCount device${totalCount != 1 ? 's' : ''} online',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.outline,
                  ),
                ),
              ],
            ),
          ),
          if (isSyncing)
            const SizedBox(
              width: 16,
              height: 16,
              child: CircularProgressIndicator(strokeWidth: 2),
            ),
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;
  final Widget? trailing;

  const _SectionHeader({
    required this.title,
    this.trailing,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.fromLTRB(16, 12, 8, 4),
      child: Row(
        children: [
          Text(
            title.toUpperCase(),
            style: theme.textTheme.labelSmall?.copyWith(
              color: theme.colorScheme.outline,
              letterSpacing: 1.2,
              fontWeight: FontWeight.w600,
            ),
          ),
          const Spacer(),
          if (trailing != null) trailing!,
        ],
      ),
    );
  }
}

class _DevicesList extends StatelessWidget {
  final List<Device> devices;
  final void Function(Device) onDeviceTap;

  const _DevicesList({
    required this.devices,
    required this.onDeviceTap,
  });

  @override
  Widget build(BuildContext context) {
    return ListView.builder(
      padding: const EdgeInsets.symmetric(vertical: 4),
      itemCount: devices.length,
      itemBuilder: (context, index) {
        final device = devices[index];
        return _DeviceListTile(
          device: device,
          onTap: () => onDeviceTap(device),
        );
      },
    );
  }
}

class _DeviceListTile extends StatelessWidget {
  final Device device;
  final VoidCallback onTap;

  const _DeviceListTile({
    required this.device,
    required this.onTap,
  });

  IconData _getPlatformIcon() {
    switch (device.platform) {
      case DevicePlatform.macos:
        return Icons.laptop_mac;
      case DevicePlatform.windows:
        return Icons.laptop_windows;
      case DevicePlatform.linux:
        return Icons.computer;
      case DevicePlatform.ios:
        return Icons.phone_iphone;
      case DevicePlatform.android:
        return Icons.phone_android;
      default:
        return Icons.devices;
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListTile(
      dense: true,
      leading: Stack(
        children: [
          Icon(
            _getPlatformIcon(),
            size: 20,
            color: theme.colorScheme.onSurfaceVariant,
          ),
          if (device.isOnline)
            Positioned(
              right: 0,
              bottom: 0,
              child: Container(
                width: 8,
                height: 8,
                decoration: BoxDecoration(
                  color: Colors.green,
                  shape: BoxShape.circle,
                  border: Border.all(
                    color: theme.colorScheme.surface,
                    width: 1.5,
                  ),
                ),
              ),
            ),
        ],
      ),
      title: Text(
        device.name,
        style: theme.textTheme.bodyMedium,
        overflow: TextOverflow.ellipsis,
      ),
      onTap: onTap,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16),
    );
  }
}

class _EmptyDevicesMessage extends StatelessWidget {
  final VoidCallback onAddDevice;

  const _EmptyDevicesMessage({required this.onAddDevice});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              Icons.devices_other,
              size: 32,
              color: theme.colorScheme.outline,
            ),
            const SizedBox(height: 8),
            Text(
              'No devices',
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.outline,
              ),
            ),
            const SizedBox(height: 8),
            TextButton.icon(
              onPressed: onAddDevice,
              icon: const Icon(Icons.add, size: 16),
              label: const Text('Add'),
            ),
          ],
        ),
      ),
    );
  }
}

class _NavItemTile extends StatelessWidget {
  final SidebarNavItem item;
  final bool isSelected;
  final bool isCollapsed;
  final VoidCallback onTap;

  const _NavItemTile({
    required this.item,
    required this.isSelected,
    required this.isCollapsed,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final icon = isSelected ? (item.selectedIcon ?? item.icon) : item.icon;

    if (isCollapsed) {
      return Tooltip(
        message: item.label,
        child: InkWell(
          onTap: onTap,
          child: Container(
            padding: const EdgeInsets.symmetric(vertical: 12),
            decoration: isSelected
                ? BoxDecoration(
                    color: theme.colorScheme.primaryContainer,
                    borderRadius: BorderRadius.circular(8),
                  )
                : null,
            child: Icon(
              icon,
              color: isSelected
                  ? theme.colorScheme.primary
                  : theme.colorScheme.onSurfaceVariant,
            ),
          ),
        ),
      );
    }

    return ListTile(
      dense: true,
      leading: Icon(
        icon,
        size: 20,
        color: isSelected
            ? theme.colorScheme.primary
            : theme.colorScheme.onSurfaceVariant,
      ),
      title: Text(
        item.label,
        style: theme.textTheme.bodyMedium?.copyWith(
          color: isSelected
              ? theme.colorScheme.primary
              : theme.colorScheme.onSurface,
          fontWeight: isSelected ? FontWeight.w600 : FontWeight.normal,
        ),
      ),
      selected: isSelected,
      selectedTileColor: theme.colorScheme.primaryContainer,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
      ),
      onTap: onTap,
      contentPadding: const EdgeInsets.symmetric(horizontal: 16),
    );
  }
}

class _CollapseToggle extends StatelessWidget {
  final bool isCollapsed;
  final VoidCallback onToggle;

  const _CollapseToggle({
    required this.isCollapsed,
    required this.onToggle,
  });

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onToggle,
      child: Padding(
        padding: const EdgeInsets.all(12),
        child: Row(
          mainAxisAlignment: isCollapsed
              ? MainAxisAlignment.center
              : MainAxisAlignment.end,
          children: [
            Icon(
              isCollapsed
                  ? Icons.chevron_right
                  : Icons.chevron_left,
              size: 20,
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
            if (!isCollapsed) ...[
              const SizedBox(width: 4),
              Text(
                'Collapse',
                style: Theme.of(context).textTheme.labelSmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
