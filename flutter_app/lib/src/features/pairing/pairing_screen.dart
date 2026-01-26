import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:permission_handler/permission_handler.dart';

import '../../core/providers/settings_provider.dart';
import '../../core/services/logging_service.dart';
import '../../core/services/toss_service.dart';

class PairingScreen extends ConsumerStatefulWidget {
  const PairingScreen({super.key});

  @override
  ConsumerState<PairingScreen> createState() => _PairingScreenState();
}

class _PairingScreenState extends ConsumerState<PairingScreen>
    with SingleTickerProviderStateMixin {
  late TabController _tabController;
  String? _pairingCode;
  String? _qrData;
  bool _isLoading = true;
  bool _isPairing = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
    _startPairing();
  }

  @override
  void dispose() {
    _tabController.dispose();
    super.dispose();
  }

  Future<void> _startPairing() async {
    LoggingService.info('Starting pairing session...');
    setState(() {
      _isLoading = true;
      _error = null;
    });

    try {
      // Check relay configuration and show warning if not configured
      final settings = ref.read(settingsProvider);
      if (settings.relayUrl == null && mounted) {
        // Show warning that pairing only works on same network without relay
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) {
            ScaffoldMessenger.of(context).showSnackBar(
              SnackBar(
                content: const Text(
                  'No relay server configured. Pairing only works on the same Wi-Fi network.',
                ),
                action: SnackBarAction(
                  label: 'Settings',
                  onPressed: () => context.push('/settings'),
                ),
                duration: const Duration(seconds: 8),
                backgroundColor: Colors.orange,
              ),
            );
          }
        });
      }

      final pairingInfo = await TossService.startPairing();
      LoggingService.info('Pairing session started with code: ${pairingInfo.code}');
      if (mounted) {
        setState(() {
          _pairingCode = pairingInfo.code;
          _qrData = pairingInfo.qrData;
          _isLoading = false;
        });
      }

      // Register pairing advertisement on relay server and mDNS
      final advResult = await TossService.registerPairingAdvertisement();

      if (mounted) {
        // Show warning if device is not discoverable at all
        if (advResult.totalFailure) {
          setState(() {
            _error = 'Failed to make device discoverable. Check your network connection.';
          });
          ScaffoldMessenger.of(context).showSnackBar(
            SnackBar(
              content: Text(
                'Failed to register for discovery: ${advResult.mdnsError ?? advResult.relayError}',
              ),
              backgroundColor: Colors.red,
              duration: const Duration(seconds: 5),
            ),
          );
        } else if (!advResult.mdnsRegistered && advResult.relayRegistered) {
          // mDNS failed but relay succeeded - local network discovery won't work
          LoggingService.warn('mDNS registration failed, only relay will work: ${advResult.mdnsError}');
        }
      }
    } catch (e) {
      LoggingService.error('Failed to start pairing', e);
      if (mounted) {
        setState(() {
          _error = 'Failed to start pairing: $e';
          _isLoading = false;
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Pair Device'),
        bottom: TabBar(
          controller: _tabController,
          tabs: const [
            Tab(text: 'Show Code'),
            Tab(text: 'Scan Code'),
          ],
        ),
      ),
      body: TabBarView(
        controller: _tabController,
        children: [
          // Tab 1: Display QR code
          _ShowCodeTab(
            isLoading: _isLoading,
            pairingCode: _pairingCode,
            qrData: _qrData,
            error: _error,
            onRegenerate: _startPairing,
          ),

          // Tab 2: Scan QR code
          _ScanCodeTab(
            onScanned: _handleQrScanned,
            onManualCode: _handleManualCode,
            isPairing: _isPairing,
          ),
        ],
      ),
    );
  }

  Future<void> _handleQrScanned(String data) async {
    if (_isPairing) return; // Prevent multiple simultaneous pairing attempts

    setState(() {
      _isPairing = true;
      _error = null;
    });

    try {
      final device = await TossService.completePairingQR(data);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Device "${device.name}" paired successfully!'),
            backgroundColor: Colors.green,
          ),
        );
        context.pop();
      }
    } catch (e) {
      if (mounted) {
        setState(() {
          _isPairing = false;
          _error = 'Pairing failed: $e';
        });
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Pairing failed: $e'),
            backgroundColor: Colors.red,
          ),
        );
      }
    }
  }

  Future<void> _handleManualCode(String code) async {
    if (_isPairing) return;

    LoggingService.info('Starting manual pairing with code: ${code.substring(0, 2)}***');
    setState(() {
      _isPairing = true;
      _error = null;
    });

    try {
      // Search for device by pairing code via mDNS and relay server
      LoggingService.debug('Searching for device...');
      final device = await TossService.findPairingDevice(code);

      // Complete the pairing
      LoggingService.info('Device found, completing pairing with: ${device.deviceName}');
      final pairedDevice = await TossService.completeManualPairing(device);

      if (mounted) {
        final viaText = device.viaRelay ? ' (via relay)' : ' (local network)';
        LoggingService.info('Pairing completed successfully${viaText}');
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('Device "${pairedDevice.name}" paired successfully$viaText!'),
            backgroundColor: Colors.green,
          ),
        );
        context.pop();
      }
    } catch (e) {
      LoggingService.error('Pairing failed', e);
      if (mounted) {
        setState(() {
          _isPairing = false;
        });

        // Extract meaningful error message
        String errorMsg = e.toString();
        // Remove common prefixes
        errorMsg = errorMsg.replaceAll('Exception: ', '');
        errorMsg = errorMsg.replaceAll('Failed to find device: ', '');

        // Check for specific error patterns and provide helpful messages
        if (errorMsg.contains('not found on local network') ||
            errorMsg.contains('same Wi-Fi')) {
          // Already a helpful message from Rust, use it as-is
        } else if (errorMsg.contains('Pairing code not found or expired')) {
          errorMsg =
              'Device not found. Make sure the other device is showing the pairing code.';
        } else if (errorMsg.contains('relay') && errorMsg.contains('contact')) {
          errorMsg =
              'Could not reach relay server. Check your internet connection.';
        } else if (!errorMsg.contains('network') && !errorMsg.contains('relay')) {
          // Generic fallback for unknown errors
          errorMsg =
              'Device not found. Make sure the other device is showing the pairing code and both devices are on the same Wi-Fi network.';
        }

        setState(() {
          _error = errorMsg;
        });

        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text(errorMsg),
            backgroundColor: Colors.orange,
            duration: const Duration(seconds: 6),
            action: SnackBarAction(
              label: 'Settings',
              onPressed: () => context.push('/settings'),
            ),
          ),
        );
      }
    }
  }
}

