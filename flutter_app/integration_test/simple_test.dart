import 'package:flutter_test/flutter_test.dart';
import 'package:toss/src/rust/frb_generated.dart';
import 'package:integration_test/integration_test.dart';

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  setUpAll(() async => await RustLib.init());

  testWidgets('Flutter Rust Bridge is initialized', (WidgetTester tester) async {
    // Verify that RustLib was initialized successfully
    // The setUpAll would have thrown if initialization failed
    expect(true, isTrue);
  });
}
