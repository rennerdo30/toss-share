import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../app.dart';
import '../../core/models/app_update.dart';
import '../../core/providers/settings_provider.dart';
import '../../core/providers/toss_provider.dart';
import '../../core/providers/update_provider.dart';
import '../../core/services/auto_start_service.dart';
import '../../shared/widgets/responsive_layout.dart';

class SettingsScreen extends ConsumerStatefulWidget {
  const SettingsScreen({super.key});

  @override
  ConsumerState<SettingsScreen> createState() => _SettingsScreenState();
}

class _SettingsScreenState extends ConsumerState<SettingsScreen> {
  Future<bool> _getAutoStartStatus() async {
    return await AutoStartService.isEnabled();
  }

  Future<void> _toggleAutoStart(BuildContext context, bool enable) async {
    final success = enable
        ? await AutoStartService.enable()
        : await AutoStartService.disable();

    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(
            success
                ? 'Auto-start ${enable ? "enabled" : "disabled"}'
                : 'Failed to ${enable ? "enable" : "disable"} auto-start',
          ),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final settings = ref.watch(settingsProvider);
    final tossState = ref.watch(tossProvider);
    final themeMode = ref.watch(themeModeProvider);
    final updateState = ref.watch(updateProvider);

    return Scaffold(
      body: ResponsiveBuilder(
        builder: (context, isMobile, isTablet, isDesktop) {
          if (isMobile) {
            // Mobile: single column list
            return _buildMobileLayout(
              context, settings, tossState, themeMode, updateState);
          }
          // Desktop/Tablet: two column grid
          return _buildDesktopLayout(
            context, settings, tossState, themeMode, updateState);
        },
      ),
    );
  }

  Widget _buildMobileLayout(
    BuildContext context,
    AppSettings settings,
    TossState tossState,
    ThemeMode themeMode,
    UpdateState updateState,
  ) {
    return ListView(
      padding: const EdgeInsets.all(16),
      children: _buildAllSections(context, settings, tossState, themeMode, updateState),
    );
  }

