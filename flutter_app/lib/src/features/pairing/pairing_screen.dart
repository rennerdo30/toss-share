import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:go_router/go_router.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:permission_handler/permission_handler.dart';

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
    setState(() => _isLoading = true);

    final pairingInfo = await TossService.startPairing();
    if (mounted) {
      setState(() {
        _pairingCode = pairingInfo.code;
        _qrData = pairingInfo.qrData;
        _isLoading = false;
      });
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
            onRegenerate: _startPairing,
          ),

          // Tab 2: Scan QR code
          _ScanCodeTab(
            onScanned: _handleQrScanned,
          ),
        ],
      ),
    );
  }

  void _handleQrScanned(String data) async {
    final device = await TossService.completePairingQR(data);
    if (mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('Device "${device.name}" paired successfully!')),
      );
      context.pop();
    }
  }
}

class _ShowCodeTab extends StatelessWidget {
  final bool isLoading;
  final String? pairingCode;
  final String? qrData;
  final VoidCallback onRegenerate;

  const _ShowCodeTab({
    required this.isLoading,
    required this.pairingCode,
    required this.qrData,
    required this.onRegenerate,
  });

  @override
  Widget build(BuildContext context) {
    if (isLoading) {
      return const Center(child: CircularProgressIndicator());
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

  const _ScanCodeTab({required this.onScanned});

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
  }

  @override
  void dispose() {
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
            style: Theme.of(context).textTheme.headlineSmall?.copyWith(
              letterSpacing: 8,
            ),
            decoration: const InputDecoration(
              hintText: '000000',
              counterText: '',
            ),
          ),
          const SizedBox(height: 16),

          SizedBox(
            width: double.infinity,
            child: ElevatedButton(
              onPressed: () {
                if (_codeController.text.length == 6) {
                  widget.onScanned(_codeController.text);
                }
              },
              child: const Text('Connect'),
            ),
          ),
        ],
      ),
    );
  }
}