class _ShowCodeTab extends StatelessWidget {
  final bool isLoading;
  final String? pairingCode;
  final String? qrData;
  final String? error;
  final VoidCallback onRegenerate;

  const _ShowCodeTab({
    required this.isLoading,
    required this.pairingCode,
    required this.qrData,
    required this.error,
    required this.onRegenerate,
  });

  @override
  Widget build(BuildContext context) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (error != null) {
      return Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Icon(
                Icons.error_outline,
                size: 64,
                color: Theme.of(context).colorScheme.error,
              ),
              const SizedBox(height: 16),
              Text(
                error!,
                style: Theme.of(context).textTheme.bodyLarge,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 24),
              ElevatedButton.icon(
                onPressed: onRegenerate,
                icon: const Icon(Icons.refresh),
                label: const Text('Try Again'),
              ),
            ],
          ),
        ),
      );
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        children: [
          // Instructions
          Text(
            'Scan this QR code from another device running Toss',
            style: Theme.of(context).textTheme.bodyLarge,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 32),

          // QR Code
          Container(
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: Colors.white,
              borderRadius: BorderRadius.circular(16),
            ),
            child: QrImageView(
              data: qrData ?? '',
              version: QrVersions.auto,
              size: 200,
            ),
          ),
          const SizedBox(height: 24),

          // Or divider
          Row(
            children: [
              const Expanded(child: Divider()),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Text(
                  'or enter code manually',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              const Expanded(child: Divider()),
            ],
          ),
          const SizedBox(height: 24),

          // Pairing code
          Text(
            pairingCode ?? '',
            style: Theme.of(context).textTheme.displaySmall?.copyWith(
                  fontWeight: FontWeight.bold,
                  letterSpacing: 8,
                ),
          ),
          const SizedBox(height: 32),

          // Regenerate button
          TextButton.icon(
            onPressed: onRegenerate,
            icon: const Icon(Icons.refresh),
            label: const Text('Generate new code'),
          ),
        ],
      ),
    );
  }
}

class _ScanCodeTab extends StatefulWidget {
  final Function(String) onScanned;
  final Function(String) onManualCode;
  final bool isPairing;

  const _ScanCodeTab({
    required this.onScanned,
    required this.onManualCode,
    required this.isPairing,
  });

  @override
  State<_ScanCodeTab> createState() => _ScanCodeTabState();
}

