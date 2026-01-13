import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../app.dart';
import '../../core/models/app_update.dart';
import '../../core/providers/settings_provider.dart';
import '../../core/providers/toss_provider.dart';
import '../../core/providers/update_provider.dart';

class SettingsScreen extends ConsumerWidget {
  const SettingsScreen({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final settings = ref.watch(settingsProvider);
    final tossState = ref.watch(tossProvider);
    final themeMode = ref.watch(themeModeProvider);
    final updateState = ref.watch(updateProvider);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Settings'),
      ),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          // Device section
          _SectionHeader(title: 'Device'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.badge),
                  title: const Text('Device Name'),
                  subtitle: Text(tossState.deviceName),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => _showDeviceNameDialog(context, ref, tossState.deviceName),
                ),
                const Divider(height: 1),
                ListTile(
                  leading: const Icon(Icons.fingerprint),
                  title: const Text('Device ID'),
                  subtitle: Text(
                    tossState.deviceId.isNotEmpty
                        ? '${tossState.deviceId.substring(0, 16)}...'
                        : 'Not initialized',
                  ),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Sync section
          _SectionHeader(title: 'Sync'),
          Card(
            child: Column(
              children: [
                SwitchListTile(
                  secondary: const Icon(Icons.sync),
                  title: const Text('Auto Sync'),
                  subtitle: const Text('Automatically sync clipboard'),
                  value: settings.autoSync,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateAutoSync(value);
                  },
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const Icon(Icons.text_fields),
                  title: const Text('Sync Text'),
                  value: settings.syncText,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateSyncText(value);
                  },
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const Icon(Icons.image),
                  title: const Text('Sync Images'),
                  value: settings.syncImages,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateSyncImages(value);
                  },
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const Icon(Icons.attach_file),
                  title: const Text('Sync Files'),
                  value: settings.syncFiles,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateSyncFiles(value);
                  },
                ),
                const Divider(height: 1),
                ListTile(
                  leading: const Icon(Icons.storage),
                  title: const Text('Max File Size'),
                  subtitle: Text('${settings.maxFileSizeMb} MB'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => _showMaxFileSizeDialog(context, ref, settings.maxFileSizeMb),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // History section
          _SectionHeader(title: 'History'),
          Card(
            child: Column(
              children: [
                SwitchListTile(
                  secondary: const Icon(Icons.history),
                  title: const Text('Save History'),
                  subtitle: const Text('Keep clipboard history locally'),
                  value: settings.historyEnabled,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateHistoryEnabled(value);
                  },
                ),
                const Divider(height: 1),
                ListTile(
                  leading: const Icon(Icons.calendar_today),
                  title: const Text('Keep History For'),
                  subtitle: Text('${settings.historyDays} days'),
                  trailing: const Icon(Icons.chevron_right),
                  enabled: settings.historyEnabled,
                  onTap: settings.historyEnabled
                      ? () => _showHistoryDaysDialog(context, ref, settings.historyDays)
                      : null,
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Network section
          _SectionHeader(title: 'Network'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.cloud),
                  title: const Text('Relay Server'),
                  subtitle: Text(settings.relayUrl ?? 'Not configured'),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => _showRelayUrlDialog(context, ref, settings.relayUrl),
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // Appearance section
          _SectionHeader(title: 'Appearance'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.palette),
                  title: const Text('Theme'),
                  subtitle: Text(_getThemeName(themeMode)),
                  trailing: const Icon(Icons.chevron_right),
                  onTap: () => _showThemeDialog(context, ref, themeMode),
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const Icon(Icons.notifications),
                  title: const Text('Notifications'),
                  value: settings.showNotifications,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateShowNotifications(value);
                  },
                ),
              ],
            ),
          ),
          const SizedBox(height: 24),

