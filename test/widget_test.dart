import 'package:flutter_test/flutter_test.dart';
import 'package:cmdhub/main.dart';

void main() {
  testWidgets('App launches', (WidgetTester tester) async {
    await tester.pumpWidget(const CmdHubApp());
    expect(find.text('CmdHub'), findsOneWidget);
  });
}
