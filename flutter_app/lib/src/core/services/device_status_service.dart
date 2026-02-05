import 'dart:async';
import 'dart:convert';
import 'package:http/http.dart' as http;

import 'logging_service.dart';

/// Response from the device status API endpoint
class DeviceStatusResponse {
  final String deviceId;
  final bool isOnline;
  final DateTime? lastSeen;

  const DeviceStatusResponse({
    required this.deviceId,
    required this.isOnline,
    this.lastSeen,
  });

  factory DeviceStatusResponse.fromJson(Map<String, dynamic> json) {
    return DeviceStatusResponse(
      deviceId: json['device_id'] as String,
      isOnline: json['is_online'] as bool,
      lastSeen: json['last_seen'] != null
          ? DateTime.fromMillisecondsSinceEpoch(
              (json['last_seen'] as int) * 1000)
          : null,
    );
  }
}

/// Service for fetching device status from the relay server API
class DeviceStatusService {
  DeviceStatusService._();

  static final DeviceStatusService _instance = DeviceStatusService._();
  static DeviceStatusService get instance => _instance;

  /// HTTP client for making requests
  final http.Client _client = http.Client();

  /// Timeout for API requests
  static const Duration _requestTimeout = Duration(seconds: 10);

  /// Get the status of a specific device from the relay server
  ///
  /// Returns null if the request fails or the relay URL is not configured
  Future<DeviceStatusResponse?> getDeviceStatus(
    String deviceId, {
    required String? relayUrl,
    String? authToken,
  }) async {
    if (relayUrl == null || relayUrl.isEmpty) {
      LoggingService.debug(
          'DeviceStatusService: No relay URL configured, skipping status check');
      return null;
    }

    try {
      final uri = Uri.parse('$relayUrl/api/v1/devices/$deviceId/status');

      final headers = <String, String>{
        'Content-Type': 'application/json',
      };

      if (authToken != null && authToken.isNotEmpty) {
        headers['Authorization'] = 'Bearer $authToken';
      }

      final response = await _client.get(uri, headers: headers).timeout(
            _requestTimeout,
            onTimeout: () =>
                throw TimeoutException('Device status request timed out'),
          );

      if (response.statusCode == 200) {
        final json = jsonDecode(response.body) as Map<String, dynamic>;
        return DeviceStatusResponse.fromJson(json);
      } else if (response.statusCode == 404) {
        LoggingService.debug(
            'DeviceStatusService: Device $deviceId not found on relay server');
        return null;
      } else {
        LoggingService.warn(
            'DeviceStatusService: Failed to get device status: ${response.statusCode}');
        return null;
      }
    } on TimeoutException {
      LoggingService.warn(
          'DeviceStatusService: Timeout getting status for device $deviceId');
      return null;
    } catch (e) {
      LoggingService.warn('DeviceStatusService: Error getting device status: $e');
      return null;
    }
  }

  /// Get status for multiple devices in parallel
  ///
  /// Returns a map of device ID to status response (null values for failed requests)
  Future<Map<String, DeviceStatusResponse?>> getMultipleDeviceStatuses(
    List<String> deviceIds, {
    required String? relayUrl,
    String? authToken,
  }) async {
    if (relayUrl == null || relayUrl.isEmpty) {
      return {};
    }

    final futures = deviceIds.map((id) async {
      final status = await getDeviceStatus(
        id,
        relayUrl: relayUrl,
        authToken: authToken,
      );
      return MapEntry(id, status);
    });

    final results = await Future.wait(futures);
    return Map.fromEntries(results);
  }

  /// Dispose of the HTTP client
  void dispose() {
    _client.close();
  }
}
