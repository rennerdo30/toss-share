//! Platform-specific secure storage service
//!
//! Provides secure storage for sensitive data using platform-native mechanisms:
//! - Android: Android Keystore via EncryptedSharedPreferences
//! - iOS: Keychain (handled by Rust via security-framework)
//! - macOS: Keychain (handled by Rust via security-framework)
//! - Windows: Credential Manager (handled by Rust)
//! - Linux: Secret Service (handled by Rust)

import 'dart:convert';
import 'dart:io';
import 'package:flutter/services.dart';
import 'package:toss/src/core/services/logging_service.dart';

/// Platform channel for Android Keystore
const _keystoreChannel = MethodChannel('toss.app/keystore');

/// Service for platform-specific secure storage
class SecureStorageService {
  static final SecureStorageService _instance = SecureStorageService._internal();
  factory SecureStorageService() => _instance;
  SecureStorageService._internal();

  /// Check if platform-specific secure storage is available
  Future<bool> isAvailable() async {
    if (!Platform.isAndroid) {
      // On non-Android platforms, secure storage is handled by Rust
      return true;
    }

    try {
      final bool? available =
          await _keystoreChannel.invokeMethod<bool>('isAvailable');
      return available ?? false;
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to check keystore availability: $e');
      return false;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return false;
    }
  }

  /// Store data securely (Android only, other platforms use Rust)
  Future<bool> store(String key, Uint8List value) async {
    if (!Platform.isAndroid) {
      LoggingService.debug('SecureStorageService.store: Not Android, skipping');
      return false;
    }

    try {
      final base64Value = base64Encode(value);
      final bool? success = await _keystoreChannel.invokeMethod<bool>(
        'store',
        {'key': key, 'value': base64Value},
      );
      return success ?? false;
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to store in keystore: $e');
      return false;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return false;
    }
  }

  /// Retrieve data from secure storage (Android only, other platforms use Rust)
  Future<Uint8List?> retrieve(String key) async {
    if (!Platform.isAndroid) {
      LoggingService.debug('SecureStorageService.retrieve: Not Android, skipping');
      return null;
    }

    try {
      final String? base64Value = await _keystoreChannel.invokeMethod<String>(
        'retrieve',
        {'key': key},
      );

      if (base64Value == null) {
        return null;
      }

      return base64Decode(base64Value);
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to retrieve from keystore: $e');
      return null;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return null;
    }
  }

  /// Delete data from secure storage (Android only, other platforms use Rust)
  Future<bool> delete(String key) async {
    if (!Platform.isAndroid) {
      LoggingService.debug('SecureStorageService.delete: Not Android, skipping');
      return false;
    }

    try {
      final bool? success = await _keystoreChannel.invokeMethod<bool>(
        'delete',
        {'key': key},
      );
      return success ?? false;
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to delete from keystore: $e');
      return false;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return false;
    }
  }

  /// Generate a new encryption key in Android Keystore
  Future<bool> generateKey(String alias) async {
    if (!Platform.isAndroid) {
      return false;
    }

    try {
      final bool? success = await _keystoreChannel.invokeMethod<bool>(
        'generateKey',
        {'alias': alias},
      );
      return success ?? false;
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to generate key: $e');
      return false;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return false;
    }
  }

  /// Encrypt data using a key from Android Keystore
  Future<Uint8List?> encrypt(String alias, Uint8List data) async {
    if (!Platform.isAndroid) {
      return null;
    }

    try {
      final base64Data = base64Encode(data);
      final String? base64Result = await _keystoreChannel.invokeMethod<String>(
        'encrypt',
        {'alias': alias, 'data': base64Data},
      );

      if (base64Result == null) {
        return null;
      }

      return base64Decode(base64Result);
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to encrypt with keystore: $e');
      return null;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return null;
    }
  }

  /// Decrypt data using a key from Android Keystore
  Future<Uint8List?> decrypt(String alias, Uint8List data) async {
    if (!Platform.isAndroid) {
      return null;
    }

    try {
      final base64Data = base64Encode(data);
      final String? base64Result = await _keystoreChannel.invokeMethod<String>(
        'decrypt',
        {'alias': alias, 'data': base64Data},
      );

      if (base64Result == null) {
        return null;
      }

      return base64Decode(base64Result);
    } on PlatformException catch (e) {
      LoggingService.warn('Failed to decrypt with keystore: $e');
      return null;
    } on MissingPluginException {
      LoggingService.debug('Keystore channel not available');
      return null;
    }
  }
}