          // About section
          _SectionHeader(title: 'About'),
          Card(
            child: Column(
              children: [
                ListTile(
                  leading: const Icon(Icons.info),
                  title: const Text('Version'),
                  subtitle: Text(ref.read(updateProvider.notifier).currentVersion),
                ),
                const Divider(height: 1),
                // Update status (desktop only)
                if (Platform.isMacOS || Platform.isWindows || Platform.isLinux) ...[
                  _buildUpdateTile(context, ref, updateState),
                  const Divider(height: 1),
                ],
                ListTile(
                  leading: const Icon(Icons.code),
                  title: const Text('Source Code'),
                  subtitle: const Text('github.com/rennerdo30/toss-share'),
                  trailing: const Icon(Icons.open_in_new),
                  onTap: () {
                    launchUrl(Uri.parse('https://github.com/rennerdo30/toss-share'));
                  },
                ),
              ],
            ),
          ),
          const SizedBox(height: 32),
        ],
      ),
    );
  }

  String _getThemeName(ThemeMode mode) {
    switch (mode) {
      case ThemeMode.system:
        return 'System';
      case ThemeMode.light:
        return 'Light';
      case ThemeMode.dark:
        return 'Dark';
    }
  }

  Widget _buildUpdateTile(BuildContext context, WidgetRef ref, UpdateState updateState) {
    IconData icon;
    Widget? trailing;
    VoidCallback? onTap;

    switch (updateState.status) {
      case UpdateStatus.idle:
      case UpdateStatus.upToDate:
        icon = Icons.check_circle;
        trailing = TextButton(
          onPressed: () => ref.read(updateProvider.notifier).checkForUpdates(),
          child: const Text('Check'),
        );
        break;
      case UpdateStatus.checking:
        icon = Icons.sync;
        trailing = const SizedBox(
          width: 20,
          height: 20,
          child: CircularProgressIndicator(strokeWidth: 2),
        );
        break;
      case UpdateStatus.available:
        icon = Icons.download;
        break;
      case UpdateStatus.downloading:
        icon = Icons.downloading;
        trailing = SizedBox(
          width: 48,
          child: LinearProgressIndicator(value: updateState.downloadProgress),
        );
        break;
      case UpdateStatus.ready:
        icon = Icons.restart_alt;
        trailing = TextButton(
          onPressed: () => ref.read(updateProvider.notifier).applyAndRestart(),
          child: const Text('Restart'),
        );
        onTap = () => ref.read(updateProvider.notifier).applyAndRestart();
        break;
      case UpdateStatus.error:
        icon = Icons.error;
        trailing = TextButton(
          onPressed: () => ref.read(updateProvider.notifier).checkForUpdates(),
          child: const Text('Retry'),
        );
        break;
    }

    return ListTile(
      leading: Icon(icon),
      title: const Text('Updates'),
      subtitle: Text(updateState.status.displayName),
      trailing: trailing,
      onTap: onTap,
    );
  }

  void _showDeviceNameDialog(BuildContext context, WidgetRef ref, String currentName) {
    final controller = TextEditingController(text: currentName);
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Device Name'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(hintText: 'Enter device name'),
          autofocus: true,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              ref.read(tossProvider.notifier).setDeviceName(controller.text);
              Navigator.pop(context);
            },
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }

  void _showMaxFileSizeDialog(BuildContext context, WidgetRef ref, int currentSize) {
    showDialog(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Max File Size'),
        children: [10, 25, 50, 100, 200].map((size) {
          return SimpleDialogOption(
            onPressed: () {
              ref.read(settingsProvider.notifier).updateMaxFileSize(size);
              Navigator.pop(context);
            },
            child: Text(
              '$size MB',
              style: TextStyle(
                fontWeight: size == currentSize ? FontWeight.bold : FontWeight.normal,
              ),
            ),
          );
        }).toList(),
      ),
    );
  }

  void _showHistoryDaysDialog(BuildContext context, WidgetRef ref, int currentDays) {
    showDialog(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Keep History For'),
        children: [1, 3, 7, 14, 30].map((days) {
          return SimpleDialogOption(
            onPressed: () {
              ref.read(settingsProvider.notifier).updateHistoryDays(days);
              Navigator.pop(context);
            },
            child: Text(
              '$days day${days > 1 ? 's' : ''}',
              style: TextStyle(
                fontWeight: days == currentDays ? FontWeight.bold : FontWeight.normal,
              ),
            ),
          );
        }).toList(),
      ),
    );
  }

  void _showRelayUrlDialog(BuildContext context, WidgetRef ref, String? currentUrl) {
    final controller = TextEditingController(text: currentUrl);
    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Relay Server URL'),
        content: TextField(
          controller: controller,
          decoration: const InputDecoration(
            hintText: 'https://relay.example.com',
          ),
          keyboardType: TextInputType.url,
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              final url = controller.text.isEmpty ? null : controller.text;
              ref.read(settingsProvider.notifier).updateRelayUrl(url);
              Navigator.pop(context);
            },
            child: const Text('Save'),
          ),
        ],
      ),
    );
  }

  void _showThemeDialog(BuildContext context, WidgetRef ref, ThemeMode currentMode) {
    showDialog(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('Theme'),
        children: ThemeMode.values.map((mode) {
          return SimpleDialogOption(
            onPressed: () {
              ref.read(themeModeProvider.notifier).state = mode;
              Navigator.pop(context);
            },
            child: Text(
              _getThemeName(mode),
              style: TextStyle(
                fontWeight: mode == currentMode ? FontWeight.bold : FontWeight.normal,
              ),
            ),
          );
        }).toList(),
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  final String title;

  const _SectionHeader({required this.title});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(left: 4, bottom: 8),
      child: Text(
        title,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
          color: Theme.of(context).colorScheme.primary,
        ),
      ),
    );
  }
}
