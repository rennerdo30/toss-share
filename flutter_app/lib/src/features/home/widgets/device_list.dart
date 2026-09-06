import 'package:flutter/material.dart';

import '../../../core/models/device.dart';
import '../../../shared/utils/platform_utils.dart';
import '../../../shared/utils/timestamp_utils.dart';
import '../../../shared/constants/layout_constants.dart';
import '../../../shared/theme/app_theme.dart';

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
        separatorBuilder: (_, __) =>
            const SizedBox(width: LayoutConstants.gutter),
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
    final statusColors = AppStatusColors.of(context);
    final borderRadius =
        BorderRadius.circular(LayoutConstants.defaultBorderRadius);

    return InkWell(
      onTap: onTap,
      borderRadius: borderRadius,
      child: Container(
        width: LayoutConstants.deviceCardWidth,
        padding: const EdgeInsets.all(LayoutConstants.smallPadding + 4),
        decoration: BoxDecoration(
          color: device.isOnline
              ? colorScheme.primary.withValues(alpha: 0.04)
              : null,
          border: Border.all(
            color: device.isOnline
                ? colorScheme.primary.withValues(alpha: 0.5)
                : colorScheme.outlineVariant,
          ),
          borderRadius: borderRadius,
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
                        color: device.isOnline
                            ? statusColors.online
                            : statusColors.offline,
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
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),

            // Last seen for offline devices
            if (!device.isOnline && device.lastSeen != null)
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Text(
                  formatLastSeen(device.lastSeen!),
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                        color: colorScheme.outline,
                        fontSize: 10,
                      ),
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                ),
              ),
          ],
        ),
      ),
    );
  }
}
