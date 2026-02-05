import 'package:flutter/material.dart';

import '../../../core/models/device.dart';
import '../../../shared/utils/platform_utils.dart';
import '../../../shared/constants/layout_constants.dart';

class DeviceList extends StatelessWidget {
  final List<Device> devices;
  final Function(Device) onDeviceTap;

  const DeviceList({
    super.key,
    required this.devices,
    required this.onDeviceTap,
  });

  @override
  Widget build(BuildContext context) {
    final isLandscape =
        MediaQuery.of(context).orientation == Orientation.landscape;
    return SizedBox(
      height: isLandscape
          ? LayoutConstants.deviceListHeightLandscape
          : LayoutConstants.deviceListHeight,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        itemCount: devices.length,
        separatorBuilder: (_, __) => const SizedBox(width: 12),
        itemBuilder: (context, index) {
          final device = devices[index];
          return DeviceCard(
            device: device,
            onTap: () => onDeviceTap(device),
          );
        },
      ),
    );
  }
}

class DeviceCard extends StatelessWidget {
  final Device device;
  final VoidCallback onTap;

  const DeviceCard({
    super.key,
    required this.device,
    required this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;

    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(12),
      child: Container(
        width: 100,
        padding: const EdgeInsets.all(12),
        decoration: BoxDecoration(
          border: Border.all(
            color: device.isOnline
                ? colorScheme.primary.withValues(alpha: 0.5)
                : colorScheme.outline.withValues(alpha: 0.3),
          ),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            // Platform icon
            Stack(
              children: [
                Icon(
                  getPlatformIcon(device.platform),
                  size: LayoutConstants.largeIconSize,
                  color: device.isOnline
                      ? colorScheme.primary
                      : colorScheme.outline,
                ),
                // Online indicator
                Positioned(
                  right: 0,
                  bottom: 0,
                  child: Semantics(
                    label: device.isOnline ? 'Online' : 'Offline',
                    child: Container(
                      width: 10,
                      height: 10,
                      decoration: BoxDecoration(
                        shape: BoxShape.circle,
                        color: device.isOnline ? Colors.green : Colors.grey,
                        border: Border.all(
                          color: Theme.of(context).scaffoldBackgroundColor,
                          width: 2,
                        ),
                      ),
                    ),
                  ),
                ),
              ],
            ),
            const SizedBox(height: 8),

            // Device name
            Text(
              device.name,
              style: Theme.of(context).textTheme.bodySmall,
              textAlign: TextAlign.center,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
            ),
          ],
        ),
      ),
    );
  }

}
