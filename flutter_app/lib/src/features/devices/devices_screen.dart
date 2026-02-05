import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../core/providers/devices_provider.dart';
import '../../core/models/device.dart';
import '../../shared/widgets/empty_state.dart';
import '../../shared/utils/platform_utils.dart';
import '../../shared/utils/timestamp_utils.dart';

class DevicesScreen extends ConsumerStatefulWidget {
  const DevicesScreen({super.key});

  @override
  ConsumerState<DevicesScreen> createState() => _DevicesScreenState();
}

class _DevicesScreenState extends ConsumerState<DevicesScreen> {
  @override
  void initState() {
    super.initState();
    // Load devices when screen is first shown
    WidgetsBinding.instance.addPostFrameCallback((_) {
      ref.read(devicesProvider.notifier).refresh();
    });
  }

  @override
  Widget build(BuildContext context) {
    final devices = ref.watch(devicesProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Devices'),
      ),
      body: devices.isEmpty
          ? _EmptyState()
          : ListView.builder(
              padding: const EdgeInsets.all(16),
              itemCount: devices.length,
              itemBuilder: (context, index) {
                final device = devices[index];
                return _DeviceListItem(
                  device: device,
                  onRemove: () {
                    _showRemoveDialog(context, ref, device);
                  },
                  onRename: () {
                    _showRenameDialog(context, ref, device);
                  },
                );
              },
            ),
    );
  }

  void _showRenameDialog(BuildContext context, WidgetRef ref, Device device) {
    final controller = TextEditingController(text: device.name);
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Rename Device'),
        content: TextField(
          controller: controller,
          autofocus: true,
          decoration: const InputDecoration(
            labelText: 'Device Name',
            hintText: 'Enter device name',
          ),
          maxLength: 100,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () async {
              final newName = controller.text.trim();
              if (newName.isEmpty) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                        content: Text('Device name cannot be empty')),
                  );
                }
                return;
              }
              try {
                await ref
                    .read(devicesProvider.notifier)
                    .renameDevice(device.id, newName);
                if (context.mounted) {
                  Navigator.pop(context);
                  ScaffoldMessenger.of(context).showSnackBar(
                    const SnackBar(
                        content: Text('Device renamed successfully')),
                  );
                }
              } catch (e) {
                if (context.mounted) {
                  ScaffoldMessenger.of(context).showSnackBar(
                    SnackBar(content: Text('Failed to rename device: $e')),
                  );
                }
              }
            },
            child: const Text('Rename'),
          ),
        ],
      ),
    );
  }

  void _showRemoveDialog(BuildContext context, WidgetRef ref, Device device) {
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Remove Device'),
        content: Text('Remove "${device.name}" from paired devices?'),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () async {
              await ref.read(devicesProvider.notifier).removeDevice(device.id);
              if (context.mounted) {
                Navigator.pop(context);
              }
            },
            child: const Text('Remove'),
          ),
        ],
      ),
    );
  }
}

class _DeviceListItem extends ConsumerWidget {
  final Device device;
  final VoidCallback onRemove;
  final VoidCallback onRename;

  const _DeviceListItem({
    required this.device,
    required this.onRemove,
    required this.onRename,
  });

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Card(
      child: ListTile(
        leading: Stack(
          children: [
            CircleAvatar(
              child: Icon(getPlatformIcon(device.platform)),
            ),
            Positioned(
              right: 0,
              bottom: 0,
              child: Semantics(
                label: device.isOnline ? 'Online' : 'Offline',
                child: Container(
                  width: 12,
                  height: 12,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: device.isOnline ? Colors.green : Colors.grey,
                    border: Border.all(
                      color: Theme.of(context).cardColor,
                      width: 2,
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
        title: Text(device.name),
        subtitle: Text(
          device.isOnline
              ? 'Online'
              : device.lastSeen != null
                  ? 'Last seen ${formatLastSeen(device.lastSeen!)}'
                  : 'Offline',
        ),
        trailing: PopupMenuButton(
          itemBuilder: (context) => [
            const PopupMenuItem(
              value: 'rename',
              child: ListTile(
                leading: Icon(Icons.edit),
                title: Text('Rename'),
                contentPadding: EdgeInsets.zero,
              ),
            ),
            const PopupMenuItem(
              value: 'remove',
              child: ListTile(
                leading: Icon(Icons.delete, color: Colors.red),
                title: Text('Remove', style: TextStyle(color: Colors.red)),
                contentPadding: EdgeInsets.zero,
              ),
            ),
          ],
          onSelected: (value) {
            if (value == 'rename') {
              onRename();
            } else if (value == 'remove') {
              onRemove();
            }
          },
        ),
      ),
    );
  }

}

class _EmptyState extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return const EmptyState(
      icon: Icons.devices,
      title: 'No devices paired',
      subtitle: 'Go to the home screen to pair a device',
    );
  }
}
