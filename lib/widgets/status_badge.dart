import 'package:flutter/material.dart';
import '../src/rust/models.dart';

class StatusBadge extends StatelessWidget {
  final InstanceStatus status;

  const StatusBadge({super.key, required this.status});

  @override
  Widget build(BuildContext context) {
    final (bgColor, textColor, text) = switch (status) {
      InstanceStatus_Running() => (
          const Color(0xFFDCFCE7),
          const Color(0xFF166534),
          '运行中',
        ),
      InstanceStatus_Exited(code: final c) => (
          const Color(0xFFF3F4F6),
          const Color(0xFF6B7280),
          '退出 $c',
        ),
      InstanceStatus_Killed() => (
          const Color(0xFFFEF3C7),
          const Color(0xFF92400E),
          '已停止',
        ),
      InstanceStatus_Error(message: final m) => (
          const Color(0xFFFEE2E2),
          const Color(0xFF991B1B),
          m.length > 12 ? '${m.substring(0, 12)}...' : m,
        ),
    };

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 2),
      decoration: BoxDecoration(
        color: bgColor,
        borderRadius: BorderRadius.circular(4),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            width: 6,
            height: 6,
            decoration: BoxDecoration(
              color: textColor,
              shape: BoxShape.circle,
            ),
          ),
          const SizedBox(width: 6),
          Text(
            text,
            style: TextStyle(
              color: textColor,
              fontSize: 11,
              fontWeight: FontWeight.w500,
              height: 1.2,
            ),
          ),
        ],
      ),
    );
  }
}