  Widget _buildDesktopLayout(
    BuildContext context,
    AppSettings settings,
    TossState tossState,
    ThemeMode themeMode,
    UpdateState updateState,
  ) {
    final theme = Theme.of(context);

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Header
          Text(
            'Settings',
            style: theme.textTheme.headlineSmall?.copyWith(
              fontWeight: FontWeight.bold,
            ),
          ),
          const SizedBox(height: 24),

          // Two-column grid
          LayoutBuilder(
            builder: (context, constraints) {
              final columnWidth = (constraints.maxWidth - 24) / 2;

              return Wrap(
                spacing: 24,
                runSpacing: 24,
                children: [
                  // Column 1
                  SizedBox(
                    width: columnWidth,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        _buildDeviceSection(tossState),
                        const SizedBox(height: 24),
                        _buildSyncSection(settings),
                        const SizedBox(height: 24),
                        _buildHistorySection(settings),
                      ],
                    ),
                  ),
                  // Column 2
                  SizedBox(
                    width: columnWidth,
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: [
                        _buildNetworkSection(settings),
                        if (Platform.isWindows) ...[
                          const SizedBox(height: 24),
                          _buildSystemSection(),
                        ],
                        const SizedBox(height: 24),
                        _buildAppearanceSection(settings, themeMode),
                        const SizedBox(height: 24),
                        _buildAboutSection(context, updateState),
                      ],
                    ),
                  ),
                ],
              );
            },
          ),
        ],
      ),
    );
  }

  List<Widget> _buildAllSections(
    BuildContext context,
    AppSettings settings,
    TossState tossState,
    ThemeMode themeMode,
    UpdateState updateState,
  ) {
    return [
      _buildDeviceSection(tossState),
      const SizedBox(height: 24),
      _buildSyncSection(settings),
      const SizedBox(height: 24),
      _buildHistorySection(settings),
      const SizedBox(height: 24),
      _buildNetworkSection(settings),
      if (Platform.isWindows) ...[
        const SizedBox(height: 24),
        _buildSystemSection(),
      ],
      const SizedBox(height: 24),
      _buildAppearanceSection(settings, themeMode),
      const SizedBox(height: 24),
      _buildAboutSection(context, updateState),
      const SizedBox(height: 32),
    ];
  }

  Widget _buildDeviceSection(TossState tossState) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'Device'),
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
                  tossState.deviceId.isEmpty
                      ? 'Not initialized'
                      : tossState.deviceId.length > 16
                          ? '${tossState.deviceId.substring(0, 16)}...'
                          : tossState.deviceId,
                ),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildSyncSection(AppSettings settings) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'Sync'),
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
                secondary: const Icon(Icons.format_paint),
                title: const Text('Sync Rich Text'),
                subtitle: const Text('HTML and RTF formatting'),
                value: settings.syncRichText,
                onChanged: (value) {
                  ref.read(settingsProvider.notifier).updateSyncRichText(value);
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
      ],
    );
  }

  Widget _buildHistorySection(AppSettings settings) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'History'),
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
      ],
    );
  }

  Widget _buildNetworkSection(AppSettings settings) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'Network'),
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
      ],
    );
  }

  Widget _buildSystemSection() {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'System'),
        Card(
          child: FutureBuilder<bool>(
            future: _getAutoStartStatus(),
            builder: (context, snapshot) {
              final autoStartEnabled = snapshot.data ?? false;
              return SwitchListTile(
                secondary: const Icon(Icons.settings_power),
                title: const Text('Start with Windows'),
                subtitle: const Text('Launch Toss when Windows starts'),
                value: autoStartEnabled,
                onChanged: (value) async {
                  await _toggleAutoStart(context, value);
                },
              );
            },
          ),
        ),
      ],
    );
  }

  Widget _buildAppearanceSection(AppSettings settings, ThemeMode themeMode) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'Appearance'),
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
              // Granular notification settings (only shown when notifications enabled)
              if (settings.showNotifications) ...[
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const SizedBox(width: 24),
                  title: const Text('Pairing requests'),
                  subtitle: const Text('When a device wants to pair'),
                  value: settings.notifyOnPairing,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateNotifyOnPairing(value);
                  },
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const SizedBox(width: 24),
                  title: const Text('Clipboard received'),
                  subtitle: const Text('When clipboard is synced from another device'),
                  value: settings.notifyOnClipboard,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateNotifyOnClipboard(value);
                  },
                ),
                const Divider(height: 1),
                SwitchListTile(
                  secondary: const SizedBox(width: 24),
                  title: const Text('Connection status'),
                  subtitle: const Text('When devices connect or disconnect'),
                  value: settings.notifyOnConnection,
                  onChanged: (value) {
                    ref.read(settingsProvider.notifier).updateNotifyOnConnection(value);
                  },
                ),
              ],
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildAboutSection(BuildContext context, UpdateState updateState) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const _SectionHeader(title: 'About'),
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
                onTap: () async {
                  final uri = Uri.parse('https://github.com/rennerdo30/toss-share');
                  try {
                    if (await canLaunchUrl(uri)) {
                      await launchUrl(uri, mode: LaunchMode.externalApplication);
                    } else {
                      if (context.mounted) {
                        ScaffoldMessenger.of(context).showSnackBar(
                          const SnackBar(content: Text('Could not open URL')),
                        );
                      }
                    }
                  } catch (e) {
                    if (context.mounted) {
                      ScaffoldMessenger.of(context).showSnackBar(
                        SnackBar(content: Text('Error opening URL: $e')),
                      );
                    }
                  }
                },
              ),
            ],
          ),
        ),
      ],
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
    final formKey = GlobalKey<FormState>();

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Device Name'),
        content: Form(
          key: formKey,
          child: TextFormField(
            controller: controller,
            decoration: const InputDecoration(
              hintText: 'Enter device name',
              helperText: 'This name identifies your device to others',
            ),
            autofocus: true,
            maxLength: 32,
            validator: (value) {
              if (value == null || value.trim().isEmpty) {
                return 'Device name cannot be empty';
              }
              if (value.trim().length < 2) {
                return 'Device name must be at least 2 characters';
              }
              return null;
            },
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              if (formKey.currentState?.validate() ?? false) {
                ref.read(tossProvider.notifier).setDeviceName(controller.text.trim());
                Navigator.pop(context);
              }
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
    final formKey = GlobalKey<FormState>();

    showDialog(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('Relay Server URL'),
        content: Form(
          key: formKey,
          child: TextFormField(
            controller: controller,
            decoration: const InputDecoration(
              hintText: 'https://relay.example.com',
              helperText: 'Leave empty to use local network only',
            ),
            keyboardType: TextInputType.url,
            validator: (value) {
              if (value == null || value.isEmpty) {
                return null; // Empty is allowed
              }
              // Basic URL validation
              final uri = Uri.tryParse(value);
              if (uri == null || !uri.hasScheme || !uri.hasAuthority) {
                return 'Please enter a valid URL';
              }
              if (uri.scheme != 'http' && uri.scheme != 'https') {
                return 'URL must start with http:// or https://';
              }
              return null;
            },
          ),
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context),
            child: const Text('Cancel'),
          ),
          TextButton(
            onPressed: () {
              if (formKey.currentState?.validate() ?? false) {
                final url = controller.text.isEmpty ? null : controller.text;
                ref.read(settingsProvider.notifier).updateRelayUrl(url);
                Navigator.pop(context);
              }
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
