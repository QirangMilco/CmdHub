import 'package:flutter/material.dart';
import '../src/rust/models.dart';

class StatusBadge extends StatelessWidget {
  final InstanceStatus status;

  const StatusBadge({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    final (color, text) = switch (status) {
      InstanceStatus_Running() => (Colors.green, '运行中'),
      InstanceStatus_Exited(code: final c) => (Colors.grey, '退出 $c'),
      InstanceStatus_Error(message: final m) => (Colors.red, m),
    };

    return Row(
      mainAxisSize: MainAxisSize.min,
      children: [
        Container(
          width: 8,
          height: 8,
          decoration: BoxDecoration(
            color: color,
            shape: BoxShape.circle,
          ),
        ),
        const SizedBox(width: 6),
        Text(text, style: TextStyle(color: color, fontSize: 12)),
      ],
    );
  }
}
