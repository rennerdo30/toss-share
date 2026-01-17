package dev.renner.toss

import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

class MainActivity : FlutterActivity() {
    private lateinit var keystoreBridge: KeystoreBridge

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)

        // Set up Android Keystore bridge for secure storage
        keystoreBridge = KeystoreBridge(this)
        keystoreBridge.setupMethodChannel(flutterEngine)
    }
}
