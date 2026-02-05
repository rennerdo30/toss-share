import 'package:flutter/material.dart';

import '../constants/layout_constants.dart';

/// A consistent empty state widget for use across the app
class EmptyState extends StatelessWidget {
  final IconData icon;
  final String title;
  final String? subtitle;
  final Widget? action;

  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    this.subtitle,
    this.action,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(LayoutConstants.largePadding),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(
              icon,
              size: LayoutConstants.emptyStateIconSize,
              color: theme.colorScheme.outline,
            ),
            const SizedBox(height: LayoutConstants.defaultPadding),
            Text(
              title,
              style: theme.textTheme.titleMedium,
              textAlign: TextAlign.center,
            ),
            if (subtitle != null) ...[
              const SizedBox(height: LayoutConstants.smallPadding),
              Text(
                subtitle!,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.outline,
                ),
                textAlign: TextAlign.center,
              ),
            ],
            if (action != null) ...[
              const SizedBox(height: LayoutConstants.defaultPadding),
              action!,
            ],
          ],
        ),
      ),
    );
  }
}