class _ScanCodeTabState extends State<_ScanCodeTab> {
  final _codeController = TextEditingController();
  bool _isScanning = false;
  MobileScannerController? _scannerController;
  bool _hasPermission = false;

  @override
  void initState() {
    super.initState();
    _requestCameraPermission();
    _codeController.addListener(_onCodeChanged);
  }

  void _onCodeChanged() {
    setState(() {});
  }

  @override
  void dispose() {
    _codeController.removeListener(_onCodeChanged);
    _codeController.dispose();
    _scannerController?.dispose();
    super.dispose();
  }

  Future<void> _requestCameraPermission() async {
    final status = await Permission.camera.request();
    if (mounted) {
      setState(() {
        _hasPermission = status.isGranted;
        if (_hasPermission) {
          _scannerController = MobileScannerController(
            detectionSpeed: DetectionSpeed.noDuplicates,
            facing: CameraFacing.back,
          );
        }
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return SingleChildScrollView(
      padding: const EdgeInsets.all(24),
      child: Column(
        children: [
          // Camera scanner
          if (!_hasPermission)
            Container(
              height: 250,
              width: double.infinity,
              decoration: BoxDecoration(
                color: Theme.of(context).colorScheme.surfaceContainerHighest,
                borderRadius: BorderRadius.circular(16),
              ),
              child: Column(
                mainAxisAlignment: MainAxisAlignment.center,
                children: [
                  Icon(
                    Icons.camera_alt_outlined,
                    size: 64,
                    color: Theme.of(context).colorScheme.outline,
                  ),
                  const SizedBox(height: 16),
                  Text(
                    'Camera permission required',
                    style: Theme.of(context).textTheme.bodyLarge?.copyWith(
                          color: Theme.of(context).colorScheme.outline,
                        ),
                  ),
                  const SizedBox(height: 16),
                  ElevatedButton(
                    onPressed: _requestCameraPermission,
                    child: const Text('Grant Permission'),
                  ),
                ],
              ),
            )
          else
            ClipRRect(
              borderRadius: BorderRadius.circular(16),
              child: SizedBox(
                height: 250,
                width: double.infinity,
                child: Stack(
                  children: [
                    MobileScanner(
                      controller: _scannerController,
                      onDetect: (capture) {
                        final List<Barcode> barcodes = capture.barcodes;
                        if (barcodes.isNotEmpty && _isScanning) {
                          final barcode = barcodes.first;
                          if (barcode.rawValue != null) {
                            // Stop scanning
                            setState(() => _isScanning = false);
                            // Handle scanned QR code
                            widget.onScanned(barcode.rawValue!);
                          }
                        }
                      },
                    ),
                    if (!_isScanning)
                      Container(
                        color: Colors.black54,
                        child: Center(
                          child: ElevatedButton.icon(
                            onPressed: () {
                              setState(() => _isScanning = true);
                            },
                            icon: const Icon(Icons.play_arrow),
                            label: const Text('Start Scanning'),
                          ),
                        ),
                      ),
                  ],
                ),
              ),
            ),
          const SizedBox(height: 24),

          // Or divider
          Row(
            children: [
              const Expanded(child: Divider()),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Text(
                  'or enter code manually',
                  style: Theme.of(context).textTheme.bodySmall,
                ),
              ),
              const Expanded(child: Divider()),
            ],
          ),
          const SizedBox(height: 24),

          // Manual code entry
          TextField(
            controller: _codeController,
            keyboardType: TextInputType.number,
            maxLength: 6,
            textAlign: TextAlign.center,
            enabled: !widget.isPairing,
            style: Theme.of(context).textTheme.headlineSmall?.copyWith(
                  letterSpacing: 8,
                ),
            decoration: const InputDecoration(
              hintText: '000000',
              counterText: '',
            ),
            inputFormatters: [
              FilteringTextInputFormatter.digitsOnly,
            ],
          ),
          const SizedBox(height: 8),
          Text(
            'Enter the 6-digit code shown on the other device',
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: Theme.of(context).colorScheme.outline,
                ),
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 16),

          SizedBox(
            width: double.infinity,
            child: ElevatedButton(
              onPressed: widget.isPairing || _codeController.text.length != 6
                  ? null
                  : () => widget.onManualCode(_codeController.text),
              child: widget.isPairing
                  ? Row(
                      mainAxisSize: MainAxisSize.min,
                      children: const [
                        SizedBox(
                          width: 16,
                          height: 16,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                        SizedBox(width: 12),
                        Text('Searching...'),
                      ],
                    )
                  : const Text('Connect'),
            ),
          ),
        ],
      ),
    );
  }
}
